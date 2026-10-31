//! JSON Canvas 1.0 (`.canvas`). Keys we don't know, including our own `"neoworks"` hints, are
//! kept as they are so Obsidian and other tools lose nothing passing through us.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};

use super::note::read_note;
use super::{Workspace, atomic_write, io};
use crate::{Error, Result};

/// Whole numbers are written as integers, like Obsidian does.
fn num<S: Serializer>(v: &f64, s: S) -> std::result::Result<S::Ok, S::Error> {
    if v.fract() == 0.0 && v.abs() < 1e15 { s.serialize_i64(*v as i64) } else { s.serialize_f64(*v) }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Canvas {
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub edges: Vec<Edge>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    /// `text`, `file`, `link` or `group`.
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, serialize_with = "num")]
    pub x: f64,
    #[serde(default, serialize_with = "num")]
    pub y: f64,
    #[serde(default, serialize_with = "num")]
    pub width: f64,
    #[serde(default, serialize_with = "num")]
    pub height: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Path of a `file` node, relative to the workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// A group's label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// `subpath`, `background`, `backgroundStyle`, `neoworks`, and anything else.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    #[serde(rename = "fromNode")]
    pub from_node: String,
    #[serde(rename = "fromSide", default, skip_serializing_if = "Option::is_none")]
    pub from_side: Option<String>,
    #[serde(rename = "fromEnd", default, skip_serializing_if = "Option::is_none")]
    pub from_end: Option<String>,
    #[serde(rename = "toNode")]
    pub to_node: String,
    #[serde(rename = "toSide", default, skip_serializing_if = "Option::is_none")]
    pub to_side: Option<String>,
    #[serde(rename = "toEnd", default, skip_serializing_if = "Option::is_none")]
    pub to_end: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Node {
    /// Our hint object on the node, if any.
    pub fn neoworks(&self) -> Option<&Map<String, Value>> {
        self.extra.get("neoworks").and_then(Value::as_object)
    }
}

impl Edge {
    pub fn neoworks(&self) -> Option<&Map<String, Value>> {
        self.extra.get("neoworks").and_then(Value::as_object)
    }
}

/// An empty file is an empty canvas (Obsidian makes new ones that way).
pub fn read_canvas(path: &Path) -> Result<Canvas> {
    let text = io(std::fs::read_to_string(path), path)?;
    if text.trim().is_empty() {
        return Ok(Canvas::default());
    }
    serde_json::from_str(&text).map_err(|e| Error::Other(format!("{}: {e}", path.display())))
}

/// A folder on a canvas is a directory with a canvas of its own inside, named after it:
/// `Garden/Seeds/Seeds.canvas`.
pub fn folder_canvas_path(dir: &Path) -> PathBuf {
    dir.join(format!("{}.canvas", dir.file_name().unwrap_or_default().to_string_lossy()))
}

/// The directory of the folder whose canvas `path` is; none for any other file.
pub fn folder_of(path: &Path) -> Option<&Path> {
    path.parent().filter(|dir| folder_canvas_path(dir) == path)
}

impl Canvas {
    /// The first place in a grid of four columns where a `w` × `h` node overlaps nothing.
    pub fn free_spot(&self, w: f64, h: f64) -> (f64, f64) {
        let gap = 40.0;
        for row in 0..200 {
            for col in 0..4 {
                let (x, y) = (col as f64 * (w + gap), row as f64 * (h + gap));
                let hit = self.nodes.iter().any(|n| {
                    x < n.x + n.width + gap / 2.0
                        && n.x < x + w + gap / 2.0
                        && y < n.y + n.height + gap / 2.0
                        && n.y < y + h + gap / 2.0
                });
                if !hit {
                    return (x, y);
                }
            }
        }
        (0.0, 0.0)
    }
}

/// Writes pretty JSON indented with tabs, through a temp file and a rename.
pub fn write_canvas(path: &Path, canvas: &Canvas) -> Result<()> {
    let mut out = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(b"\t");
    let mut ser = serde_json::Serializer::with_formatter(&mut out, fmt);
    canvas.serialize(&mut ser)?;
    out.push(b'\n');
    atomic_write(path, &out)
}

