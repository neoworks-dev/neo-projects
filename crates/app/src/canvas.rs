//! The canvases the window shows: the overview of every project, and a project's own canvas with
//! its board. Each node comes with what it stands for (an issue's card, a note's title, an
//! image's path), and the arrows are the relationships between nodes on the canvas, from GitHub
//! and from `relations.jsonl`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use np_core::media::{self, Media};
use np_core::previews::{self, Preview};
use np_core::{sheet, views};
use np_core::workspace::{
    self, Canvas, Node, Project, Stroke, Uri, Workspace, folder_canvas_path, folder_of, issues, read_canvas,
    read_note, write_canvas,
};
use np_core::{Cache, Error, Result};
use rusqlite::OptionalExtension;
use serde_json::{Map, Value, json};

/// The overview's canvas: a file in the workspace, so Obsidian shows it too.
pub const OVERVIEW: &str = "Projects.canvas";

const PROJECT_W: f64 = 320.0;
const PROJECT_H: f64 = 230.0;

pub fn rel(ws: &Workspace, path: &Path) -> String {
    ws.rel(path)
}

fn new_id() -> String {
    uuid::Uuid::now_v7().simple().to_string()[16..].to_string()
}

pub fn project_json(ws: &Workspace, cache: &Cache, p: &Project) -> Result<Value> {
    let repo = p.repos.first().cloned();
    let summary = match repo.as_deref().and_then(|r| r.split_once('/')) {
        Some((owner, name)) => match cache.repo_id(owner, name)? {
            Some(id) => Some(views::repo_summary(cache.conn(), &id)?),
            None => None,
        },
        None => Some(ws.issue_summary(p)),
    };
    let repo_id = match repo.as_deref().and_then(|r| r.split_once('/')) {
        Some((owner, name)) => cache.repo_id(owner, name)?,
        None => None,
    };
    Ok(json!({
        "id": p.id,
        "name": p.name,
        "folder": rel(ws, &p.folder),
        "note": rel(ws, &p.note),
        "repo": repo,
        "repoId": repo_id,
        "local": repo.is_none(),
        "abs": p.folder,
        "roots": p.roots.iter().filter_map(|r| ws.root_path(r).map(|path| json!({"name": r, "path": path}))).collect::<Vec<_>>(),
        "summary": summary,
    }))
}

fn load(path: &Path) -> Result<Canvas> {
    if path.exists() { read_canvas(path) } else { Ok(Canvas::default()) }
}

fn neoworks_kind(node: &Node) -> Option<&str> {
    node.neoworks().and_then(|n| n.get("kind")).and_then(Value::as_str)
}

