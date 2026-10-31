//! The disk side: what lives in the workspace folder and its linked roots.
//!
//! Disk is the truth; `cache.db` only indexes it ([`index`]). A [`Workspace`] is the folder, its
//! `.neoworks-projects/workspace.json`, and everything read from or written to files under it.
//! Everything here is plain and synchronous, safe to call from command threads.

pub mod board;
pub mod canvas;
pub mod index;
pub mod issues;
pub mod note;
pub mod project;
pub mod relations;
pub mod uri;
pub mod watch;

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub use board::{Board, BoardColumn, Column, Filter, board_notes, read_board, write_board};
pub use canvas::{Canvas, Edge, Node, folder_canvas_path, folder_of, read_canvas, write_canvas};
pub use index::{SearchHit, apply, index_all, index_path, mime_for, rename_path, search};
pub use note::{Note, attach_file, create_note, ensure_id, read_note, read_props, rename_prop, set_prop, write_body};
pub use project::{Project, ProjectError};
pub use relations::Relation;
pub use uri::Uri;
pub use watch::{WatchEvent, WatchHandle};

use crate::{Error, Result};

/// The app's own folder inside a workspace.
pub const DIR: &str = ".neoworks-projects";
/// The name of the workspace folder itself in `file:` URIs.
pub const WORKSPACE_ROOT: &str = "workspace";

/// `.neoworks-projects/workspace.json`. Keys we don't know are kept when it is saved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "one")]
    pub version: u32,
    /// Linked roots: name to folder (`~` allowed).
    #[serde(default)]
    pub roots: BTreeMap<String, String>,
    #[serde(default)]
    pub github: GithubConfig,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GithubConfig {
    pub login: Option<String>,
    #[serde(rename = "pollSeconds", default = "poll")]
    pub poll_seconds: u64,
}

fn one() -> u32 {
    1
}

fn poll() -> u64 {
    120
}

impl Default for GithubConfig {
    fn default() -> Self {
        GithubConfig { login: None, poll_seconds: poll() }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config { version: 1, roots: BTreeMap::new(), github: GithubConfig::default(), extra: Map::new() }
    }
}

/// A workspace folder with its config loaded.
#[derive(Debug, Clone)]
pub struct Workspace {
    root: PathBuf,
    config: Config,
}

impl Workspace {
    /// Opens the workspace at `root`, creating `.neoworks-projects/workspace.json` when missing.
    pub fn open(root: impl AsRef<Path>) -> Result<Workspace> {
        let root = root.as_ref().to_path_buf();
        let dir = root.join(DIR);
        io(std::fs::create_dir_all(&dir), &dir)?;
        let file = dir.join("workspace.json");
        let ws = match std::fs::read_to_string(&file) {
            Ok(text) => Workspace { root, config: serde_json::from_str(&text)? },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let ws = Workspace { root, config: Config::default() };
                ws.save()?;
                ws
            }
            Err(e) => return Err(io_error(e, &file)),
        };
        Ok(ws)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `<root>/.neoworks-projects`.
    pub fn dir(&self) -> PathBuf {
        self.root.join(DIR)
    }

    /// Where `cache.db` goes.
    pub fn cache_path(&self) -> PathBuf {
        self.dir().join("cache.db")
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Linked roots with `~` expanded, by name.
    pub fn roots(&self) -> Vec<(String, PathBuf)> {
        self.config.roots.iter().map(|(name, path)| (name.clone(), expand_home(path))).collect()
    }

    /// Links a folder under `name` (`harness:docs/x.md` style references use the name).
    pub fn set_root(&mut self, name: &str, path: &str) -> Result<()> {
        if name.is_empty() || name == WORKSPACE_ROOT || name.contains(['/', ':', '%']) {
            return Err(Error::Other(format!("{name:?} is not a usable root name")));
        }
        self.config.roots.insert(name.to_string(), path.to_string());
        self.save()
    }

    pub fn remove_root(&mut self, name: &str) -> Result<bool> {
        let had = self.config.roots.remove(name).is_some();
        if had {
            self.save()?;
        }
        Ok(had)
    }

    pub fn set_login(&mut self, login: Option<&str>) -> Result<()> {
        self.config.github.login = login.map(str::to_string);
        self.save()
    }

    fn save(&self) -> Result<()> {
        let mut text = serde_json::to_string_pretty(&self.config)?;
        text.push('\n');
        atomic_write(&self.dir().join("workspace.json"), text.as_bytes())
    }

    /// Every file under the workspace and the linked roots, hidden folders and dependency folders
    /// skipped, each once.
    pub fn files(&self) -> Vec<PathBuf> {
        let mut out = Vec::new();
        walk(&self.root, &mut out);
        for (_, root) in self.roots() {
            if !root.starts_with(&self.root) {
                walk(&root, &mut out);
            }
        }
        out
    }
}

/// `~` or `~/x` become `$HOME` or `$HOME/x`.
pub fn expand_home(path: &str) -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    match (path.strip_prefix('~'), home) {
        (Some(""), Some(home)) => home,
        (Some(rest), Some(home)) if rest.starts_with('/') => home.join(rest.trim_start_matches('/')),
        _ => PathBuf::from(path),
    }
}

/// Writes through a temp file and a rename, so a reader never sees half a file.
pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = path.with_file_name(format!(".{name}.{}.tmp", std::process::id()));
    let result = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    io(result, path)
}

