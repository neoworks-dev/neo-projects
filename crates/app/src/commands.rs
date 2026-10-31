//! The commands the UI calls with `invoke`. Each answers with JSON on a thread of its own; none
//! touches the UI. Edits that GitHub should hear of go to the cache's outbox and wake the sync.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use np_core::github::{Client, Http};
use np_core::workspace::{
    self, Stroke, Uri, attach_file, copy_file_as, create_note, ensure_id, issues, read_note, read_props, rename_prop, set_prop, write_body,
};
use rusqlite::OptionalExtension;
use np_core::{Error, fields, media, views};
use serde_json::{Map, Value, json};
use svelte_gpui::{CommandError, Commands};

use crate::canvas::{self, rel};
use crate::folders;
use crate::state::{App, Open, expand};

type Answer = Result<Value, CommandError>;

fn fail(e: Error) -> CommandError {
    CommandError::from(e.to_string())
}

fn arg<'a>(args: &'a Value, name: &str) -> Result<&'a str, CommandError> {
    args[name].as_str().ok_or_else(|| CommandError::from(format!("missing argument {name}")))
}

/// Registers every command; each gets the app state.
pub fn all(commands: Commands, app: Arc<App>) -> Commands {
    let mut commands = commands;
    macro_rules! command {
        ($name:literal, $f:expr) => {{
            let app = app.clone();
            let f = $f;
            commands = commands.command($name, move |args: Value| -> Answer { f(&app, args) });
        }};
    }

    command!("workspace", |app: &App, _| Ok(json!({"path": app.path()})));
    command!("workspace_open", |app: &App, args: Value| {
        let open = app.open(Path::new(arg(&args, "path")?)).map_err(fail)?;
        Ok(json!({"path": open.root}))
    });
    command!("overview", |app: &App, _| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let (path, view) = canvas::overview(&ws, &open.cache()).map_err(fail)?;
        open.wrote(&path);
        open.previews.want_view(&view);
        Ok(view)
    });
    command!("project_view", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let project = canvas::find_project(&ws, arg(&args, "project")?).map_err(fail)?;
        let inside = args["inside"].as_str();
        let unfolded: Vec<String> =
            args["unfolded"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
        open.wrote(&canvas::view_canvas_path(&ws, &project, inside).map_err(fail)?);
        let view = canvas::project_view(&ws, &open.cache(), arg(&args, "project")?, inside, &unfolded).map_err(fail)?;
        open.previews.want_view(&view);
        Ok(view)
    });
    // A folder in `dir` (workspace-relative), with its canvas. Answers with its node.
    command!("folder_create", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let dir = unique(&ws.root().join(arg(&args, "dir")?), &sanitize(arg(&args, "name")?), "");
        std::fs::create_dir(&dir).map_err(|e| e.to_string())?;
        let path = workspace::folder_canvas_path(&dir);
        open.wrote(&path);
        workspace::write_canvas(&path, &workspace::Canvas::default()).map_err(fail)?;
        Ok(json!({"id": node_id(), "type": "file", "file": rel(&ws, &path), "x": 0, "y": 0, "width": 220, "height": 170}))
    });
    // The node `node` of the canvas `from` moved onto the canvas `into` (both workspace-relative):
    // into a folder, or out of one. `at` is where it goes, `[x, y]`, or where there is room.
    command!("folder_put", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let root = open.ws().root().to_path_buf();
        let from = root.join(arg(&args, "from")?);
        let into = root.join(arg(&args, "into")?);
        let at = match (args["at"][0].as_f64(), args["at"][1].as_f64()) {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        };
        folders::put(&open, &from, arg(&args, "node")?, &into, at).map_err(fail)?;
        app.emitter.emit("changed", json!({"reason": "folder"}));
        Ok(Value::Null)
    });
    // Answers with the folder's new directory, workspace-relative.
    command!("folder_rename", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let dir = ws.root().join(arg(&args, "dir")?);
        let to = folders::rename(&open, &dir, &sanitize(arg(&args, "name")?)).map_err(fail)?;
        app.emitter.emit("changed", json!({"reason": "rename"}));
        Ok(json!(rel(&ws, &to)))
    });
    command!("project_create", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let repo = args["repo"].as_str().filter(|r| !r.is_empty());
        let project = ws.create_project(arg(&args, "name")?, repo).map_err(fail)?;
        workspace::index_path(&ws, &open.cache(), &project.note).map_err(fail)?;
        open.sync.now();
        canvas::project_json(&ws, &open.cache(), &project).map_err(fail)
    });
    command!("github_repos", |app: &App, _| github_repos(app).map_err(fail));
    command!("canvas_save", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let path = ws.root().join(arg(&args, "path")?);
        let nodes = args["nodes"].as_array().cloned().unwrap_or_default();
        open.wrote(&path);
        canvas::save(&ws, &open.cache(), &path, &nodes).map_err(fail)?;
        Ok(Value::Null)
    });
    command!("canvas_strokes", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let path = open.ws().root().join(arg(&args, "path")?);
        let strokes: Vec<Stroke> = serde_json::from_value(args["strokes"].clone()).map_err(|e| e.to_string())?;
        open.wrote(&path);
        canvas::save_strokes(&path, &strokes).map_err(fail)?;
        Ok(Value::Null)
    });
    command!("issue", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let node = arg(&args, "node")?;
        if issues::is_local(node) {
            return open.ws().issue_detail(node).map_err(fail);
        }
        if !node.starts_with("local:") {
            open.sync.comments(node);
        }
        views::issue(open.cache().conn(), node).map(|v| v.unwrap_or(Value::Null)).map_err(fail)
    });
    command!("issue_search", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let query = arg(&args, "query")?;
        let mut found = open.ws().search_issues(query, 30);
        found.extend(views::search(open.cache().conn(), query, 30).map_err(fail)?);
        Ok(json!(found))
    });
    command!("set_field", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let node = arg(&args, "node")?;
        if issues::is_local(node) {
            let ws = open.ws().clone();
            open.wrote(&ws.issue_path(node).map_err(fail)?);
            ws.set_issue_field(node, arg(&args, "field")?, &args["value"]).map_err(fail)?;
            return edited(app, &open);
        }
        open.cache().set_field(node, arg(&args, "field")?, args["value"].clone()).map_err(fail)?;
        edited(app, &open)
    });
    command!("move_card", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        move_card(&open, arg(&args, "node")?, arg(&args, "column")?).map_err(fail)?;
        edited(app, &open)
    });
    command!("create_issue", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let node = open.cache().create_issue(arg(&args, "repo")?, arg(&args, "title")?, "").map_err(fail)?;
        if let Some(column) = args["column"].as_str() {
            move_card(&open, &node, column).map_err(fail)?;
        }
        edited(app, &open)?;
        Ok(json!(node))
    });
    command!("local_issue_create", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let project = canvas::find_project(&ws, arg(&args, "folder")?).map_err(fail)?;
        let node = ws.create_issue(&project, arg(&args, "title")?, args["column"].as_str()).map_err(fail)?;
        let path = ws.issue_path(&node).map_err(fail)?;
        open.wrote(&path);
        workspace::index_path(&ws, &open.cache(), &path).map_err(fail)?;
        edited(app, &open)?;
        Ok(json!(node))
    });
    command!("comment", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let node = arg(&args, "node")?;
        if issues::is_local(node) {
            let ws = open.ws().clone();
            open.wrote(&ws.issue_path(node).map_err(fail)?);
            let author = ws.config().github.login.clone().or_else(|| std::env::var("USER").ok()).unwrap_or("you".into());
            ws.add_issue_comment(node, &author, arg(&args, "body")?).map_err(fail)?;
            return edited(app, &open);
        }
        open.cache().add_comment(node, arg(&args, "body")?).map_err(fail)?;
        edited(app, &open)
    });
    command!("resolve", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let keep = args["keepLocal"].as_bool().unwrap_or(false);
        open.cache().resolve(arg(&args, "node")?, arg(&args, "field")?, keep).map_err(fail)?;
        edited(app, &open)
    });
    command!("relate", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        relate(&open, arg(&args, "from")?, arg(&args, "to")?, arg(&args, "type")?).map_err(fail)?;
        edited(app, &open)
    });
    command!("unrelate", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        unrelate(&open, &args["arrow"]).map_err(fail)?;
        edited(app, &open)
    });
    command!("note_create", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let folder = ws.root().join(arg(&args, "folder")?);
        let mut props = Map::new();
        if let (Some(key), Some(value)) = (args["prop"].as_str(), args["value"].as_str()) {
            props.insert(key.to_string(), json!(value));
        }
        let path = create_note(&folder, arg(&args, "title")?, &props).map_err(fail)?;
        ensure_id(&path).map_err(fail)?;
        open.wrote(&path);
        workspace::index_path(&ws, &open.cache(), &path).map_err(fail)?;
        note_json(&ws, &path)
    });
    command!("note_read", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        note_json(&ws, &ws.root().join(arg(&args, "path")?))
    });
    command!("note_write", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let path = ws.root().join(arg(&args, "path")?);
        open.wrote(&path);
        write_body(&path, arg(&args, "body")?).map_err(fail)?;
        workspace::index_path(&ws, &open.cache(), &path).map_err(fail)?;
        Ok(Value::Null)
    });
    // A file dropped into a note, copied into the note's folder. Answers with the file's name,
    // which is how the note links it.
    command!("note_attach", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let note = ws.root().join(arg(&args, "path")?);
        let Some(folder) = note.parent() else {
            return Err("The note has no folder".into());
        };
        let attached = attach_file(folder, Path::new(arg(&args, "file")?)).map_err(fail)?;
        open.wrote(&attached);
        Ok(json!(attached.file_name().unwrap_or_default().to_string_lossy()))
    });
    // A property's value: null removes the key. The id is what links and canvases point at, so
    // it can't be changed here.
    command!("note_prop", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let path = ws.root().join(arg(&args, "path")?);
        let key = arg(&args, "key")?;
        if key == "id" {
            return Err("The id can't be edited".into());
        }
        let value = Some(&args["value"]).filter(|value| !value.is_null());
        open.wrote(&path);
        set_prop(&path, key, value).map_err(fail)?;
        workspace::index_path(&ws, &open.cache(), &path).map_err(fail)?;
        note_json(&ws, &path)
    });
    command!("note_prop_rename", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let path = ws.root().join(arg(&args, "path")?);
        let (from, to) = (arg(&args, "from")?, arg(&args, "to")?);
        if from == "id" || to == "id" {
            return Err("The id can't be edited".into());
        }
        open.wrote(&path);
        rename_prop(&path, from, to).map_err(fail)?;
        workspace::index_path(&ws, &open.cache(), &path).map_err(fail)?;
        note_json(&ws, &path)
    });
    command!("note_rename", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let from = ws.root().join(arg(&args, "path")?);
        let title = sanitize(arg(&args, "title")?);
        let to = from.with_file_name(format!("{title}.md"));
        if to != from {
            if to.exists() {
                return Err(format!("{} already exists", rel(&ws, &to)).into());
            }
            ensure_id(&from).map_err(fail)?;
            std::fs::rename(&from, &to).map_err(|e| e.to_string())?;
            open.wrote(&from);
            open.wrote(&to);
            workspace::rename_path(&ws, &open.cache(), &from, &to).map_err(fail)?;
            for path in workspace::folders::rename_in_canvases(&ws, &rel(&ws, &from), &rel(&ws, &to)).map_err(fail)? {
                open.wrote(&path);
            }
            app.emitter.emit("changed", json!({"reason": "rename"}));
        }
        note_json(&ws, &to)
    });
    command!("files_link", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let paths: Vec<PathBuf> =
            args["paths"].as_array().into_iter().flatten().filter_map(Value::as_str).map(PathBuf::from).collect();
        link_files(&open, &paths).map_err(fail)
    });
    // A file from outside the workspace, such as an image pasted from the clipboard, copied into
    // the folder `dir` as `name`. Answers with its node.
    command!("file_copy_in", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let dir = open.ws().root().join(arg(&args, "dir")?);
        let copied = copy_file_as(&dir, Path::new(arg(&args, "file")?), arg(&args, "name")?).map_err(fail)?;
        open.wrote(&copied);
        let nodes = link_files(&open, &[copied]).map_err(fail)?;
        Ok(nodes[0].clone())
    });
    command!("url_link", |app: &App, args: Value| {
        let url = arg(&args, "url")?.trim().to_string();
        let url = if url.contains("://") { url } else { format!("https://{url}") };
        app.get().map_err(fail)?.previews.want(&url);
        // Tall enough for the preview image above the title.
        Ok(json!({"id": node_id(), "type": "link", "url": url, "x": 0, "y": 0, "width": 320, "height": 260}))
    });
    command!("board_create", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let folder = ws.root().join(arg(&args, "folder")?);
        let title = arg(&args, "title")?;
        let path = unique(&folder, &sanitize(title), "board");
        let board = workspace::Board {
            version: 1,
            title: Some(title.to_string()),
            filter: workspace::Filter { folder: Some(rel(&ws, &folder)), has: vec!["status".into()], tags: vec![] },
            group_by: "status".into(),
            columns: ["Backlog", "Todo", "In Progress", "Done"]
                .iter()
                .map(|v| workspace::Column { value: v.to_string(), label: None })
                .collect(),
            show: vec![],
            order: Default::default(),
        };
        open.wrote(&path);
        workspace::write_board(&path, &board).map_err(fail)?;
        Ok(
            json!({"id": node_id(), "type": "file", "file": rel(&ws, &path), "x": 0, "y": 0, "width": 900, "height": 520}),
        )
    });
    command!("board_move", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let board = workspace::read_board(&ws.root().join(arg(&args, "board")?)).map_err(fail)?;
        let note = ws.root().join(arg(&args, "note")?);
        let value = args["value"].as_str().map(|v| json!(v));
        open.wrote(&note);
        workspace::set_prop(&note, &board.group_by, value.as_ref()).map_err(fail)?;
        workspace::index_path(&ws, &open.cache(), &note).map_err(fail)?;
        app.emitter.emit("changed", json!({"reason": "board"}));
        Ok(Value::Null)
    });
    command!("dir_list", |_app: &App, args: Value| {
        Ok(json!(workspace::list_dir(Path::new(arg(&args, "path")?)).map_err(fail)?))
    });
    command!("highlight", |_app: &App, args: Value| {
        Ok(json!(np_core::highlight::highlight(arg(&args, "code")?, args["lang"].as_str().unwrap_or(""))))
    });
    command!("open_external", |app: &App, args: Value| {
        let target = arg(&args, "target")?;
        let target = match app.get() {
            Ok(open) if !target.contains("://") && !target.starts_with('/') => {
                open.root.join(target).to_string_lossy().to_string()
            }
            _ => target.to_string(),
        };
        std::process::Command::new("xdg-open").arg(&target).spawn().map_err(|e| e.to_string())?;
        Ok(Value::Null)
    });
    command!("sync_now", |app: &App, _| {
        app.get().map_err(fail)?.sync.now();
        Ok(Value::Null)
    });
    command!("sync_status", |app: &App, _| Ok(json!(app.get().map_err(fail)?.status())));
    commands
}