/// The overview: every project as a node of `Projects.canvas` (added where there is room when
/// new), the blocks between their repositories as arrows.
pub fn overview(ws: &Workspace, cache: &Cache) -> Result<(PathBuf, Value)> {
    let path = ws.root().join(OVERVIEW);
    let mut canvas = load(&path)?;
    let (projects, errors) = ws.projects();
    let mut by_note: HashMap<String, &Project> = projects.iter().map(|p| (rel(ws, &p.note), p)).collect();
    let mut changed = false;
    for node in &canvas.nodes {
        if let Some(file) = &node.file {
            by_note.remove(file);
        }
    }
    let mut missing: Vec<&Project> = by_note.into_values().collect();
    missing.sort_by(|a, b| a.name.cmp(&b.name));
    for p in missing {
        let (x, y) = canvas.free_spot(PROJECT_W, PROJECT_H);
        canvas.nodes.push(Node {
            id: new_id(),
            kind: "file".into(),
            x,
            y,
            width: PROJECT_W,
            height: PROJECT_H,
            color: None,
            text: None,
            file: Some(rel(ws, &p.note)),
            url: None,
            label: None,
            extra: Map::new(),
        });
        changed = true;
    }
    if changed {
        write_canvas(&path, &canvas)?;
    }
    let notes: HashMap<String, &Project> = projects.iter().map(|p| (rel(ws, &p.note), p)).collect();
    let mut repo_node: HashMap<String, String> = HashMap::new();
    let mut nodes = Vec::new();
    for node in &canvas.nodes {
        let mut v = serde_json::to_value(node)?;
        if let Some(p) = node.file.as_ref().and_then(|f| notes.get(f)) {
            let project = project_json(ws, cache, p)?;
            if let Some(id) = project["repoId"].as_str() {
                repo_node.insert(id.to_string(), node.id.clone());
            }
            v["item"] =
                json!({"kind": "project", "uri": format!("file:workspace/{}", rel(ws, &p.note)), "project": project});
        } else {
            v["item"] = item(ws, cache, node)?;
        }
        nodes.push(v);
    }
    let mut arrows = Vec::new();
    for block in views::cross_repo_blocks(cache.conn())? {
        let (Some(from), Some(to)) = (
            block["from"].as_str().and_then(|r| repo_node.get(r)),
            block["to"].as_str().and_then(|r| repo_node.get(r)),
        ) else {
            continue;
        };
        arrows.push(json!({"id": format!("x:{from}:{to}"), "type": "blocks", "from": from, "to": to,
                           "source": "derived", "count": block["count"]}));
    }
    let projects: Vec<Value> = projects.iter().map(|p| project_json(ws, cache, p)).collect::<Result<_>>()?;
    let errors: Vec<String> = errors.into_iter().map(|e| e.message).collect();
    Ok((
        path.clone(),
        json!({"path": rel(ws, &path), "nodes": nodes, "arrows": arrows, "projects": projects, "errors": errors}),
    ))
}

pub fn find_project(ws: &Workspace, folder: &str) -> Result<Project> {
    let (projects, _) = ws.projects();
    projects
        .into_iter()
        .find(|p| rel(ws, &p.folder) == folder)
        .ok_or_else(|| Error::Other(format!("no project in {folder}")))
}

pub fn project_canvas_path(p: &Project) -> PathBuf {
    p.folder.join(format!("{}.canvas", p.name))
}

/// The canvas a project view shows: the project's own, or with `inside` (a workspace-relative
/// directory in the project) that folder's.
pub fn view_canvas_path(ws: &Workspace, project: &Project, inside: Option<&str>) -> Result<PathBuf> {
    let Some(inside) = inside else {
        return Ok(project_canvas_path(project));
    };
    let dir = ws.root().join(inside);
    if !dir.starts_with(&project.folder) || !dir.is_dir() {
        return Err(Error::Other(format!("{inside} is not a folder of {}", project.name)));
    }
    Ok(folder_canvas_path(&dir))
}

