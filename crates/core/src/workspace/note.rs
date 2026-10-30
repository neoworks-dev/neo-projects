//! Markdown notes with YAML front matter.
//!
//! The front matter is the user's: comments, quoting, key order and spacing stay as they wrote
//! them. We read it with a YAML parser but write by editing lines of the block as text, only
//! the lines of the key being changed.

use std::ops::Range;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use super::{Workspace, atomic_write, io, new_id, safe_filename};
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// The `id` in the front matter, if it has one yet.
    pub id: Option<String>,
    /// The file name without `.md`.
    pub title: String,
    pub props: Map<String, Value>,
    /// Everything after the front matter block, as written.
    pub body: String,
    pub path: PathBuf,
}

/// Where the front matter block sits in a file.
struct Split {
    /// The lines between the two `---`.
    inner: Range<usize>,
    /// Where the body starts: after the closing line.
    body: usize,
}

fn split(text: &str) -> Option<Split> {
    let mut lines = text.split_inclusive('\n');
    let first = lines.next()?;
    if first.trim_end() != "---" {
        return None;
    }
    let start = first.len();
    let mut pos = start;
    for line in lines {
        if matches!(line.trim_end(), "---" | "...") {
            return Some(Split { inner: start..pos, body: pos + line.len() });
        }
        pos += line.len();
    }
    None
}

/// The front matter as JSON; empty when there is none or it doesn't parse.
fn parse_props(text: &str) -> Map<String, Value> {
    let Some(s) = split(text) else { return Map::new() };
    match serde_yaml_ng::from_str::<Value>(&text[s.inner]) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

pub(crate) fn parse_note(path: &Path, text: &str) -> Note {
    let props = parse_props(text);
    let body = split(text).map_or(text, |s| &text[s.body..]).to_string();
    let id = props.get("id").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string);
    let title = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    Note { id, title, props, body, path: path.to_path_buf() }
}

pub fn read_note(path: &Path) -> Result<Note> {
    let text = io(std::fs::read_to_string(path), path)?;
    Ok(parse_note(path, &text))
}

/// One value as a YAML scalar or flow list (JSON for anything nested).
fn render(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) if s.contains('\n') => v.to_string(),
        Value::String(s) => {
            serde_yaml_ng::to_string(s).map(|y| y.trim_end().to_string()).unwrap_or_else(|_| v.to_string())
        }
        Value::Array(items) => format!("[{}]", items.iter().map(render).collect::<Vec<_>>().join(", ")),
        Value::Object(_) => v.to_string(),
    }
}

fn check_key(key: &str) -> Result<()> {
    if key.is_empty()
        || key.contains([':', '#', '\n', '"', '\''])
        || key.starts_with(['-', '[', '{'])
        || key != key.trim()
    {
        return Err(Error::Other(format!("{key:?} is not a usable front matter key")));
    }
    Ok(())
}

/// The front matter's keys and values in the order the file lists them; empty when there is
/// none or it doesn't parse. `Note::props` is sorted by key, which a properties list can't show.
pub fn read_props(path: &Path) -> Result<Vec<(String, Value)>> {
    let text = io(std::fs::read_to_string(path), path)?;
    let Some(s) = split(&text) else { return Ok(Vec::new()) };
    let Ok(serde_yaml_ng::Value::Mapping(map)) = serde_yaml_ng::from_str(&text[s.inner]) else {
        return Ok(Vec::new());
    };
    let mut props = Vec::new();
    for (key, value) in map {
        let (Some(key), Ok(value)) = (key.as_str(), serde_json::to_value(value)) else { continue };
        props.push((key.to_string(), value));
    }
    Ok(props)
}

/// Which of the block's lines sets the top-level `key`.
fn key_line(lines: &[&str], key: &str) -> Option<usize> {
    lines
        .iter()
        .position(|l| l.strip_prefix(key).is_some_and(|rest| rest.trim_start_matches([' ', '\t']).starts_with(':')))
}

/// A line that continues the value of the key above it.
fn continues(line: &str) -> bool {
    line.starts_with([' ', '\t']) || line.starts_with("- ") || line.trim_end() == "-" || line.trim().is_empty()
}

