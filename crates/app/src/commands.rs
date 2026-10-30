//! The commands the UI calls with `invoke`. Each answers with JSON on a thread of its own; none
//! touches the UI. Edits that GitHub should hear of go to the cache's outbox and wake the sync.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use np_core::github::{Client, Http};
use np_core::workspace::{self, Uri, create_note, ensure_id, read_note, write_body};
use np_core::{Error, fields, views};
use serde_json::{Map, Value, json};
use svelte_gpui::{CommandError, Commands};

use crate::canvas::{self, rel};
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
        Ok(view)
    });
    command!("project_view", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let ws = open.ws().clone();
        let project = canvas::find_project(&ws, arg(&args, "project")?).map_err(fail)?;
        open.wrote(&canvas::project_canvas_path(&project));
        canvas::project_view(&ws, &open.cache(), arg(&args, "project")?).map_err(fail)
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
    command!("issue", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        let node = arg(&args, "node")?;
        if !node.starts_with("local:") {
            open.sync.comments(node);
        }
        views::issue(open.cache().conn(), node).map(|v| v.unwrap_or(Value::Null)).map_err(fail)
    });
    command!("issue_search", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        Ok(json!(views::search(open.cache().conn(), arg(&args, "query")?, 30).map_err(fail)?))
    });
    command!("set_field", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        open.cache().set_field(arg(&args, "node")?, arg(&args, "field")?, args["value"].clone()).map_err(fail)?;
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
    command!("comment", |app: &App, args: Value| {
        let open = app.get().map_err(fail)?;
        open.cache().add_comment(arg(&args, "node")?, arg(&args, "body")?).map_err(fail)?;
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
            for path in canvas::rename_in_canvases(&ws, &rel(&ws, &from), &rel(&ws, &to)).map_err(fail)? {
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
    command!("url_link", |_app: &App, args: Value| {
        let url = arg(&args, "url")?.trim().to_string();
        let url = if url.contains("://") { url } else { format!("https://{url}") };
        Ok(json!({"id": node_id(), "type": "link", "url": url, "x": 0, "y": 0, "width": 300, "height": 84}))
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

fn unique(folder: &Path, stem: &str, ext: &str) -> PathBuf {
    let mut path = folder.join(format!("{stem}.{ext}"));
    let mut n = 2;
    while path.exists() {
        path = folder.join(format!("{stem} {n}.{ext}"));
        n += 1;
    }
    path
}

fn note_json(ws: &workspace::Workspace, path: &Path) -> Answer {
    let note = read_note(path).map_err(fail)?;
    let uri = note.id.as_ref().map(|id| format!("note:{id}")).or_else(|| ws.uri_for_path(path).map(|u| u.to_string()));
    Ok(json!({"path": rel(ws, path), "uri": uri, "title": note.title, "body": note.body}))
}

/// Moves an issue to a board column: sets its Status (closing it for the done column, reopening
/// it when it leaves), or, on an Open/Closed board, its state.
fn move_card(open: &Open, node: &str, column: &str) -> np_core::Result<()> {
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
    Uri::parse(from)?;
    Uri::parse(to)?;
    let ws = open.ws().clone();
    open.wrote(&ws.relations_path());
    ws.add_relation(kind, from, to, None, None, "user")?;
    Ok(())
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
        Some("file") => {
            let ws = open.ws().clone();
            open.wrote(&ws.relations_path());
            ws.remove_relation(arrow["id"].as_str().unwrap_or_default())?;
        }
        _ => return Err(Error::Other("this arrow is computed and can't be removed".into())),
    }
    Ok(())
}

/// Canvas nodes for files dropped on the window. Files are linked where they are: inside the
/// workspace by path, elsewhere through a linked root for their folder (added when needed).
fn link_files(open: &Open, paths: &[PathBuf]) -> np_core::Result<Value> {
    let mut ws = open.ws();
    let mut nodes = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        let path = expand(path);
        let offset = i as f64 * 30.0;
        let image = workspace::mime_for(&path).starts_with("image/");
        let (w, h) = if image {
            (360.0, 260.0)
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