/// After an edit: the window draws it at once, and the sync sends it soon.
fn edited(app: &App, open: &Open) -> Answer {
    open.sync.soon();
    app.emitter.emit("changed", json!({"reason": "edit"}));
    Ok(Value::Null)
}

fn node_id() -> String {
    uuid::Uuid::now_v7().simple().to_string()[16..].to_string()
}

/// A title made safe as a file name.
fn sanitize(title: &str) -> String {
    let clean: String = title
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '-' } else { c })
        .collect();
    let clean = clean.trim().trim_matches('.').to_string();
    if clean.is_empty() { "Untitled".into() } else { clean }
}

/// `<stem>.<ext>` in `folder`, numbered past the names taken. An empty `ext` names a folder.
fn unique(folder: &Path, stem: &str, ext: &str) -> PathBuf {
    let name = |stem: String| if ext.is_empty() { stem } else { format!("{stem}.{ext}") };
    let mut path = folder.join(name(stem.to_string()));
    let mut n = 2;
    while path.exists() {
        path = folder.join(name(format!("{stem} {n}")));
        n += 1;
    }
    path
}

fn note_json(ws: &workspace::Workspace, path: &Path) -> Answer {
    let note = read_note(path).map_err(fail)?;
    let uri = note.id.as_ref().map(|id| format!("note:{id}")).or_else(|| ws.uri_for_path(path).map(|u| u.to_string()));
    let props: Vec<Value> =
        read_props(path).map_err(fail)?.into_iter().map(|(key, value)| json!({"key": key, "value": value})).collect();
    Ok(json!({"path": rel(ws, path), "uri": uri, "title": note.title, "body": note.body, "props": props}))
}