/// `text` with the top-level `key` set to `value` (removed on `None`), every other byte untouched.
fn edit(text: &str, key: &str, value: Option<&Value>) -> String {
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let rendered = value.map(render);
    let Some(s) = split(text) else {
        return match rendered {
            None => text.to_string(),
            Some(r) => format!("---{eol}{key}: {r}{eol}---{eol}{text}"),
        };
    };
    let inner = &text[s.inner.clone()];
    let lines: Vec<&str> = inner.split_inclusive('\n').collect();
    let (from, to) = match key_line(&lines, key) {
        Some(i) => {
            let mut j = i + 1;
            while j < lines.len() && continues(lines[j]) {
                j += 1;
            }
            while j > i + 1 && lines[j - 1].trim().is_empty() {
                j -= 1;
            }
            (lines[..i].iter().map(|l| l.len()).sum::<usize>(), lines[..j].iter().map(|l| l.len()).sum::<usize>())
        }
        None => (inner.len(), inner.len()),
    };
    let old = &inner[from..to];
    let replacement = match rendered {
        None => String::new(),
        Some(r) => {
            // Keep a comment trailing the old single-line value.
            let comment = old
                .split_once(':')
                .map(|(_, v)| v)
                .filter(|v| v.trim_end().lines().count() <= 1 && !v.trim_start().starts_with(['"', '\'']))
                .and_then(|v| v.find(" #").map(|i| v[i..].trim_end()))
                .unwrap_or("");
            format!("{key}: {r}{comment}{eol}")
        }
    };
    let abs = s.inner.start;
    format!("{}{}{}", &text[..abs + from], replacement, &text[abs + to..])
}

/// Sets (or with `None` removes) one top-level front matter key. Creates the block when absent.
/// `value` is a scalar or a list; the rest of the block is left byte for byte.
pub fn set_prop(path: &Path, key: &str, value: Option<&Value>) -> Result<()> {
    check_key(key)?;
    let text = io(std::fs::read_to_string(path), path)?;
    let out = edit(&text, key, value);
    if out != text {
        atomic_write(path, out.as_bytes())?;
    }
    Ok(())
}

/// Renames the top-level key `from` to `to` on its line, keeping its value, a comment after it
/// and every other byte. Refuses when `to` is already set.
pub fn rename_prop(path: &Path, from: &str, to: &str) -> Result<()> {
    check_key(to)?;
    if from == to {
        return Ok(());
    }
    let text = io(std::fs::read_to_string(path), path)?;
    let Some(s) = split(&text) else { return Err(Error::Other(format!("{from:?} is not set"))) };
    let lines: Vec<&str> = text[s.inner.clone()].split_inclusive('\n').collect();
    if key_line(&lines, to).is_some() {
        return Err(Error::Other(format!("{to:?} is already set")));
    }
    let Some(at) = key_line(&lines, from) else { return Err(Error::Other(format!("{from:?} is not set"))) };
    let start = s.inner.start + lines[..at].iter().map(|l| l.len()).sum::<usize>();
    let out = format!("{}{to}{}", &text[..start], &text[start + from.len()..]);
    atomic_write(path, out.as_bytes())
}

/// The note's `id`, written as a new UUIDv7 first if it has none.
pub fn ensure_id(path: &Path) -> Result<String> {
    if let Some(id) = read_note(path)?.id {
        return Ok(id);
    }
    let id = new_id();
    set_prop(path, "id", Some(&Value::String(id.clone())))?;
    Ok(id)
}

/// Replaces everything after the front matter; the block itself is kept byte for byte.
pub fn write_body(path: &Path, body: &str) -> Result<()> {
    let text = io(std::fs::read_to_string(path), path)?;
    let head = split(&text).map_or(0, |s| s.body);
    atomic_write(path, format!("{}{body}", &text[..head]).as_bytes())
}

