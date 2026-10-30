//! `.neoworks-projects/relations.jsonl`: every relationship GitHub can't hold, one per line.
//!
//! Sorted by id (UUIDv7, so by creation time) so the file diffs well, and rewritten atomically.
//! Issue-to-issue `blocks` and `parent-of` belong on GitHub; the caller routes those there.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Workspace, atomic_write, io, new_id};
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    /// ISO 8601 UTC.
    pub created: String,
    /// Who made it: `user`, `plugin:comfy`, `agent:…`.
    pub by: String,
}

/// Read-modify-write of the file is one step at a time within the process.
static LOCK: Mutex<()> = Mutex::new(());

impl Workspace {
    pub fn relations_path(&self) -> PathBuf {
        self.dir().join("relations.jsonl")
    }

    pub fn relations(&self) -> Result<Vec<Relation>> {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        self.load_relations()
    }

    fn load_relations(&self) -> Result<Vec<Relation>> {
        let path = self.relations_path();
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(super::io_error(e, &path)),
        };
        let mut out = Vec::new();
        for (i, line) in text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty()) {
            let r =
                serde_json::from_str(line).map_err(|e| Error::Other(format!("{}:{}: {e}", path.display(), i + 1)))?;
            out.push(r);
        }
        Ok(out)
    }

    fn store_relations(&self, mut all: Vec<Relation>) -> Result<()> {
        all.sort_by(|a, b| a.id.cmp(&b.id));
        let mut text = String::new();
        for r in &all {
            text.push_str(&serde_json::to_string(r)?);
            text.push('\n');
        }
        io(std::fs::create_dir_all(self.dir()), &self.dir())?;
        atomic_write(&self.relations_path(), text.as_bytes())
    }

    /// Records a relationship. `kind` is `relates`, `derived-from`, `<plugin>:<name>`, …
    pub fn add_relation(
        &self,
        kind: &str,
        from: &str,
        to: &str,
        label: Option<&str>,
        data: Option<Value>,
        by: &str,
    ) -> Result<Relation> {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut all = self.load_relations()?;
        let r = Relation {
            id: new_id(),
            kind: kind.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            label: label.map(str::to_string),
            data,
            created: crate::cache::now(),
            by: by.to_string(),
        };
        all.push(r.clone());
        self.store_relations(all)?;
        Ok(r)
    }

    /// Removes one by id; whether it was there.
    pub fn remove_relation(&self, id: &str) -> Result<bool> {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut all = self.load_relations()?;
        let before = all.len();
        all.retain(|r| r.id != id);
        if all.len() == before {
            return Ok(false);
        }
        self.store_relations(all)?;
        Ok(true)
    }

    /// Points every end that is `old` at `new`, for a renamed item. Returns how many ends changed.
    pub fn rewrite_endpoint(&self, old: &str, new: &str) -> Result<usize> {
        self.rewrite(|s| (s == old).then(|| new.to_string()))
    }

    /// Like [`Workspace::rewrite_endpoint`] for everything below a renamed folder (`old/…`).
    pub fn rewrite_prefix(&self, old: &str, new: &str) -> Result<usize> {
        let dir = format!("{}/", old.trim_end_matches('/'));
        self.rewrite(|s| s.strip_prefix(&dir).map(|rest| format!("{}/{rest}", new.trim_end_matches('/'))))
    }

    fn rewrite(&self, f: impl Fn(&str) -> Option<String>) -> Result<usize> {
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut all = self.load_relations()?;
        let mut changed = 0;
        for r in &mut all {
            for end in [&mut r.from, &mut r.to] {
                if let Some(new) = f(end) {
                    *end = new;
                    changed += 1;
                }
            }
        }
        if changed > 0 {
            self.store_relations(all)?;
        }
        Ok(changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ws() -> (tempfile::TempDir, Workspace) {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        (dir, ws)
    }

    #[test]
    fn add_list_remove() {
        let (_d, ws) = ws();
        assert!(ws.relations().unwrap().is_empty());
        let a = ws.add_relation("relates", "note:1", "note:2", Some("see"), None, "user").unwrap();
        let b = ws
            .add_relation(
                "derived-from",
                "file:workspace/b.png",
                "file:workspace/a.png",
                None,
                Some(json!({"p": 1})),
                "plugin:comfy",
            )
            .unwrap();
        assert!(a.id < b.id);
        let all = ws.relations().unwrap();
        assert_eq!(all, vec![a.clone(), b.clone()]);
        assert!(a.created.ends_with('Z') && a.created.len() == 20);

        assert!(ws.remove_relation(&a.id).unwrap());
        assert!(!ws.remove_relation(&a.id).unwrap());
        assert_eq!(ws.relations().unwrap(), vec![b]);
    }

    #[test]
    fn file_is_one_json_object_per_line_sorted_by_id() {
        let (_d, ws) = ws();
        for i in 0..5 {
            ws.add_relation("relates", &format!("note:{i}"), "note:x", None, None, "user").unwrap();
        }
        // Put a late line in the middle by hand; the next write sorts it.
        let text = std::fs::read_to_string(ws.relations_path()).unwrap();
        let mut lines: Vec<&str> = text.lines().collect();
        lines.reverse();
        std::fs::write(ws.relations_path(), lines.join("\n")).unwrap();
        ws.add_relation("relates", "note:new", "note:x", None, None, "user").unwrap();
        let text = std::fs::read_to_string(ws.relations_path()).unwrap();
        let ids: Vec<String> = text
            .lines()
            .map(|l| serde_json::from_str::<Value>(l).unwrap()["id"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(ids.len(), 6);
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
        assert!(text.ends_with('\n'));
        assert!(!text.contains("\"label\""));
    }

    #[test]
    fn rewrite_endpoint_and_prefix() {
        let (_d, ws) = ws();
        ws.add_relation("relates", "file:workspace/a.md", "file:workspace/dir/x.png", None, None, "user").unwrap();
        ws.add_relation("relates", "file:workspace/a.md", "note:1", None, None, "user").unwrap();
        assert_eq!(ws.rewrite_endpoint("file:workspace/a.md", "file:workspace/b.md").unwrap(), 2);
        assert_eq!(ws.rewrite_endpoint("file:workspace/zzz", "file:workspace/y").unwrap(), 0);
        assert_eq!(ws.rewrite_prefix("file:workspace/dir", "file:workspace/other").unwrap(), 1);
        let all = ws.relations().unwrap();
        assert_eq!(all[0].from, "file:workspace/b.md");
        assert_eq!(all[0].to, "file:workspace/other/x.png");
        assert_eq!(all[1].to, "note:1");
    }

    #[test]
    fn a_bad_line_is_an_error_not_silently_dropped() {
        let (_d, ws) = ws();
        std::fs::write(ws.relations_path(), "{not json}\n").unwrap();
        assert!(ws.relations().is_err());
        assert!(ws.add_relation("relates", "a", "b", None, None, "user").is_err());
    }
}
