//! Item identities: `note:<uuid>`, `file:<root>/<path>`, web URLs and `neo:session/<id>`.

use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

use super::{WORKSPACE_ROOT, Workspace};
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Uri {
    /// A note by the `id` in its front matter.
    Note(String),
    /// A file by root (`workspace` or a linked root's name) and the path below it, `/` separated.
    File { root: String, path: String },
    /// A web page, GitHub issues included: the whole `http(s)` URL.
    Web(String),
    /// An AI chat in the neo daemon.
    Session(String),
}

impl Uri {
    pub fn parse(s: &str) -> Result<Uri> {
        let bad = || Error::Other(format!("not an item URI: {s:?}"));
        if let Some(id) = s.strip_prefix("note:") {
            return if id.is_empty() { Err(bad()) } else { Ok(Uri::Note(id.to_string())) };
        }
        if let Some(rest) = s.strip_prefix("file:") {
            let (root, path) = rest.split_once('/').unwrap_or((rest, ""));
            if root.is_empty() {
                return Err(bad());
            }
            return Ok(Uri::File { root: root.to_string(), path: decode(path).ok_or_else(bad)? });
        }
        if let Some(id) = s.strip_prefix("neo:session/") {
            return if id.is_empty() { Err(bad()) } else { Ok(Uri::Session(id.to_string())) };
        }
        if s.starts_with("https://") || s.starts_with("http://") {
            return Ok(Uri::Web(s.to_string()));
        }
        Err(bad())
    }

    /// A `file:` URI for a path below `root`.
    pub fn file(root: &str, path: &str) -> Uri {
        Uri::File { root: root.to_string(), path: path.to_string() }
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Uri::Note(id) => write!(f, "note:{id}"),
            Uri::File { root, path } => write!(f, "file:{root}/{}", encode(path)),
            Uri::Web(url) => f.write_str(url),
            Uri::Session(id) => write!(f, "neo:session/{id}"),
        }
    }
}

impl FromStr for Uri {
    type Err = Error;
    fn from_str(s: &str) -> Result<Uri> {
        Uri::parse(s)
    }
}

/// Percent-encodes everything but unreserved characters and `/`.
fn encode(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = s.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

impl Workspace {
    /// The folder a root name stands for.
    pub fn root_path(&self, root: &str) -> Option<PathBuf> {
        if root == WORKSPACE_ROOT {
            return Some(self.root().to_path_buf());
        }
        self.roots().into_iter().find(|(name, _)| name == root).map(|(_, path)| path)
    }

    /// The path a `file:` URI points at. `None` for other URIs, unknown roots and paths that climb out with `..`.
    pub fn resolve_file(&self, uri: &Uri) -> Option<PathBuf> {
        let Uri::File { root, path } = uri else { return None };
        let mut out = self.root_path(root)?;
        for part in path.split('/').filter(|p| !p.is_empty()) {
            if part == ".." {
                return None;
            }
            out.push(part);
        }
        Some(out)
    }

    /// The `file:` URI of an absolute path, under the first root that contains it: the workspace, then linked roots.
    pub fn uri_for_path(&self, path: &Path) -> Option<Uri> {
        let mut roots = vec![(WORKSPACE_ROOT.to_string(), self.root().to_path_buf())];
        roots.extend(self.roots());
        for (name, root) in roots {
            if let Ok(rel) = path.strip_prefix(&root) {
                let mut parts = Vec::new();
                for c in rel.components() {
                    match c {
                        Component::Normal(s) => parts.push(s.to_string_lossy().into_owned()),
                        _ => return None,
                    }
                }
                return Some(Uri::File { root: name, path: parts.join("/") });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        for s in [
            "note:0192f5c4-aaaa",
            "file:workspace/Harness/Split%20the%20daemon.md",
            "file:harness/docs/design.md",
            "https://github.com/o/r/issues/5",
            "http://example.com/a?b=c",
            "neo:session/abc",
        ] {
            assert_eq!(Uri::parse(s).unwrap().to_string(), s);
        }
    }

    #[test]
    fn encodes_awkward_paths() {
        let u = Uri::file("workspace", "Ünï/a b#c%.md");
        let s = u.to_string();
        assert_eq!(s, "file:workspace/%C3%9Cn%C3%AF/a%20b%23c%25.md");
        assert_eq!(Uri::parse(&s).unwrap(), u);
    }

    #[test]
    fn rejects_garbage() {
        for s in ["", "note:", "file:", "file:/x", "ftp://x", "file:workspace/%zz", "neo:session/"] {
            assert!(Uri::parse(s).is_err(), "{s}");
        }
    }

    #[test]
    fn resolves_and_maps_paths_back() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let mut ws = Workspace::open(dir.path()).unwrap();
        ws.set_root("harness", other.path().to_str().unwrap()).unwrap();

        let p = dir.path().join("A/b c.md");
        let uri = ws.uri_for_path(&p).unwrap();
        assert_eq!(uri, Uri::file("workspace", "A/b c.md"));
        assert_eq!(ws.resolve_file(&uri).unwrap(), p);

        let q = other.path().join("docs/x.md");
        let uri = ws.uri_for_path(&q).unwrap();
        assert_eq!(uri.to_string(), "file:harness/docs/x.md");
        assert_eq!(ws.resolve_file(&uri).unwrap(), q);

        assert!(ws.uri_for_path(Path::new("/nowhere/else")).is_none());
        assert!(ws.resolve_file(&Uri::file("workspace", "../x")).is_none());
        assert!(ws.resolve_file(&Uri::file("nope", "x")).is_none());
        assert!(ws.resolve_file(&Uri::Note("x".into())).is_none());
    }

    #[test]
    fn workspace_wins_over_a_root_inside_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut ws = Workspace::open(dir.path()).unwrap();
        ws.set_root("inner", dir.path().join("sub").to_str().unwrap()).unwrap();
        let uri = ws.uri_for_path(&dir.path().join("sub/a.md")).unwrap();
        assert_eq!(uri, Uri::file("workspace", "sub/a.md"));
    }
}