/// A project's canvas with its board, the board's issues minus those placed on the canvas or in
/// its folders, and the arrows between what is placed. Inside one of its folders, that folder's
/// canvas, which has no board.
///
/// The folders whose node ids are in `unfolded` bring their nodes along, each with the id
/// `<folder node id>/<its id>` and an `in` saying where it lives; arrows reach them too.
pub fn project_view(
    ws: &Workspace,
    cache: &Cache,
    folder: &str,
    inside: Option<&str>,
    unfolded: &[String],
) -> Result<Value> {
    let project = find_project(ws, folder)?;
    let path = view_canvas_path(ws, &project, inside)?;
    let mut canvas = load(&path)?;
    let repo = project.repos.first().cloned();
    let repo_id = match repo.as_deref().and_then(|r| r.split_once('/')) {
        Some((owner, name)) => cache.repo_id(owner, name)?,
        None => None,
    };
    let mut board = match &repo_id {
        Some(id) => views::board(cache.conn(), id)?,
        None if repo.is_none() => ws.issue_board(&project),
        None => json!({"statusField": null, "columns": []}),
    };
    if inside.is_none() && !canvas.nodes.iter().any(|n| neoworks_kind(n) == Some("board")) {
        // As wide as its columns need (Open and Closed until the first sync tells), never narrower.
        let columns = board["columns"].as_array().map_or(2, Vec::len).max(2) as f64;
        let mut extra = Map::new();
        extra.insert("neoworks".into(), json!({"kind": "board"}));
        // A GitHub board links its repository; a local one is a group, which other JSON Canvas
        // readers draw as a labelled frame.
        let (kind, url, label) = match &repo {
            Some(repo) => ("link", Some(format!("https://github.com/{repo}")), None),
            None => ("group", None, Some("Issues".to_string())),
        };
        canvas.nodes.insert(
            0,
            Node {
                id: new_id(),
                kind: kind.into(),
                x: 0.0,
                y: 0.0,
                width: 24.0 + columns * 290.0,
                height: 640.0,
                color: None,
                text: None,
                file: None,
                url,
                label,
                extra,
            },
        );
        write_canvas(&path, &canvas)?;
    }
    let mut nodes = Vec::new();
    let mut inner = Vec::new();
    let mut with_inner = canvas.clone();
    for node in &canvas.nodes {
        let mut v = serde_json::to_value(node)?;
        v["item"] = item(ws, cache, node)?;
        nodes.push(v);
        let Some(inner_canvas) = node.file.as_ref().map(|f| ws.root().join(f)).filter(|p| folder_of(p).is_some()) else {
            continue;
        };
        for child in load(&inner_canvas)?.nodes {
            let mut v = serde_json::to_value(&child)?;
            v["item"] = item(ws, cache, &child)?;
            if !unfolded.contains(&node.id) {
                // Only for the board: an issue in a folder is placed, folded or not.
                inner.push(v);
                continue;
            }
            let id = format!("{}/{}", node.id, child.id);
            v["in"] = json!({"folder": node.id, "canvas": rel(ws, &inner_canvas), "id": child.id});
            v["id"] = json!(id);
            with_inner.nodes.push(Node { id, ..child });
            nodes.push(v);
        }
    }
    let mut placed_issues = HashSet::new();
    for v in nodes.iter().chain(&inner) {
        if v["item"]["kind"] == "issue"
            && let Some(id) = v["item"]["card"]["node"].as_str()
        {
            placed_issues.insert(id.to_string());
        }
    }
    if let Some(columns) = board["columns"].as_array_mut() {
        for column in columns {
            if let Some(cards) = column["cards"].as_array_mut() {
                cards.retain(|c| !c["node"].as_str().is_some_and(|n| placed_issues.contains(n)));
            }
        }
    }
    let arrows = arrows(ws, cache, &with_inner)?;
    let mut project = project_json(ws, cache, &project)?;
    project["repoId"] = json!(repo_id);
    let strokes = canvas.strokes();
    Ok(json!({
        "path": rel(ws, &path),
        "project": project,
        "board": board,
        "nodes": nodes,
        "arrows": arrows,
        "strokes": strokes,
    }))
}

/// Replaces the strokes drawn on the background of the canvas at `path`.
pub fn save_strokes(path: &Path, strokes: &[Stroke]) -> Result<()> {
    let mut canvas = load(path)?;
    canvas.set_strokes(strokes)?;
    write_canvas(path, &canvas)
}

/// The GitHub issue a URL names, by the mirror: its node id.
pub fn issue_by_url(cache: &Cache, url: &str) -> Result<Option<String>> {
    Ok(cache.conn().query_row("SELECT node_id FROM gh_issues WHERE url = ?1", [url], |r| r.get(0)).optional()?)
}

