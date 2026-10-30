//! Local edits. Each one changes the mirror and queues the change for GitHub in one transaction,
//! so the window shows it at once and nothing is lost before it is pushed.

use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

use crate::cache::now;
use crate::fields::{self, normalize};
use crate::{Cache, Result};

impl Cache {
    /// Sets a synced field of an issue (see [`crate::fields`]). Setting it back to what GitHub has
    /// drops the queued change instead of sending a no-op.
    pub fn set_field(&mut self, node: &str, field: &str, value: Value) -> Result<()> {
        let value = normalize(field, value);
        let tx = self.conn.transaction()?;
        if fields::read(&tx, node, field)? == value {
            return Ok(());
        }
        fields::write(&tx, node, field, &value)?;
        tx.execute("DELETE FROM outbox WHERE target = ?1 AND op = 'set' AND field = ?2", params![node, field])?;
        tx.execute("DELETE FROM conflicts WHERE target = ?1 AND field = ?2", params![node, field])?;
        let unchanged = fields::base(&tx, node, field)?.as_ref() == Some(&value);
        if !unchanged {
            tx.execute(
                "INSERT INTO outbox (created, target, op, field, value) VALUES (?1, ?2, 'set', ?3, ?4)",
                params![now(), node, field, value.to_string()],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// A new issue in the repository, shown at once under a `local:` id until GitHub gives it its own.
    pub fn create_issue(&mut self, repo: &str, title: &str, body: &str) -> Result<String> {
        let node = format!("local:{}", uuid::Uuid::now_v7());
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO gh_issues (node_id, repo, title, body, created, updated) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![node, repo, title, body, now()],
        )?;
        tx.execute(
            "INSERT INTO outbox (created, target, op, value) VALUES (?1, ?2, 'create', ?3)",
            params![now(), node, json!({"repo": repo, "title": title, "body": body}).to_string()],
        )?;
        tx.commit()?;
        Ok(node)
    }

    /// A comment on an issue, as you.
    pub fn add_comment(&mut self, issue: &str, body: &str) -> Result<String> {
        let id = format!("local:{}", uuid::Uuid::now_v7());
        let login: Option<String> =
            self.conn.query_row("SELECT value FROM meta WHERE key = 'login'", [], |r| r.get(0)).optional()?;
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO gh_comments (node_id, issue, author, body, created, updated) VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            params![id, issue, login, body, now()],
        )?;
        tx.execute(
            "INSERT INTO outbox (created, target, op, value) VALUES (?1, ?2, 'comment', ?3)",
            params![now(), issue, json!({"id": id, "body": body}).to_string()],
        )?;
        tx.execute("UPDATE gh_issues SET comment_count = comment_count + 1 WHERE node_id = ?1", [issue])?;
        tx.commit()?;
        Ok(id)
    }

    /// Settles a conflict: `keep_local` pushes your value over GitHub's, otherwise GitHub's wins.
    pub fn resolve(&mut self, node: &str, field: &str, keep_local: bool) -> Result<()> {
        let tx = self.conn.transaction()?;
        let remote: Option<String> = tx
            .query_row("SELECT remote FROM conflicts WHERE target = ?1 AND field = ?2", params![node, field], |r| {
                r.get(0)
            })
            .optional()?;
        let Some(remote) = remote else { return Ok(()) };
        let remote: Value = serde_json::from_str(&remote)?;
        fields::set_base(&tx, node, field, &remote)?;
        if keep_local {
            tx.execute(
                "UPDATE outbox SET state = 'waiting' WHERE target = ?1 AND op = 'set' AND field = ?2",
                params![node, field],
            )?;
        } else {
            fields::write(&tx, node, field, &remote)?;
            tx.execute("DELETE FROM outbox WHERE target = ?1 AND op = 'set' AND field = ?2", params![node, field])?;
        }
        tx.execute("DELETE FROM conflicts WHERE target = ?1 AND field = ?2", params![node, field])?;
        tx.commit()?;
        Ok(())
    }
}
