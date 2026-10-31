//! The index of the disk in `cache.db`: files, notes, local relations and full-text search.
//!
//! Nothing here is truth; deleting `cache.db` and calling [`index_all`] rebuilds it. The watcher
//! keeps it current with [`index_path`] and [`apply`].

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use rusqlite::{Connection, OptionalExtension, params};

use super::note::parse_note;
use super::project::project_at;
use super::watch::WatchEvent;
use super::{Uri, Workspace, skipped, walk};
use crate::cache::iso;
use crate::{Cache, Result};

/// One full-text hit.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub uri: String,
    pub title: String,
    /// The matching part of the body, matches in `[brackets]`.
    pub snippet: String,
}

/// The MIME type for a file name, by extension.
pub fn mime_for(path: &Path) -> &'static str {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "md" | "markdown" => "text/markdown",
        "txt" | "log" => "text/plain",
        "canvas" | "json" | "jsonl" => "application/json",
        "board" | "yaml" | "yml" => "application/yaml",
        "toml" => "application/toml",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "text/javascript",
        "ts" => "text/typescript",
        "rs" => "text/x-rust",
        "csv" => "text/csv",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "tif" | "tiff" => "image/tiff",
        "bmp" => "image/bmp",
        "ico" => "image/vnd.microsoft.icon",
        "heic" => "image/heic",
        "heif" => "image/heif",
        "avif" => "image/avif",
        "jxl" => "image/jxl",
        "jp2" => "image/jp2",
        "tga" => "image/x-tga",
        "psd" => "image/vnd.adobe.photoshop",
        "exr" => "image/x-exr",
        "hdr" => "image/vnd.radiance",
        "pdf" => "application/pdf",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "mkv" => "video/x-matroska",
        "zip" => "application/zip",
        _ => "application/octet-stream",
    }
}

fn hash_file(path: &Path) -> std::io::Result<String> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(std::fs::File::open(path)?)?;
    Ok(hasher.finalize().to_hex().to_string())
}

/// The nearest project folder above a note, remembered per folder.
struct Projects {
    root: PathBuf,
    memo: HashMap<PathBuf, Option<String>>,
}

impl Projects {
    fn new(root: &Path) -> Projects {
        Projects { root: root.to_path_buf(), memo: HashMap::new() }
    }

    fn of(&mut self, path: &Path) -> Option<String> {
        for dir in path.ancestors().skip(1) {
            if dir == self.root || !dir.starts_with(&self.root) {
                break;
            }
            if let Some(name) = self.memo.entry(dir.to_path_buf()).or_insert_with(|| project_at(dir).map(|p| p.name)) {
                return Some(name.clone());
            }
        }
        None
    }
}

/// Whether a path is somewhere the index does not look: below a hidden folder of its root.
fn ignored(ws: &Workspace, path: &Path) -> bool {
    let mut roots = vec![ws.root().to_path_buf()];
    roots.extend(ws.roots().into_iter().map(|(_, p)| p));
    roots
        .iter()
        .find_map(|r| path.strip_prefix(r).ok())
        .is_none_or(|rel| rel.components().any(|c| skipped(&c.as_os_str().to_string_lossy())))
}

fn remove(conn: &Connection, uri: &str) -> Result<()> {
    for sql in
        ["DELETE FROM files WHERE uri = ?1", "DELETE FROM notes WHERE uri = ?1", "DELETE FROM search WHERE uri = ?1"]
    {
        conn.execute(sql, [uri])?;
    }
    Ok(())
}

/// Removes the file at `uri` and everything below it (a folder that went away).
fn remove_tree(conn: &Connection, uri: &str) -> Result<()> {
    remove(conn, uri)?;
    let prefix = format!("{uri}/");
    for sql in [
        "DELETE FROM files WHERE substr(uri, 1, length(?1)) = ?1",
        "DELETE FROM notes WHERE substr(uri, 1, length(?1)) = ?1",
        "DELETE FROM search WHERE substr(uri, 1, length(?1)) = ?1",
    ] {
        conn.execute(sql, [&prefix])?;
    }
    Ok(())
}