impl Workspace {
    /// The identity of the item a node stands for: the `neoworks.uri` hint if present; a `file`
    /// node's note id when the file is a note with one, else its `file:` URI; a `link` node's URL.
    pub fn node_uri(&self, node: &Node) -> Option<super::Uri> {
        use super::Uri;
        if let Some(uri) = node.neoworks().and_then(|n| n.get("uri")).and_then(Value::as_str)
            && let Ok(u) = Uri::parse(uri)
        {
            return Some(u);
        }
        match node.kind.as_str() {
            "file" => {
                let rel = node.file.as_deref()?.trim_start_matches('/');
                let uri = Uri::file(super::WORKSPACE_ROOT, rel);
                let path = self.resolve_file(&uri)?;
                if path.extension().is_some_and(|e| e == "md")
                    && let Some(id) = read_note(&path).ok().and_then(|n| n.id)
                {
                    return Some(Uri::Note(id));
                }
                Some(uri)
            }
            "link" => Uri::parse(node.url.as_deref()?).ok(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Uri;

    const OBSIDIAN: &str = r##"{
	"nodes":[
		{"id":"a","type":"file","file":"Harness/Idea.md","x":-120,"y":40,"width":400,"height":300,"color":"4","styleAttributes":{"x":1}},
		{"id":"b","type":"link","url":"https://github.com/o/r/issues/5","x":500,"y":40.5,"width":250,"height":60,"neoworks":{"uri":"https://github.com/o/r/issues/5","pin":true}},
		{"id":"g","type":"group","label":"Q1","x":0,"y":0,"width":900,"height":600,"background":"bg.png","backgroundStyle":"cover"}
	],
	"edges":[
		{"id":"e","fromNode":"a","fromSide":"right","toNode":"b","toSide":"left","toEnd":"arrow","label":"blocks","neoworks":{"relation":"r1"}}
	],
	"metadata":{"version":"1.0-1.0"}
}"##;

    #[test]
    fn round_trips_with_unknown_keys() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("c.canvas");
        std::fs::write(&p, OBSIDIAN).unwrap();
        let c = read_canvas(&p).unwrap();
        assert_eq!(c.nodes.len(), 3);
        assert_eq!(c.nodes[0].extra["styleAttributes"], serde_json::json!({"x": 1}));
        assert_eq!(c.nodes[1].neoworks().unwrap()["pin"], true);
        assert_eq!(c.edges[0].neoworks().unwrap()["relation"], "r1");
        assert_eq!(c.extra["metadata"]["version"], "1.0-1.0");

        write_canvas(&p, &c).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("\n\t\"nodes\""));
        let again: Value = serde_json::from_str(&text).unwrap();
        let before: Value = serde_json::from_str(OBSIDIAN).unwrap();
        assert_eq!(again, before);
        assert!(text.contains("\"x\": -120,"), "integers stay integers: {text}");
        assert!(text.contains("\"y\": 40.5"));
        assert_eq!(read_canvas(&p).unwrap(), c);
    }

    #[test]
    fn empty_file_is_an_empty_canvas() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("c.canvas");
        std::fs::write(&p, "").unwrap();
        assert_eq!(read_canvas(&p).unwrap(), Canvas::default());
        std::fs::write(&p, "{}").unwrap();
        assert_eq!(read_canvas(&p).unwrap(), Canvas::default());
    }

    #[test]
    fn node_uris() {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        std::fs::create_dir_all(dir.path().join("H")).unwrap();
        std::fs::write(dir.path().join("H/With.md"), "---\nid: abc\n---\n").unwrap();
        std::fs::write(dir.path().join("H/Without.md"), "text").unwrap();
        std::fs::write(dir.path().join("H/pic.png"), "x").unwrap();

        let node = |kind: &str, file: Option<&str>, url: Option<&str>| Node {
            id: "n".into(),
            kind: kind.into(),
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            color: None,
            text: None,
            file: file.map(Into::into),
            url: url.map(Into::into),
            label: None,
            extra: Map::new(),
        };
        assert_eq!(ws.node_uri(&node("file", Some("H/With.md"), None)), Some(Uri::Note("abc".into())));
        assert_eq!(
            ws.node_uri(&node("file", Some("H/Without.md"), None)),
            Some(Uri::file("workspace", "H/Without.md"))
        );
        assert_eq!(ws.node_uri(&node("file", Some("H/pic.png"), None)), Some(Uri::file("workspace", "H/pic.png")));
        assert_eq!(
            ws.node_uri(&node("link", None, Some("https://github.com/o/r/issues/1"))),
            Some(Uri::Web("https://github.com/o/r/issues/1".into()))
        );
        let mut hinted = node("link", None, Some("https://example.com"));
        hinted.extra.insert("neoworks".into(), serde_json::json!({"uri": "neo:session/s1"}));
        assert_eq!(ws.node_uri(&hinted), Some(Uri::Session("s1".into())));
        assert_eq!(ws.node_uri(&node("text", None, None)), None);
        assert_eq!(ws.node_uri(&node("group", None, None)), None);
    }
}