/// Makes `<title>.md` in `folder` (`Title 2.md` when taken) with the given front matter, and an
/// `id` unless `props` has one. Returns the new path.
pub fn create_note(folder: &Path, title: &str, props: &Map<String, Value>) -> Result<PathBuf> {
    io(std::fs::create_dir_all(folder), folder)?;
    let mut head = String::from("---\n");
    if !props.contains_key("id") {
        head.push_str(&format!("id: {}\n", new_id()));
    }
    for (k, v) in props {
        check_key(k)?;
        head.push_str(&format!("{k}: {}\n", render(v)));
    }
    head.push_str("---\n");

    let stem = safe_filename(title);
    for n in 1.. {
        let name = if n == 1 { format!("{stem}.md") } else { format!("{stem} {n}.md") };
        let path = folder.join(name);
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut f) => {
                use std::io::Write;
                io(f.write_all(head.as_bytes()), &path)?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(super::io_error(e, &path)),
        }
    }
    unreachable!()
}

/// Copies `source` into `folder` under its own name (`photo 2.png` when taken), for a note there
/// to link. A file already in `folder` stays where it is. Returns the file's path in `folder`.
pub fn attach_file(folder: &Path, source: &Path) -> Result<PathBuf> {
    if source.parent() == Some(folder) {
        return Ok(source.to_path_buf());
    }
    let Some(name) = source.file_name().map(Path::new) else {
        return Err(Error::Other(format!("{} is not a file", source.display())));
    };
    let stem = name.file_stem().unwrap_or_default().to_string_lossy();
    let extension = name.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    for n in 1.. {
        let candidate = if n == 1 { format!("{stem}{extension}") } else { format!("{stem} {n}{extension}") };
        let path = folder.join(candidate);
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => {
                io(std::fs::copy(source, &path), source)?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(super::io_error(e, &path)),
        }
    }
    unreachable!()
}

impl Workspace {
    /// Every `.md` file in the workspace as a note.
    pub fn notes(&self) -> Vec<Note> {
        let mut files = Vec::new();
        super::walk(self.root(), &mut files);
        files
            .into_iter()
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .filter_map(|p| read_note(&p).ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SAMPLE: &str = "---\n# mine\nid:   keep-me   # why\ntags:\n  - a\n  - b\n\nstatus: todo # note\nlist: [x,   y]\n---\n# Heading\n\ntext\n";

    fn file(text: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("Note.md");
        std::fs::write(&p, text).unwrap();
        (dir, p)
    }

    #[test]
    fn reads_props_title_and_body() {
        let (_d, p) = file(SAMPLE);
        let n = read_note(&p).unwrap();
        assert_eq!(n.id.as_deref(), Some("keep-me"));
        assert_eq!(n.title, "Note");
        assert_eq!(n.props["tags"], json!(["a", "b"]));
        assert_eq!(n.props["list"], json!(["x", "y"]));
        assert_eq!(n.body, "# Heading\n\ntext\n");
    }

    #[test]
    fn no_front_matter_is_all_body() {
        let (_d, p) = file("just text\n---\nnot front matter\n");
        let n = read_note(&p).unwrap();
        assert!(n.props.is_empty() && n.id.is_none());
        assert_eq!(n.body, "just text\n---\nnot front matter\n");
    }

    #[test]
    fn broken_yaml_still_reads() {
        let (_d, p) = file("---\na: [unclosed\n---\nbody");
        let n = read_note(&p).unwrap();
        assert!(n.props.is_empty());
        assert_eq!(n.body, "body");
    }

    #[test]
    fn set_prop_replaces_one_line_only() {
        let (_d, p) = file(SAMPLE);
        set_prop(&p, "status", Some(&json!("doing"))).unwrap();
        let got = std::fs::read_to_string(&p).unwrap();
        assert_eq!(got, SAMPLE.replace("status: todo # note", "status: doing # note"));
    }

    #[test]
    fn set_prop_replaces_a_block_list_and_keeps_neighbours() {
        let (_d, p) = file(SAMPLE);
        set_prop(&p, "tags", Some(&json!(["c", "d e"]))).unwrap();
        let got = std::fs::read_to_string(&p).unwrap();
        assert!(got.contains("tags: [c, d e]\n\nstatus: todo # note\n"), "{got}");
        assert!(got.starts_with("---\n# mine\nid:   keep-me   # why\n"));
        assert!(got.ends_with("---\n# Heading\n\ntext\n"));
    }

    #[test]
    fn set_prop_appends_and_removes() {
        let (_d, p) = file(SAMPLE);
        set_prop(&p, "priority", Some(&json!(2))).unwrap();
        let got = std::fs::read_to_string(&p).unwrap();
        assert_eq!(got, SAMPLE.replace("list: [x,   y]\n", "list: [x,   y]\npriority: 2\n"));
        set_prop(&p, "priority", None).unwrap();
        set_prop(&p, "status", None).unwrap();
        let got = std::fs::read_to_string(&p).unwrap();
        assert_eq!(got, SAMPLE.replace("status: todo # note\n", ""));
        set_prop(&p, "absent", None).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), got);
    }