/// Inserts or updates the row of one file. `force` rehashes even when size and mtime are unchanged.
fn upsert(conn: &Connection, ws: &Workspace, projects: &mut Projects, path: &Path, force: bool) -> Result<()> {
    let Some(uri @ Uri::File { .. }) = ws.uri_for_path(path) else { return Ok(()) };
    let Uri::File { root, path: rel } = &uri else { return Ok(()) };
    let uri_s = uri.to_string();
    let Ok(meta) = std::fs::metadata(path) else { return remove(conn, &uri_s) };
    if !meta.is_file() {
        return Ok(());
    }
    let size = meta.len() as i64;
    let mtime =
        meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as i64);
    let is_md = path.extension().is_some_and(|e| e == "md");

    let old: Option<(i64, i64, String)> = conn
        .query_row("SELECT size, mtime, hash FROM files WHERE uri = ?1", [&uri_s], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .optional()?;
    let same = old.as_ref().is_some_and(|(s, m, _)| *s == size && *m == mtime);
    if same && !force && !is_md {
        return Ok(());
    }
    let hash = match old {
        Some((_, _, h)) if same && !force => h,
        _ => match hash_file(path) {
            Ok(h) => h,
            Err(_) => return remove(conn, &uri_s),
        },
    };

    let text = if is_md { std::fs::read_to_string(path).ok() } else { None };
    let note = text.as_ref().map(|t| parse_note(path, t));
    conn.execute(
        "INSERT OR REPLACE INTO files (uri, root, path, hash, size, mtime, mime, note_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![uri_s, root, rel, hash, size, mtime, mime_for(path), note.as_ref().and_then(|n| n.id.clone())],
    )?;
    conn.execute("DELETE FROM notes WHERE uri = ?1", [&uri_s])?;
    conn.execute("DELETE FROM search WHERE uri = ?1", [&uri_s])?;
    if let Some(n) = note {
        let project = projects.of(path);
        conn.execute(
            "INSERT OR REPLACE INTO notes (id, uri, path, title, props, project, updated) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                n.id.clone().unwrap_or_else(|| uri_s.clone()),
                uri_s,
                path.to_string_lossy(),
                n.title,
                serde_json::to_string(&n.props)?,
                project,
                iso(mtime.div_euclid(1000))
            ],
        )?;
        conn.execute("INSERT INTO search (uri, title, body) VALUES (?1, ?2, ?3)", params![uri_s, n.title, n.body])?;
    }
    Ok(())
}

fn index_relations(conn: &Connection, ws: &Workspace) -> Result<()> {
    let all = ws.relations()?;
    conn.execute("DELETE FROM relations WHERE source = 'file'", [])?;
    for r in all {
        conn.execute(
            "INSERT OR REPLACE INTO relations (id, type, from_uri, to_uri, label, data, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'file')",
            params![r.id, r.kind, r.from, r.to, r.label, r.data.map(|d| d.to_string())],
        )?;
    }
    Ok(())
}