/// What a node stands for, for the window to draw.
pub fn item(ws: &Workspace, cache: &Cache, node: &Node) -> Result<Value> {
    if neoworks_kind(node) == Some("board") {
        return Ok(json!({"kind": "board", "uri": node.url}));
    }
    let uri = ws.node_uri(node);
    match node.kind.as_str() {
        "text" => Ok(json!({"kind": "text", "uri": null, "text": node.text})),
        "group" => Ok(json!({"kind": "group", "uri": null, "title": node.label})),
        "link" => {
            let url = node.url.clone().unwrap_or_default();
            if let Some(Uri::File { .. }) = &uri {
                let path = uri.as_ref().and_then(|u| ws.resolve_file(u));
                return Ok(file_item(ws, uri.as_ref().map(ToString::to_string), path));
            }
            if url.starts_with("https://github.com/")
                && let Some(id) = issue_by_url(cache, &url)?
                && let Some(card) = views::card(cache.conn(), &id)?
            {
                return Ok(json!({"kind": "issue", "uri": url, "card": card}));
            }
            let host = url.split("//").nth(1).and_then(|r| r.split('/').next()).unwrap_or(&url).to_string();
            let title = node
                .extra
                .get("title")
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| url.split("//").nth(1).unwrap_or(&url).trim_end_matches('/').to_string());
            let mut link = json!({"kind": "link", "uri": url, "title": title, "host": host});
            if let Some(preview) = previews::get_preview(cache, &url)? {
                link["preview"] = preview_json(ws, preview);
            }
            Ok(link)
        }
        "file" => {
            let rel_path = node.file.clone().unwrap_or_default();
            let path = ws.root().join(&rel_path);
            if rel_path.ends_with(".board") {
                return local_board(ws, &path);
            }
            if let Some(dir) = folder_of(&path) {
                return folder_item(ws, cache, uri.map(|u| u.to_string()), dir, &path);
            }
            if let Some(card) = local_issue_card(ws, &path) {
                return Ok(json!({"kind": "issue", "uri": uri.map(|u| u.to_string()), "card": card}));
            }
            Ok(file_item(ws, uri.map(|u| u.to_string()), Some(path)))
        }
        _ => Ok(json!({"kind": "missing", "uri": uri.map(|u| u.to_string()), "title": node.kind})),
    }
}

/// A link's preview with its images as absolute paths, for the window to draw, and the image's
/// width divided by its height.
fn preview_json(ws: &Workspace, preview: Preview) -> Value {
    let dir = crate::previews::dir(ws);
    let image = preview.image.map(|f| dir.join(f));
    let favicon = preview.favicon.map(|f| dir.join(f).to_string_lossy().into_owned());
    json!({
        "title": preview.title,
        "description": preview.description,
        "site": preview.site,
        "imageAspect": image.as_deref().and_then(aspect),
        "image": image.map(|p| p.to_string_lossy().into_owned()),
        "favicon": favicon,
    })
}

/// An image file's width divided by its height, from its header.
pub fn aspect(path: &Path) -> Option<f64> {
    let size = imagesize::size(path).ok()?;
    if size.height == 0 {
        return None;
    }
    Some(size.width as f64 / size.height as f64)
}

/// How many of a folder's nodes peek out of it on the canvas.
const PEEK: usize = 3;

/// A folder by its canvas: its directory, workspace-relative, how many nodes it holds, and the
/// first few of them, to peek out of it.
fn folder_item(ws: &Workspace, cache: &Cache, uri: Option<String>, dir: &Path, canvas_path: &Path) -> Result<Value> {
    if !dir.is_dir() {
        return Ok(json!({"kind": "missing", "uri": uri, "title": "Folder not found"}));
    }
    let nodes = load(canvas_path).map(|c| c.nodes).unwrap_or_default();
    let peek = nodes.iter().take(PEEK).map(|n| peek(ws, cache, n)).collect::<Result<Vec<_>>>()?;
    let name = dir.file_name().unwrap_or_default().to_string_lossy();
    Ok(json!({"kind": "folder", "uri": uri, "path": rel(ws, dir), "canvas": rel(ws, canvas_path), "title": name,
              "count": nodes.len(), "peek": peek}))
}