/// Moves an issue to a board column: sets its Status (closing it for the done column, reopening
/// it when it leaves), or, on an Open/Closed board, its state.
fn move_card(open: &Open, node: &str, column: &str) -> np_core::Result<()> {
    if issues::is_local(node) {
        let ws = open.ws().clone();
        open.wrote(&ws.issue_path(node)?);
        return ws.move_issue(node, column);
    }
    let mut cache = open.cache();
    let state = fields::read(cache.conn(), node, "state")?;
    let is_open = state["state"] == "OPEN";
    match column {
        "open" | "closed" => {
            let want = if column == "open" { "OPEN" } else { "CLOSED" };
            if (want == "OPEN") != is_open {
                cache.set_field(node, "state", json!({"state": want, "reason": null}))?;
            }
        }
        _ => {
            let repo: String =
                cache.conn().query_row("SELECT repo FROM gh_issues WHERE node_id = ?1", [node], |r| r.get(0))?;
            let status = views::status_field(cache.conn(), &repo)?
                .ok_or_else(|| Error::Other("this repository has no Status field".into()))?;
            let field = format!("field:{}", status.id);
            if column == "none" {
                cache.set_field(node, &field, Value::Null)?;
                return Ok(());
            }
            let option = status
                .options
                .iter()
                .find(|o| o["id"] == column)
                .ok_or_else(|| Error::Other(format!("no column {column}")))?;
            let done = option["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case("done"));
            cache.set_field(
                node,
                &field,
                json!({"kind": "single_select", "optionId": option["id"], "name": option["name"]}),
            )?;
            if done && is_open {
                cache.set_field(node, "state", json!({"state": "CLOSED", "reason": "COMPLETED"}))?;
            } else if !done && !is_open {
                cache.set_field(node, "state", json!({"state": "OPEN", "reason": null}))?;
            }
        }
    }
    Ok(())
}

/// A new relationship. Blocks and sub-issues between two GitHub issues go to GitHub; everything
/// else is written to `relations.jsonl`.
fn relate(open: &Open, from: &str, to: &str, kind: &str) -> np_core::Result<()> {
    let issue = |uri: &str| -> np_core::Result<Option<String>> {
        if uri.starts_with("https://github.com/") { canvas::issue_by_url(&open.cache(), uri) } else { Ok(None) }
    };
    if matches!(kind, "blocks" | "parent-of")
        && let (Some(a), Some(b)) = (issue(from)?, issue(to)?)
    {
        let mut cache = open.cache();
        if kind == "blocks" {
            let mut blocked = fields::read(cache.conn(), &b, "blockedBy")?;
            if let Some(list) = blocked.as_array_mut() {
                list.push(json!(a));
            }
            cache.set_field(&b, "blockedBy", blocked)?;
        } else {
            cache.set_field(&b, "parent", json!(a))?;
        }
        return Ok(());
    }
    if matches!(kind, "blocks" | "parent-of")
        && let (Some(a), Some(b)) = (local_issue(open, from)?, local_issue(open, to)?)
    {
        let ws = open.ws().clone();
        open.wrote(&ws.issue_path(&b)?);
        if kind == "blocks" {
            let mut blocked: Vec<Value> = ws.issue_detail(&b)?["blockedByList"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|l| l["node"].clone())
                .collect();
            blocked.push(json!(a));
            return ws.set_issue_field(&b, "blockedBy", &json!(blocked));
        }
        return ws.set_issue_field(&b, "parent", &json!(a));
    }
    Uri::parse(from)?;
    Uri::parse(to)?;
    let ws = open.ws().clone();
    open.wrote(&ws.relations_path());
    ws.add_relation(kind, from, to, None, None, "user")?;
    Ok(())
}

/// The local issue a canvas URI names (a note id, as canvases name notes), as its node id.
fn local_issue(open: &Open, uri: &str) -> np_core::Result<Option<String>> {
    let Some(id) = uri.strip_prefix("note:") else { return Ok(None) };
    let path: Option<String> =
        open.cache().conn().query_row("SELECT path FROM notes WHERE id = ?1", [id], |r| r.get(0)).optional()?;
    let ws = open.ws().clone();
    let Some(path) = path.map(PathBuf::from).filter(|p| issues::read_issue(p).is_ok()) else { return Ok(None) };
    Ok(Some(format!("{}{}", issues::NODE_PREFIX, rel(&ws, &path))))
}

fn unrelate(open: &Open, arrow: &Value) -> np_core::Result<()> {
    match arrow["source"].as_str() {
        Some("github") => {
            let (Some(from), Some(to)) = (arrow["issues"][0].as_str(), arrow["issues"][1].as_str()) else {
                return Err(Error::Other("not a GitHub relationship".into()));
            };
            let mut cache = open.cache();
            if arrow["type"] == "blocks" {
                let blocked = fields::read(cache.conn(), to, "blockedBy")?;
                let rest: Vec<Value> =
                    blocked.as_array().into_iter().flatten().filter(|b| *b != from).cloned().collect();
                cache.set_field(to, "blockedBy", json!(rest))?;
            } else {
                cache.set_field(to, "parent", Value::Null)?;
            }
        }
        Some("local") => {
            let (Some(from), Some(to)) = (arrow["issues"][0].as_str(), arrow["issues"][1].as_str()) else {
                return Err(Error::Other("not a local relationship".into()));
            };
            let ws = open.ws().clone();
            open.wrote(&ws.issue_path(to)?);
            if arrow["type"] == "blocks" {
                let rest: Vec<Value> = ws.issue_detail(to)?["blockedByList"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|l| l["node"].clone())
                    .filter(|n| n != from)
                    .collect();
                ws.set_issue_field(to, "blockedBy", &json!(rest))?;
            } else {
                ws.set_issue_field(to, "parent", &Value::Null)?;
            }
        }
        Some("file") => {
            let ws = open.ws().clone();
            open.wrote(&ws.relations_path());
            ws.remove_relation(arrow["id"].as_str().unwrap_or_default())?;
        }
        _ => return Err(Error::Other("this arrow is computed and can't be removed".into())),
    }
    Ok(())
}

/// Canvas nodes for files dropped on the window or pasted. Files are linked where they are: inside
/// the workspace by path, elsewhere through a linked root for their folder (added when needed).
/// A path with no file behind it gets no node: pasted text can look like paths.
fn link_files(open: &Open, paths: &[PathBuf]) -> np_core::Result<Value> {
    let mut ws = open.ws();
    let mut nodes = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let path = expand(path);
        if !path.exists() {
            continue;
        }
        let offset = i as f64 * 30.0;
        let image = workspace::mime_for(&path).starts_with("image/");
        // An image's box has its own proportions, at most 360 on its longer side, so the node is
        // no bigger than the picture. A PDF's poster is a page, upright; a video's a frame, wide.
        let (w, h) = if image {
            match canvas::aspect(&path) {
                Some(aspect) if aspect >= 1.0 => (360.0, (360.0 / aspect).round()),
                Some(aspect) => ((360.0 * aspect).round(), 360.0),
                None => (360.0, 260.0),
            }
        } else if media::kind(&path) == Some(media::Kind::Pdf) {
            (240.0, 340.0)
        } else if media::kind(&path) == Some(media::Kind::Video) {
            (320.0, 180.0)
        } else if path.extension().is_some_and(|e| e == "md") {
            (300.0, 200.0)
        } else {
            (280.0, 76.0)
        };
        if let Ok(inside) = path.strip_prefix(ws.root()) {
            nodes.push(json!({"id": node_id(), "type": "file", "file": inside.to_string_lossy(),
                              "x": offset, "y": offset, "width": w, "height": h}));
            continue;
        }
        let uri = match ws.uri_for_path(&path) {
            Some(uri) => uri,
            None => {
                let dir = path.parent().unwrap_or(Path::new("/"));
                let base = dir.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or("files".into());
                let base: String =
                    base.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' }).collect();
                let mut name = base.clone();
                let mut n = 2;
                while ws.roots().iter().any(|(r, _)| *r == name) {
                    name = format!("{base}-{n}");
                    n += 1;
                }
                ws.set_root(&name, &dir.to_string_lossy())?;
                ws.uri_for_path(&path).ok_or_else(|| Error::Other(format!("can't link {}", path.display())))?
            }
        };
        nodes.push(json!({"id": node_id(), "type": "link", "url": format!("file://{}", path.display()),
                          "neoworks": {"uri": uri.to_string()}, "x": offset, "y": offset, "width": w, "height": h}));
    }
    Ok(json!(nodes))
}

