//! The sync thread: sends the outbox and pulls every project's repository, on an interval and
//! whenever the window asks, with a connection of its own so the window never waits on GitHub.

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use np_core::github::{self, Client, Http};
use np_core::workspace::Workspace;
use np_core::{Cache, Result};
use serde::Serialize;
use serde_json::json;
use svelte_gpui::Emitter;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub running: bool,
    pub pending: usize,
    pub failed: Vec<Failure>,
    pub conflicts: usize,
    pub last_sync: Option<String>,
    pub error: Option<String>,
    pub login: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub target: String,
    pub field: Option<String>,
    pub error: String,
}

enum Msg {
    /// Push and pull everything now.
    Now,
    /// An edit was made: push soon (edits made in a burst go out together).
    Soon,
    /// The issue's panel opened: fetch its comments.
    Comments(String),
}

pub struct Sync {
    tx: Sender<Msg>,
    status: Arc<Mutex<SyncStatus>>,
}

impl Sync {
    pub fn start(ws: Workspace, emitter: Emitter) -> Result<Sync> {
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(SyncStatus { login: ws.config().github.login.clone(), ..Default::default() }));
        let mut cache = Cache::open(&ws.cache_path())?;
        let shared = status.clone();
        std::thread::spawn(move || {
            let poll = Duration::from_secs(ws.config().github.poll_seconds.max(30));
            let mut next_pull = Instant::now();
            let mut token: Option<(Http, Instant)> = None;
            loop {
                let wait = next_pull.saturating_duration_since(Instant::now());
                let (pull, comments) = match rx.recv_timeout(wait) {
                    Ok(Msg::Now) => (true, None),
                    Ok(Msg::Soon) => {
                        std::thread::sleep(Duration::from_millis(600));
                        (false, None)
                    }
                    Ok(Msg::Comments(node)) => (false, Some(node)),
                    Err(RecvTimeoutError::Timeout) => (true, None),
                    Err(RecvTimeoutError::Disconnected) => return,
                };
                // Read the workspace again: projects come and go, and the login may change.
                let ws = Workspace::open(ws.root()).unwrap_or_else(|_| ws.clone());
                let login = ws.config().github.login.clone();
                if token.as_ref().is_none_or(|(_, at)| at.elapsed() > Duration::from_secs(30 * 60)) {
                    token = Http::from_gh(login.as_deref()).ok().map(|h| (h, Instant::now()));
                }
                let Some((http, _)) = token.take() else {
                    let mut s = shared.lock().unwrap();
                    s.error = Some("Not logged in to GitHub: run `gh auth login`.".into());
                    s.login = login;
                    emitter.emit("sync", json!(*s));
                    next_pull = Instant::now() + poll;
                    continue;
                };
                let client = Client::new(http);
                if let Some(node) = comments {
                    if github::pull_comments(&mut cache, &client, &node).is_ok() {
                        emitter.emit("changed", json!({"reason": "comments"}));
                    }
                    token = Some((client_into_http(client), Instant::now()));
                    continue;
                }
                set(&shared, &emitter, |s| {
                    s.running = true;
                    s.login = login.clone();
                });
                let mut error = None;
                let pushed = github::push(&mut cache, &client);
                if let Err(e) = &pushed {
                    error = Some(e.to_string());
                }
                let mut changed = pushed.as_ref().is_ok_and(|r| r.sent > 0);
                if pull {
                    let (projects, _) = ws.projects();
                    for repo in projects.iter().flat_map(|p| p.repos.first()) {
                        let Some((owner, name)) = repo.split_once('/') else { continue };
                        match github::pull(&mut cache, &client, owner, name) {
                            Ok(report) => changed |= report.issues > 0,
                            Err(e) => error = Some(format!("{repo}: {e}")),
                        }
                    }
                    next_pull = Instant::now() + poll;
                }
                token = Some((client_into_http(client), Instant::now()));
                let pending = cache.outbox().unwrap_or_default();
                let conflicts = cache.conflicts().map(|c| c.len()).unwrap_or(0);
                set(&shared, &emitter, |s| {
                    s.running = false;
                    s.pending = pending.len();
                    s.failed = pending
                        .iter()
                        .filter(|e| e.state == "failed")
                        .map(|e| Failure {
                            target: e.target.clone(),
                            field: e.field.clone(),
                            error: e.last_error.clone().unwrap_or_default(),
                        })
                        .collect();
                    s.conflicts = conflicts;
                    s.error = error.clone();
                    if pull && error.is_none() {
                        s.last_sync = Some(now_iso());
                    }
                });
                if changed {
                    emitter.emit("changed", json!({"reason": "sync"}));
                }
            }
        });
        Ok(Sync { tx, status })
    }

    pub fn now(&self) {
        let _ = self.tx.send(Msg::Now);
    }

    pub fn soon(&self) {
        let _ = self.tx.send(Msg::Soon);
    }

    pub fn comments(&self, node: &str) {
        let _ = self.tx.send(Msg::Comments(node.to_string()));
    }

    pub fn status(&self) -> SyncStatus {
        self.status.lock().unwrap().clone()
    }
}

fn set(status: &Mutex<SyncStatus>, emitter: &Emitter, change: impl FnOnce(&mut SyncStatus)) {
    let mut s = status.lock().unwrap();
    change(&mut s);
    emitter.emit("sync", json!(*s));
}

fn client_into_http(client: Client<Http>) -> Http {
    client.into_transport()
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    np_core::cache::iso(secs)
}
