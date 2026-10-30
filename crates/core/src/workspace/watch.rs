//! Watching the workspace and linked roots for changes made outside the app.
//!
//! Events are debounced and renames are paired, so the caller sees one `Renamed { from, to }`
//! and can hand it to [`super::index::rename_path`] (which updates the index and relations).
//! Hidden folders (`.git`, our own `.neoworks-projects`) are not reported, except
//! `relations.jsonl`.

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify::event::{ModifyKind, RenameMode};
use notify::{EventKind, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};

use super::{Workspace, skipped};
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum WatchEvent {
    Created(PathBuf),
    Changed(PathBuf),
    Removed(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

/// Keeps the watch alive; dropping it stops watching.
pub struct WatchHandle {
    _debouncer: Debouncer<RecommendedWatcher, RecommendedCache>,
}

impl Workspace {
    /// Watches the workspace and its linked roots (recursively), calling `callback` with each
    /// batch of events after a 300 ms quiet period. The callback runs on a watcher thread.
    pub fn watch(&self, callback: impl Fn(Vec<WatchEvent>) + Send + 'static) -> Result<WatchHandle> {
        self.watch_with(Duration::from_millis(300), callback)
    }

    pub fn watch_with(
        &self,
        debounce: Duration,
        callback: impl Fn(Vec<WatchEvent>) + Send + 'static,
    ) -> Result<WatchHandle> {
        let mut roots = vec![self.root().to_path_buf()];
        roots.extend(self.roots().into_iter().map(|(_, p)| p).filter(|p| p.is_dir() && !p.starts_with(self.root())));
        let relations = self.relations_path();
        let filter_roots = roots.clone();

        let mut debouncer = new_debouncer(debounce, None, move |result: DebounceEventResult| {
            let Ok(events) = result else { return };
            let mut out = Vec::new();
            for e in events {
                out.extend(translate(&e.event, &filter_roots, &relations));
            }
            if !out.is_empty() {
                callback(out);
            }
        })
        .map_err(|e| Error::Other(format!("watch: {e}")))?;
        for root in &roots {
            debouncer
                .watch(root, RecursiveMode::Recursive)
                .map_err(|e| Error::Other(format!("watch {}: {e}", root.display())))?;
        }
        Ok(WatchHandle { _debouncer: debouncer })
    }
}

/// Whether a path is reported: not below a hidden folder of its root.
fn wanted(path: &Path, roots: &[PathBuf], relations: &Path) -> bool {
    if path == relations {
        return true;
    }
    roots
        .iter()
        .find_map(|r| path.strip_prefix(r).ok())
        .is_some_and(|rel| !rel.components().any(|c| skipped(&c.as_os_str().to_string_lossy())))
}

fn translate(event: &notify::Event, roots: &[PathBuf], relations: &Path) -> Vec<WatchEvent> {
    let paths = &event.paths;
    let keep = |p: &PathBuf| wanted(p, roots, relations);
    match &event.kind {
        EventKind::Create(_) => paths.iter().filter(|p| keep(p)).cloned().map(WatchEvent::Created).collect(),
        EventKind::Remove(_) => paths.iter().filter(|p| keep(p)).cloned().map(WatchEvent::Removed).collect(),
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) if paths.len() == 2 => {
            match (keep(&paths[0]), keep(&paths[1])) {
                (true, true) => vec![WatchEvent::Renamed { from: paths[0].clone(), to: paths[1].clone() }],
                (true, false) => vec![WatchEvent::Removed(paths[0].clone())],
                (false, true) => vec![WatchEvent::Created(paths[1].clone())],
                _ => Vec::new(),
            }
        }
        // A half of a rename whose other half was not seen: it left, or it arrived.
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
            paths.iter().filter(|p| keep(p)).cloned().map(WatchEvent::Removed).collect()
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
            paths.iter().filter(|p| keep(p)).cloned().map(WatchEvent::Created).collect()
        }
        EventKind::Modify(_) => paths.iter().filter(|p| keep(p)).cloned().map(WatchEvent::Changed).collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn collect(rx: &mpsc::Receiver<Vec<WatchEvent>>, until: impl Fn(&[WatchEvent]) -> bool) -> Vec<WatchEvent> {
        let mut all = Vec::new();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            if let Ok(batch) = rx.recv_timeout(Duration::from_millis(200)) {
                all.extend(batch);
                if until(&all) {
                    break;
                }
            }
        }
        all
    }

    fn start(dir: &Path) -> (Workspace, WatchHandle, mpsc::Receiver<Vec<WatchEvent>>) {
        let ws = Workspace::open(dir).unwrap();
        let (tx, rx) = mpsc::channel();
        let tx = std::sync::Mutex::new(tx);
        let handle = ws
            .watch_with(Duration::from_millis(100), move |events| {
                let _ = tx.lock().unwrap().send(events);
            })
            .unwrap();
        (ws, handle, rx)
    }

    #[test]
    fn reports_create_change_remove_and_ignores_hidden() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        let (_ws, _handle, rx) = start(&root);

        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join(".git/ignored"), "x").unwrap();
        let p = root.join("a.md");
        std::fs::write(&p, "one").unwrap();
        let got =
            collect(&rx, |e| e.iter().any(|x| matches!(x, WatchEvent::Created(q) | WatchEvent::Changed(q) if *q == p)));
        assert!(got.iter().any(|e| matches!(e, WatchEvent::Created(q) | WatchEvent::Changed(q) if *q == p)), "{got:?}");

        std::thread::sleep(Duration::from_millis(300));
        std::fs::write(&p, "two").unwrap();
        let got = collect(&rx, |e| e.iter().any(|x| matches!(x, WatchEvent::Changed(q) if *q == p)));
        assert!(got.iter().any(|e| matches!(e, WatchEvent::Changed(q) if *q == p)), "{got:?}");

        std::fs::remove_file(&p).unwrap();
        let got = collect(&rx, |e| e.iter().any(|x| matches!(x, WatchEvent::Removed(q) if *q == p)));
        assert!(got.iter().any(|e| matches!(e, WatchEvent::Removed(q) if *q == p)), "{got:?}");
        assert!(!got.iter().any(|e| format!("{e:?}").contains(".git")));
    }

    #[test]
    fn pairs_a_rename() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        std::fs::write(root.join("old.md"), "x").unwrap();
        let (_ws, _handle, rx) = start(&root);

        let (from, to) = (root.join("old.md"), root.join("new.md"));
        std::fs::rename(&from, &to).unwrap();
        let want = WatchEvent::Renamed { from, to };
        let got = collect(&rx, |e| e.contains(&want));
        assert!(got.contains(&want), "{got:?}");
    }

    #[test]
    fn translate_filters_hidden_paths() {
        let roots = vec![PathBuf::from("/w")];
        let rel = PathBuf::from("/w/.neoworks-projects/relations.jsonl");
        let ev =
            |p: &str| notify::Event::new(EventKind::Create(notify::event::CreateKind::File)).add_path(PathBuf::from(p));
        assert!(translate(&ev("/w/.neoworks-projects/cache.db"), &roots, &rel).is_empty());
        assert!(translate(&ev("/w/.git/HEAD"), &roots, &rel).is_empty());
        assert!(translate(&ev("/elsewhere/a.md"), &roots, &rel).is_empty());
        assert_eq!(translate(&ev("/w/sub/a.md"), &roots, &rel).len(), 1);
        assert_eq!(translate(&ev("/w/.neoworks-projects/relations.jsonl"), &roots, &rel).len(), 1);
    }
}