    #[test]
    fn set_prop_quotes_when_needed_and_creates_the_block() {
        let (_d, p) = file("body\n");
        set_prop(&p, "title", Some(&json!("a: b"))).unwrap();
        set_prop(&p, "flag", Some(&json!("true"))).unwrap();
        let got = std::fs::read_to_string(&p).unwrap();
        assert!(got.ends_with("---\nbody\n"));
        let n = read_note(&p).unwrap();
        assert_eq!(n.props["title"], json!("a: b"));
        assert_eq!(n.props["flag"], json!("true"));
        assert_eq!(n.body, "body\n");
    }

    #[test]
    fn crlf_is_kept() {
        let (_d, p) = file("---\r\na: 1\r\n---\r\nbody\r\n");
        set_prop(&p, "b", Some(&json!(2))).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "---\r\na: 1\r\nb: 2\r\n---\r\nbody\r\n");
    }

    #[test]
    fn keys_that_merely_start_alike_are_not_touched() {
        let (_d, p) = file("---\nstatus_old: 1\nstatus: 2\n---\n");
        set_prop(&p, "status", Some(&json!(3))).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "---\nstatus_old: 1\nstatus: 3\n---\n");
    }

    #[test]
    fn bad_keys_are_refused() {
        let (_d, p) = file("x");
        assert!(set_prop(&p, "a: b", Some(&json!(1))).is_err());
        assert!(set_prop(&p, "", Some(&json!(1))).is_err());
    }

    #[test]
    fn ensure_id_writes_once() {
        let (_d, p) = file("---\n# c\nx: 1\n---\nbody");
        let id = ensure_id(&p).unwrap();
        assert_eq!(uuid::Uuid::parse_str(&id).unwrap().get_version_num(), 7);
        assert_eq!(ensure_id(&p).unwrap(), id);
        let got = std::fs::read_to_string(&p).unwrap();
        assert_eq!(got, format!("---\n# c\nx: 1\nid: {id}\n---\nbody"));
        let (_d2, q) = file("---\nid: mine\n---\n");
        assert_eq!(ensure_id(&q).unwrap(), "mine");
    }

    #[test]
    fn write_body_keeps_the_block() {
        let (_d, p) = file(SAMPLE);
        write_body(&p, "new\n").unwrap();
        let got = std::fs::read_to_string(&p).unwrap();
        assert_eq!(got, SAMPLE.replace("# Heading\n\ntext\n", "new\n"));
        let n = read_note(&p).unwrap();
        write_body(&p, &n.body).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), got);

        let (_d, q) = file("plain");
        write_body(&q, "other").unwrap();
        assert_eq!(std::fs::read_to_string(&q).unwrap(), "other");
    }

    #[test]
    fn create_note_is_safe_and_unique() {
        let dir = tempfile::tempdir().unwrap();
        let mut props = Map::new();
        props.insert("status".into(), json!("todo"));
        props.insert("tags".into(), json!(["a"]));
        let a = create_note(dir.path(), "Plan: v1/v2", &props).unwrap();
        let b = create_note(dir.path(), "Plan: v1/v2", &props).unwrap();
        assert_eq!(a.file_name().unwrap(), "Plan- v1-v2.md");
        assert_eq!(b.file_name().unwrap(), "Plan- v1-v2 2.md");
        let n = read_note(&a).unwrap();
        assert!(n.id.is_some());
        assert_eq!(n.props["status"], json!("todo"));
        assert_eq!(n.props["tags"], json!(["a"]));
        assert_ne!(read_note(&b).unwrap().id, n.id);
    }
}