/// What a folder shows of a node inside it: its kind, its name, and a picture when it has one. A
/// folder inside shows only its name, so folders in folders aren't all read.
fn peek(ws: &Workspace, cache: &Cache, node: &Node) -> Result<Value> {
    if let Some(dir) = node.file.as_ref().map(|f| ws.root().join(f)).as_deref().and_then(folder_of) {
        return Ok(json!({"kind": "folder", "title": dir.file_name().unwrap_or_default().to_string_lossy()}));
    }
    let it = item(ws, cache, node)?;
    let title = match it["kind"].as_str() {
        Some("issue") => it["card"]["title"].clone(),
        Some("text") => it["text"].clone(),
        Some("link") => it["preview"]["title"].as_str().map_or(it["title"].clone(), |t| json!(t)),
        _ => it["title"].clone(),
    };
    let image = match it["kind"].as_str() {
        Some("image") => it["path"].clone(),
        Some("file") => it["media"]["poster"].clone(),
        Some("link") => it["preview"]["image"].clone(),
        _ => Value::Null,
    };
    Ok(json!({"kind": it["kind"], "title": title, "image": image, "mime": it["mime"]}))
}

/// The card of the local issue a file is, if it is one.
fn local_issue_card(ws: &Workspace, path: &Path) -> Option<Value> {
    let issue = issues::read_issue(path).ok()?;
    let project = ws.issue_project(path).ok()?;
    let siblings = ws.local_issues(&project);
    Some(ws.issue_card(&project, &issue, &siblings))
}

fn file_item(ws: &Workspace, uri: Option<String>, path: Option<PathBuf>) -> Value {
    let Some(path) = path.filter(|p| p.exists()) else {
        return json!({"kind": "missing", "uri": uri, "title": "File not found"});
    };
    let title = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let mime = workspace::mime_for(&path);
    if path.extension().is_some_and(|e| e == "md") {
        let note = read_note(&path);
        let (title, body) = note.map(|n| (n.title, n.body)).unwrap_or((title, String::new()));
        let excerpt: String = body.trim().chars().take(400).collect();
        return json!({"kind": "note", "uri": uri, "path": rel(ws, &path), "abs": path, "title": title, "excerpt": excerpt});
    }
    if mime.starts_with("image/") {
        return json!({"kind": "image", "uri": uri, "path": path, "title": title, "aspect": aspect(&path)});
    }
    let name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    if sheet::is_sheet(&path)
        && let Some(head) = sheet::head(&path)
    {
        return json!({"kind": "sheet", "uri": uri, "path": path, "title": name, "lang": sheet::lang(&path), "head": head});
    }
    let mut file = json!({"kind": "file", "uri": uri, "path": path, "title": name, "mime": mime});
    if let Some(made) = media::get(&crate::previews::dir(ws), &path) {
        file["media"] = media_json(ws, made);
    }
    file
}

/// A PDF's or video's preview for the window: the poster as an absolute path with its width
/// divided by its height, and the page count or length. A failed one has no poster.
fn media_json(ws: &Workspace, made: Media) -> Value {
    let poster = made.poster.map(|f| crate::previews::dir(ws).join(f));
    json!({
        "posterAspect": poster.as_deref().and_then(aspect),
        "poster": poster.map(|p| p.to_string_lossy().into_owned()),
        "pages": made.pages,
        "duration": made.duration,
    })
}