pub(crate) fn io<T>(r: std::io::Result<T>, path: &Path) -> Result<T> {
    r.map_err(|e| io_error(e, path))
}

pub(crate) fn io_error(e: std::io::Error, path: &Path) -> Error {
    Error::Other(format!("{}: {e}", path.display()))
}

pub(crate) fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Whether a folder or file is left out of every scan.
pub(crate) fn skipped(name: &str) -> bool {
    name.starts_with('.') || name == "node_modules"
}

/// Files below `dir`, sorted, skipping hidden entries, `node_modules`, a Cargo `target`, and
/// symlinked folders.
pub(crate) fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(read) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = read.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if skipped(&name) {
            continue;
        }
        let path = entry.path();
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_dir() {
            if name == "target" && dir.join("Cargo.toml").exists() {
                continue;
            }
            walk(&path, out);
        } else if kind.is_file() || (kind.is_symlink() && path.is_file()) {
            out.push(path);
        }
    }
}

/// One entry of a folder, for the file explorer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub dir: bool,
}

/// What a folder holds, folders first, each group by name, leaving out what [`walk`] leaves out.
pub fn list_dir(dir: &Path) -> Result<Vec<DirEntry>> {
    let read = io(std::fs::read_dir(dir), dir)?;
    let mut entries: Vec<DirEntry> = read
        .filter_map(|e| e.ok())
        .filter(|e| !skipped(&e.file_name().to_string_lossy()))
        .filter_map(|e| {
            let path = e.path();
            let dir = path.is_dir();
            if dir && e.file_name() == "target" && path.parent().is_some_and(|p| p.join("Cargo.toml").exists()) {
                return None;
            }
            Some(DirEntry { name: e.file_name().to_string_lossy().into_owned(), path, dir })
        })
        .collect();
    entries.sort_by(|a, b| b.dir.cmp(&a.dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(entries)
}

/// A file name made from free text: no separators or reserved characters, no leading dots.
pub(crate) fn safe_filename(title: &str) -> String {
    let cleaned: String =
        title.chars().map(|c| if c.is_control() || "/\\:*?\"<>|#^[]".contains(c) { '-' } else { c }).collect();
    let cleaned = cleaned.trim().trim_start_matches('.').trim_end_matches(['.', ' ']).trim();
    let cleaned: String = cleaned.chars().take(120).collect();
    if cleaned.is_empty() { "Untitled".to_string() } else { cleaned }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_creates_and_reads_config() {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        let text = std::fs::read_to_string(dir.path().join(".neoworks-projects/workspace.json")).unwrap();
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v, serde_json::json!({"version":1,"roots":{},"github":{"login":null,"pollSeconds":120}}));
        assert_eq!(ws.config().github.poll_seconds, 120);
    }

    #[test]
    fn changes_are_saved_and_unknown_keys_kept() {
        let dir = tempfile::tempdir().unwrap();
        let mut ws = Workspace::open(dir.path()).unwrap();
        ws.set_root("harness", "~/code/harness").unwrap();
        ws.set_login(Some("moritz")).unwrap();
        let file = dir.path().join(".neoworks-projects/workspace.json");
        let mut v: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        v["neo"] = serde_json::json!({"enabled": true});
        std::fs::write(&file, v.to_string()).unwrap();

        let mut again = Workspace::open(dir.path()).unwrap();
        assert_eq!(again.config().roots["harness"], "~/code/harness");
        assert_eq!(again.config().github.login.as_deref(), Some("moritz"));
        assert!(again.remove_root("harness").unwrap());
        assert!(!again.remove_root("harness").unwrap());
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(v["neo"]["enabled"], true);
        assert_eq!(v["roots"], serde_json::json!({}));
    }

    #[test]
    fn bad_root_names_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut ws = Workspace::open(dir.path()).unwrap();
        assert!(ws.set_root("workspace", "/x").is_err());
        assert!(ws.set_root("a/b", "/x").is_err());
    }

    #[test]
    fn home_is_expanded() {
        let home = std::env::var("HOME").unwrap();
        assert_eq!(expand_home("~"), PathBuf::from(&home));
        assert_eq!(expand_home("~/a/b"), PathBuf::from(&home).join("a/b"));
        assert_eq!(expand_home("/abs"), PathBuf::from("/abs"));
        assert_eq!(expand_home("~other"), PathBuf::from("~other"));
    }

    #[test]
    fn filenames_are_made_safe() {
        assert_eq!(safe_filename("a/b: c?"), "a-b- c-");
        assert_eq!(safe_filename("  .hidden.. "), "hidden");
        assert_eq!(safe_filename("///"), "---");
        assert_eq!(safe_filename(""), "Untitled");
    }

    #[test]
    fn list_dir_puts_folders_first_and_skips_hidden_and_build_output() {
        let dir = tempfile::tempdir().unwrap();
        for f in ["b.md", "A.png", ".secret", "Cargo.toml"] {
            std::fs::write(dir.path().join(f), "").unwrap();
        }
        for d in ["zeta", "Alpha", "target", "node_modules", ".git"] {
            std::fs::create_dir(dir.path().join(d)).unwrap();
        }
        let names: Vec<(String, bool)> = list_dir(dir.path()).unwrap().into_iter().map(|e| (e.name, e.dir)).collect();
        let expected = [("Alpha", true), ("zeta", true), ("A.png", false), ("b.md", false), ("Cargo.toml", false)];
        assert_eq!(names, expected.map(|(n, d)| (n.to_string(), d)));
    }
}
