//! The open workspace: its folder, a connection to its cache, the sync thread and the file
//! watcher. One at a time; opening another replaces it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use np_core::workspace::{self, WatchHandle, Workspace};
use np_core::{Cache, Error, Result};
use serde_json::json;
use svelte_gpui::Emitter;

use crate::sync::{Sync, SyncStatus};

pub struct App {
    pub emitter: Emitter,
    open: Mutex<Option<Arc<Open>>>,
}

pub struct Open {
    pub root: PathBuf,
    pub ws: Mutex<Workspace>,
    pub cache: Mutex<Cache>,
    pub sync: Sync,
    /// Files the window itself wrote just now, so the watcher doesn't report them back.
    written: Mutex<HashMap<PathBuf, Instant>>,
    _watch: Mutex<Option<WatchHandle>>,
}

impl App {
    pub fn new(emitter: Emitter) -> App {
        App { emitter, open: Mutex::new(None) }
    }

    /// The open workspace, or an error the window can show.
    pub fn get(&self) -> Result<Arc<Open>> {
        self.open.lock().unwrap().clone().ok_or_else(|| Error::Other("no workspace is open".into()))
    }

    pub fn path(&self) -> Option<PathBuf> {
        self.open.lock().unwrap().as_ref().map(|o| o.root.clone())
    }

    /// Opens the workspace at `root`: indexes it, follows its files and starts syncing.
    pub fn open(&self, root: &Path) -> Result<Arc<Open>> {
        let root = expand(root);
        if !root.is_dir() {
            return Err(Error::Other(format!("{} is not a folder", root.display())));
        }
        let mut ws = Workspace::open(&root)?;
        if ws.config().github.login.is_none()
            && let Ok(login) = std::env::var("NP_GH_USER")
        {
            ws.set_login(Some(&login))?;
        }
        let cache = Cache::open(&ws.cache_path())?;
        workspace::index_all(&ws, &cache)?;
        let sync = Sync::start(ws.clone(), self.emitter.clone())?;
        let open = Arc::new(Open {
            root: root.clone(),
            ws: Mutex::new(ws),
            cache: Mutex::new(cache),
            sync,
            written: Mutex::new(HashMap::new()),
            _watch: Mutex::new(None),
        });
        let watch = watch(&open, self.emitter.clone())?;
        *open._watch.lock().unwrap() = Some(watch);
        *self.open.lock().unwrap() = Some(open.clone());
        remember(&root);
        Ok(open)
    }
}

impl Open {
    pub fn ws(&self) -> MutexGuard<'_, Workspace> {
        self.ws.lock().unwrap()
    }

    pub fn cache(&self) -> MutexGuard<'_, Cache> {
        self.cache.lock().unwrap()
    }

    /// Notes that the window wrote `path`, so the watcher's report of it is dropped.
    pub fn wrote(&self, path: &Path) {
        self.written.lock().unwrap().insert(path.to_path_buf(), Instant::now());
    }

    pub fn status(&self) -> SyncStatus {
        self.sync.status()
    }
}

/// Follows the workspace's files (and linked roots): keeps the index current, and tells the
/// window when something changed that it didn't write itself.
fn watch(open: &Arc<Open>, emitter: Emitter) -> Result<WatchHandle> {
    let weak = Arc::downgrade(open);
    let ws = open.ws().clone();
    let cache = Cache::open(&ws.cache_path())?;
    let cache = Mutex::new(cache);
    ws.watch(move |events| {
        let Some(open) = weak.upgrade() else { return };
        let ws = open.ws().clone();
        if let Err(e) = workspace::apply(&ws, &cache.lock().unwrap(), &events) {
            eprintln!("index: {e}");
        }
        let mut written = open.written.lock().unwrap();
        written.retain(|_, at| at.elapsed() < Duration::from_secs(3));
        let theirs = events.iter().any(|e| {
            let path = match e {
                workspace::WatchEvent::Created(p)
                | workspace::WatchEvent::Changed(p)
                | workspace::WatchEvent::Removed(p) => p,
                workspace::WatchEvent::Renamed { to, .. } => to,
            };
            !written.contains_key(path) && !path.components().any(|c| c.as_os_str() == workspace::DIR)
        });
        if theirs {
            emitter.emit("changed", json!({"reason": "files"}));
        }
    })
}

/// `~` at the start means the home directory.
pub fn expand(path: &Path) -> PathBuf {
    match path.strip_prefix("~") {
        Ok(rest) => std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default().join(rest),
        Err(_) => path.to_path_buf(),
    }
}

fn state_file() -> PathBuf {
    let config =
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| expand(Path::new("~/.config")));
    config.join("neoworks-projects").join("state.json")
}

/// The workspace opened last, to open again at start. `NP_WORKSPACE` wins.
pub fn last_workspace() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("NP_WORKSPACE") {
        return Some(expand(Path::new(&path)));
    }
    let text = std::fs::read_to_string(state_file()).ok()?;
    let state: serde_json::Value = serde_json::from_str(&text).ok()?;
    state["workspace"].as_str().map(PathBuf::from).filter(|p| p.is_dir())
}

fn remember(root: &Path) {
    if std::env::var_os("NP_WORKSPACE").is_some() {
        return;
    }
    let file = state_file();
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&file, json!({"workspace": root}).to_string());
}