fn local_board(ws: &Workspace, path: &Path) -> Result<Value> {
    let board = workspace::read_board(path)?;
    let notes: Vec<_> = ws.notes().into_iter().filter(|n| board.matches(n, ws.root())).collect();
    let columns: Vec<Value> = workspace::board_notes(&board, &notes)
        .into_iter()
        .map(|col| {
            json!({
                "value": col.value,
                "label": col.label,
                "notes": col.notes.iter().map(|n| json!({
                    "path": rel(ws, &n.path),
                    "title": n.title,
                    "excerpt": n.body.trim().chars().take(140).collect::<String>(),
                })).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(json!({"kind": "localboard", "uri": format!("file:workspace/{}", rel(ws, path)), "path": rel(ws, path),
              "title": board.title.clone().unwrap_or_else(|| path.file_stem().unwrap_or_default().to_string_lossy().to_string()),
              "groupBy": board.group_by, "columns": columns}))
}

/// Each node's identity for relationships: a note's id, a file's URI, an issue's GitHub URL.
fn identities(ws: &Workspace, canvas: &Canvas) -> BTreeMap<String, String> {
    canvas.nodes.iter().filter_map(|n| ws.node_uri(n).map(|u| (u.to_string(), n.id.clone()))).collect()
}

/// The relationships between nodes of the canvas: GitHub's blocks and sub-issues between issues
/// on it, and `relations.jsonl`'s between anything on it.
pub fn arrows(ws: &Workspace, cache: &Cache, canvas: &Canvas) -> Result<Vec<Value>> {
    let ids = identities(ws, canvas);
    let mut out = Vec::new();
    let mut by_issue: HashMap<String, String> = HashMap::new();
    for (uri, node) in &ids {
        if uri.starts_with("https://github.com/")
            && let Some(issue) = issue_by_url(cache, uri)?
        {
            by_issue.insert(issue, node.clone());
        }
    }
    let mut stmt = cache.conn().prepare("SELECT kind, from_node, to_node FROM gh_links")?;
    let links: Vec<(String, String, String)> =
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<rusqlite::Result<_>>()?;
    for (kind, from, to) in links {
        if let (Some(a), Some(b)) = (by_issue.get(&from), by_issue.get(&to)) {
            let label = if kind == "blocks" { "blocks" } else { "parent of" };
            out.push(json!({"id": format!("gh:{kind}:{from}:{to}"), "type": if kind == "blocks" { "blocks" } else { "parent-of" },
                            "from": a, "to": b, "label": label, "source": "github",
                            "issues": [from, to]}));
        }
    }
    // Local issues placed on the canvas, by their node id.
    let local: HashMap<String, String> = canvas
        .nodes
        .iter()
        .filter_map(|n| {
            let file = n.file.as_deref()?;
            let path = ws.root().join(file);
            issues::read_issue(&path).ok().map(|_| (format!("{}{file}", issues::NODE_PREFIX), n.id.clone()))
        })
        .collect();
    let keys: Vec<String> = local.keys().cloned().collect();
    for (kind, from, to) in ws.issue_links(&keys) {
        let (Some(a), Some(b)) = (local.get(&from), local.get(&to)) else { continue };
        let (kind, label) = if kind == "blocks" { ("blocks", "blocks") } else { ("parent-of", "parent of") };
        out.push(json!({"id": format!("local:{kind}:{from}:{to}"), "type": kind, "from": a, "to": b, "label": label,
                        "source": "local", "issues": [from, to]}));
    }
    for r in ws.relations()? {
        if let (Some(a), Some(b)) = (ids.get(&r.from), ids.get(&r.to)) {
            let label = r.label.clone().or_else(|| (r.kind != "relates").then(|| r.kind.replace('-', " ")));
            out.push(json!({"id": r.id, "type": r.kind, "from": a, "to": b, "label": label, "source": "file"}));
        }
    }
    Ok(out)
}

/// Writes a canvas from the window's nodes (without what the host added to them), and its edges
/// from the relationships, so other JSON Canvas readers see the arrows too.
pub fn save(ws: &Workspace, cache: &Cache, path: &Path, nodes: &[Value]) -> Result<()> {
    let mut canvas = load(path)?;
    canvas.nodes = nodes
        .iter()
        .map(|n| {
            let mut n = n.clone();
            if let Some(obj) = n.as_object_mut() {
                obj.remove("item");
            }
            serde_json::from_value(n).map_err(Error::from)
        })
        .collect::<Result<_>>()?;
    let arrows = arrows(ws, cache, &canvas)?;
    canvas.edges = arrows
        .iter()
        .map(|a| {
            let mut extra = Map::new();
            extra.insert("neoworks".into(), json!({"relation": a["id"]}));
            workspace::Edge {
                id: a["id"].as_str().unwrap_or_default().chars().rev().take(16).collect(),
                from_node: a["from"].as_str().unwrap_or_default().to_string(),
                from_side: None,
                from_end: None,
                to_node: a["to"].as_str().unwrap_or_default().to_string(),
                to_side: None,
                to_end: Some("arrow".into()),
                color: None,
                label: a["label"].as_str().map(str::to_string),
                extra,
            }
        })
        .collect();
    write_canvas(path, &canvas)
}