/// Brings the whole index in line with the disk: new and changed files in, vanished ones out.
pub fn index_all(ws: &Workspace, cache: &Cache) -> Result<()> {
    let tx = cache.conn().unchecked_transaction()?;
    let mut projects = Projects::new(ws.root());
    let mut seen = HashSet::new();
    for path in ws.files() {
        if let Some(uri) = ws.uri_for_path(&path) {
            seen.insert(uri.to_string());
            upsert(&tx, ws, &mut projects, &path, false)?;
        }
    }
    let known: Vec<String> = {
        let mut stmt = tx.prepare("SELECT uri FROM files")?;
        stmt.query_map([], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?
    };
    for uri in known.into_iter().filter(|u| !seen.contains(u)) {
        remove(&tx, &uri)?;
    }
    index_relations(&tx, ws)?;
    tx.commit()?;
    Ok(())
}

/// Updates the index for one path that was created, changed or removed: a file, a folder (all
/// of it), or `relations.jsonl`.
pub fn index_path(ws: &Workspace, cache: &Cache, path: &Path) -> Result<()> {
    let tx = cache.conn().unchecked_transaction()?;
    if path == ws.relations_path() {
        index_relations(&tx, ws)?;
    } else if !ignored(ws, path)
        && let Some(uri) = ws.uri_for_path(path).map(|u| u.to_string())
    {
        let mut projects = Projects::new(ws.root());
        if path.is_dir() {
            let mut files = Vec::new();
            walk(path, &mut files);
            let mut seen = HashSet::new();
            for f in &files {
                if let Some(u) = ws.uri_for_path(f) {
                    seen.insert(u.to_string());
                }
                upsert(&tx, ws, &mut projects, f, false)?;
            }
            let prefix = format!("{uri}/");
            let known: Vec<String> = {
                let mut stmt = tx.prepare("SELECT uri FROM files WHERE substr(uri, 1, length(?1)) = ?1")?;
                stmt.query_map([&prefix], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?
            };
            for u in known.into_iter().filter(|u| !seen.contains(u)) {
                remove(&tx, &u)?;
            }
        } else if path.is_file() {
            upsert(&tx, ws, &mut projects, path, true)?;
        } else {
            remove_tree(&tx, &uri)?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// A file or folder moved: updates the index at both places and points relations that named the
/// old `file:` URI (or anything below it) at the new one. Notes keep their `note:` identity
/// across a rename, so only `file:` ends change.
pub fn rename_path(ws: &Workspace, cache: &Cache, from: &Path, to: &Path) -> Result<()> {
    if let (Some(old), Some(new)) = (ws.uri_for_path(from), ws.uri_for_path(to)) {
        let (old, new) = (old.to_string(), new.to_string());
        ws.rewrite_endpoint(&old, &new)?;
        ws.rewrite_prefix(&old, &new)?;
    }
    index_path(ws, cache, from)?;
    index_path(ws, cache, to)?;
    index_path(ws, cache, &ws.relations_path())
}

/// Applies watcher events to the index and to relations (renames).
pub fn apply(ws: &Workspace, cache: &Cache, events: &[WatchEvent]) -> Result<()> {
    for event in events {
        match event {
            WatchEvent::Created(p) | WatchEvent::Changed(p) | WatchEvent::Removed(p) => index_path(ws, cache, p)?,
            WatchEvent::Renamed { from, to } => rename_path(ws, cache, from, to)?,
        }
    }
    Ok(())
}

/// Full-text search over notes. Each word matches as a prefix; all must match.
pub fn search(cache: &Cache, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|t| t.replace('"', ""))
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{t}\"*"))
        .collect();
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    let mut stmt = cache.conn().prepare(
        "SELECT uri, title, snippet(search, 2, '[', ']', '…', 12) FROM search WHERE search MATCH ?1 ORDER BY rank LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![terms.join(" "), limit as i64], |r| {
        Ok(SearchHit { uri: r.get(0)?, title: r.get(1)?, snippet: r.get(2)? })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::note::set_prop;

    struct Fixture {
        dir: tempfile::TempDir,
        ws: Workspace,
        cache: Cache,
    }

    fn fixture() -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        let cache = Cache::open_in_memory().unwrap();
        Fixture { dir, ws, cache }
    }

    impl Fixture {
        fn write(&self, rel: &str, text: &str) -> PathBuf {
            let p = self.dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, text).unwrap();
            p
        }

        fn count(&self, table: &str) -> i64 {
            self.cache.conn().query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0)).unwrap()
        }
    }

    #[test]
    fn index_all_fills_files_notes_search() {
        let f = fixture();
        f.write("Harness/Harness.md", "---\ntype: project\nid: p1\n---\n");
        f.write("Harness/Idea one.md", "---\nid: n1\nstatus: todo\n---\nthe daemon needs splitting\n");
        f.write("Loose.md", "no front matter, daemon too");
        f.write("pic.PNG", "png");
        f.write(".hidden/x.md", "secret");
        index_all(&f.ws, &f.cache).unwrap();

        assert_eq!(f.count("files"), 4);
        assert_eq!(f.count("notes"), 3);
        let (title, project, props): (String, Option<String>, String) = f
            .cache
            .conn()
            .query_row("SELECT title, project, props FROM notes WHERE id = 'n1'", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap();
        assert_eq!((title.as_str(), project.as_deref()), ("Idea one", Some("Harness")));
        assert!(props.contains("\"status\":\"todo\""));
        // A note without an id is keyed by its uri.
        let loose: String =
            f.cache.conn().query_row("SELECT id FROM notes WHERE title = 'Loose'", [], |r| r.get(0)).unwrap();
        assert_eq!(loose, "file:workspace/Loose.md");
        let (mime, note_id): (String, Option<String>) = f
            .cache
            .conn()
            .query_row("SELECT mime, note_id FROM files WHERE path = 'pic.PNG'", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!((mime.as_str(), note_id), ("image/png", None));

        let hits = search(&f.cache, "daem split", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].uri, "file:workspace/Harness/Idea%20one.md");
        assert!(hits[0].snippet.contains("[daemon]"));
        assert_eq!(search(&f.cache, "daemon", 10).unwrap().len(), 2);
        assert!(search(&f.cache, "  \"", 10).unwrap().is_empty());
    }

    #[test]
    fn index_all_is_repeatable_and_drops_vanished_files() {
        let f = fixture();
        let a = f.write("a.md", "alpha");
        f.write("b.md", "beta");
        index_all(&f.ws, &f.cache).unwrap();
        index_all(&f.ws, &f.cache).unwrap();
        assert_eq!((f.count("files"), f.count("notes"), f.count("search")), (2, 2, 2));
        std::fs::remove_file(a).unwrap();
        index_all(&f.ws, &f.cache).unwrap();
        assert_eq!((f.count("files"), f.count("notes"), f.count("search")), (1, 1, 1));
    }

    #[test]
    fn index_path_inserts_updates_deletes() {
        let f = fixture();
        let p = f.write("Note.md", "---\nid: n1\n---\nfirst body");
        index_path(&f.ws, &f.cache, &p).unwrap();
        assert_eq!(f.count("notes"), 1);
        let h1: String = f.cache.conn().query_row("SELECT hash FROM files", [], |r| r.get(0)).unwrap();

        set_prop(&p, "status", Some(&serde_json::json!("doing"))).unwrap();
        std::fs::write(&p, std::fs::read_to_string(&p).unwrap().replace("first", "second")).unwrap();
        index_path(&f.ws, &f.cache, &p).unwrap();
        let h2: String = f.cache.conn().query_row("SELECT hash FROM files", [], |r| r.get(0)).unwrap();
        assert_ne!(h1, h2);
        assert_eq!(f.count("notes"), 1);
        assert_eq!(search(&f.cache, "second", 5).unwrap().len(), 1);
        assert!(search(&f.cache, "first", 5).unwrap().is_empty());

        std::fs::remove_file(&p).unwrap();
        index_path(&f.ws, &f.cache, &p).unwrap();
        assert_eq!((f.count("files"), f.count("notes"), f.count("search")), (0, 0, 0));
    }

    #[test]
    fn index_path_handles_folders_and_ignores_hidden() {
        let f = fixture();
        f.write("d/a.md", "a");
        f.write("d/sub/b.md", "b");
        let hidden = f.write(".git/x.md", "x");
        index_path(&f.ws, &f.cache, &f.dir.path().join("d")).unwrap();
        index_path(&f.ws, &f.cache, &hidden).unwrap();
        assert_eq!(f.count("files"), 2);
        std::fs::remove_dir_all(f.dir.path().join("d")).unwrap();
        index_path(&f.ws, &f.cache, &f.dir.path().join("d")).unwrap();
        assert_eq!((f.count("files"), f.count("search")), (0, 0));
    }

    #[test]
    fn relations_are_indexed() {
        let f = fixture();
        let r = f.ws.add_relation("relates", "note:1", "note:2", Some("x"), None, "user").unwrap();
        index_path(&f.ws, &f.cache, &f.ws.relations_path()).unwrap();
        let (from, label): (String, Option<String>) = f
            .cache
            .conn()
            .query_row("SELECT from_uri, label FROM relations WHERE id = ?1", [&r.id], |x| Ok((x.get(0)?, x.get(1)?)))
            .unwrap();
        assert_eq!((from.as_str(), label.as_deref()), ("note:1", Some("x")));
        f.ws.remove_relation(&r.id).unwrap();
        index_all(&f.ws, &f.cache).unwrap();
        assert_eq!(f.count("relations"), 0);
    }

    #[test]
    fn rename_moves_rows_and_rewrites_relations() {
        let f = fixture();
        let a = f.write("img/logo.png", "png");
        f.write("img/deep/x.png", "x");
        f.ws.add_relation(
            "derived-from",
            "file:workspace/img/logo-v2.png",
            "file:workspace/img/logo.png",
            None,
            None,
            "u",
        )
        .unwrap();
        f.ws.add_relation("relates", "note:1", "file:workspace/img/deep/x.png", None, None, "u").unwrap();
        index_all(&f.ws, &f.cache).unwrap();

        let b = f.dir.path().join("img/logo-final.png");
        std::fs::rename(&a, &b).unwrap();
        rename_path(&f.ws, &f.cache, &a, &b).unwrap();
        let paths: Vec<String> = {
            let mut s = f.cache.conn().prepare("SELECT path FROM files ORDER BY path").unwrap();
            s.query_map([], |r| r.get(0)).unwrap().map(|r| r.unwrap()).collect()
        };
        assert_eq!(paths, ["img/deep/x.png", "img/logo-final.png"]);
        let to: String = f
            .cache
            .conn()
            .query_row("SELECT to_uri FROM relations WHERE type = 'derived-from'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(to, "file:workspace/img/logo-final.png");

        let (from_dir, to_dir) = (f.dir.path().join("img"), f.dir.path().join("pics"));
        std::fs::rename(&from_dir, &to_dir).unwrap();
        apply(&f.ws, &f.cache, &[WatchEvent::Renamed { from: from_dir, to: to_dir }]).unwrap();
        let rels = f.ws.relations().unwrap();
        assert_eq!(rels[1].to, "file:workspace/pics/deep/x.png");
        assert_eq!(rels[0].to, "file:workspace/pics/logo-final.png");
        let n: i64 =
            f.cache.conn().query_row("SELECT count(*) FROM files WHERE path LIKE 'pics/%'", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 2);
        assert_eq!(f.count("files"), 2);
    }

    #[test]
    fn linked_roots_are_indexed_under_their_name() {
        let mut f = fixture();
        let other = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(other.path().join("docs")).unwrap();
        std::fs::write(other.path().join("docs/design.md"), "design doc").unwrap();
        f.ws.set_root("harness", other.path().to_str().unwrap()).unwrap();
        index_all(&f.ws, &f.cache).unwrap();
        let uri: String =
            f.cache.conn().query_row("SELECT uri FROM files WHERE root = 'harness'", [], |r| r.get(0)).unwrap();
        assert_eq!(uri, "file:harness/docs/design.md");
        index_path(&f.ws, &f.cache, &other.path().join("docs/design.md")).unwrap();
        assert_eq!(f.count("files"), 1);
    }

    #[test]
    fn fts5_and_migration_from_version_1() {
        // A version 1 database keeps its rows when opened by this build.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache.db");
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(include_str!("../schema.sql")).unwrap();
            conn.execute("INSERT INTO meta (key, value) VALUES ('k', 'v')", []).unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
        }
        let cache = Cache::open(&path).unwrap();
        let v: String = cache.conn().query_row("SELECT value FROM meta WHERE key = 'k'", [], |r| r.get(0)).unwrap();
        assert_eq!(v, "v");
        let version: i64 = cache.conn().pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(version, 4);
        cache.conn().execute("INSERT INTO search (uri, title, body) VALUES ('u', 't', 'hello world')", []).unwrap();
        drop(cache);
        assert!(Cache::open(&path).is_ok());
    }
}