/// Your repositories and your organizations', newest first, marked when a project has one.
fn github_repos(app: &App) -> np_core::Result<Value> {
    let open = app.get()?;
    let ws = open.ws().clone();
    let client = Client::new(Http::from_gh(ws.config().github.login.as_deref())?);
    // The viewer's own list leaves out repositories reached only through an organization, so
    // each organization's are asked for too.
    let data = client.run(
        "query { viewer {
           repositories(first: 100, orderBy: {field: UPDATED_AT, direction: DESC},
             affiliations: [OWNER, ORGANIZATION_MEMBER, COLLABORATOR]) {
             nodes { nameWithOwner description updatedAt isArchived } }
           organizations(first: 30) { nodes { repositories(first: 100, orderBy: {field: UPDATED_AT, direction: DESC}) {
             nodes { nameWithOwner description updatedAt isArchived } } } } } }",
        json!({}),
    )?;
    let (projects, _) = ws.projects();
    let taken: Vec<String> = projects.iter().flat_map(|p| p.repos.iter().map(|r| r.to_lowercase())).collect();
    let own = data["viewer"]["repositories"]["nodes"].as_array().cloned().unwrap_or_default();
    let orgs = data["viewer"]["organizations"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|o| o["repositories"]["nodes"].as_array().cloned().unwrap_or_default());
    let mut by_name: std::collections::BTreeMap<String, Value> = std::collections::BTreeMap::new();
    for mut r in own.into_iter().chain(orgs) {
        let name = r["nameWithOwner"].as_str().unwrap_or_default().to_lowercase();
        r["taken"] = json!(taken.contains(&name));
        by_name.insert(name, r);
    }
    let mut repos: Vec<Value> = by_name.into_values().collect();
    repos.sort_by(|a, b| b["updatedAt"].as_str().cmp(&a["updatedAt"].as_str()));
    Ok(json!(repos))
}
