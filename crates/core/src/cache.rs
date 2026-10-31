use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;

use crate::Result;

const SCHEMA: &str = include_str!("schema.sql");
const SCHEMA_V2: &str = include_str!("schema_v2.sql");
const SCHEMA_V3: &str = include_str!("schema_v3.sql");
const VERSION: i64 = 3;

/// `cache.db`. One per workspace, owned by the window's Rust host.
pub struct Cache {
    pub(crate) conn: Connection,
}

impl Cache {
    pub fn open(path: &Path) -> Result<Cache> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        // The window, the sync thread and the file watcher each hold a connection; writes wait
        // for each other instead of failing.
        conn.busy_timeout(std::time::Duration::from_secs(10))?;
        Cache::init(conn)
    }

    pub fn open_in_memory() -> Result<Cache> {
        Cache::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Cache> {
        conn.pragma_update(None, "foreign_keys", true)?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if (0..VERSION).contains(&version) {
            // An older database keeps its data; only the tables of later versions are added.
            let tx = conn.unchecked_transaction()?;
            if version == 0 {
                tx.execute_batch(SCHEMA)?;
            }
            if version <= 1 {
                tx.execute_batch(SCHEMA_V2)?;
            }
            tx.execute_batch(SCHEMA_V3)?;
            tx.commit()?;
            conn.pragma_update(None, "user_version", VERSION)?;
        } else if version != VERSION {
            return Err(crate::Error::Other(format!("cache.db is version {version}, this build reads {VERSION}")));
        }
        Ok(Cache { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Edits waiting for GitHub, oldest first.
    pub fn outbox(&self) -> Result<Vec<OutboxEntry>> {
        let mut stmt = self
            .conn
            .prepare("SELECT seq, target, op, field, value, state, attempts, last_error FROM outbox ORDER BY seq")?;
        let rows = stmt.query_map([], OutboxEntry::from_row)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn conflicts(&self) -> Result<Vec<Conflict>> {
        let mut stmt =
            self.conn.prepare("SELECT target, field, base, local, remote FROM conflicts ORDER BY detected")?;
        let rows = stmt.query_map([], |r| {
            Ok(Conflict {
                target: r.get(0)?,
                field: r.get(1)?,
                base: r.get::<_, Option<String>>(2)?,
                local: r.get(3)?,
                remote: r.get(4)?,
            })
        })?;
        let raw: Vec<Conflict> = rows.collect::<rusqlite::Result<_>>()?;
        Ok(raw)
    }

    /// The repository's node id for `owner/name`, if it has been pulled.
    pub fn repo_id(&self, owner: &str, name: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT node_id FROM gh_repos WHERE owner = ?1 AND name = ?2", params![owner, name], |r| {
                r.get(0)
            })
            .optional()?)
    }

    /// The issue's node id by repository and number.
    pub fn issue_id(&self, repo: &str, number: i64) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT node_id FROM gh_issues WHERE repo = ?1 AND number = ?2", params![repo, number], |r| {
                r.get(0)
            })
            .optional()?)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutboxEntry {
    pub seq: i64,
    pub target: String,
    pub op: String,
    pub field: Option<String>,
    pub value: Value,
    pub state: String,
    pub attempts: i64,
    pub last_error: Option<String>,
}

impl OutboxEntry {
    fn from_row(r: &rusqlite::Row) -> rusqlite::Result<OutboxEntry> {
        let value: String = r.get(4)?;
        Ok(OutboxEntry {
            seq: r.get(0)?,
            target: r.get(1)?,
            op: r.get(2)?,
            field: r.get(3)?,
            value: serde_json::from_str(&value).unwrap_or(Value::Null),
            state: r.get(5)?,
            attempts: r.get(6)?,
            last_error: r.get(7)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Conflict {
    pub target: String,
    pub field: String,
    pub base: Option<String>,
    pub local: String,
    pub remote: String,
}

pub(crate) fn now() -> String {
    let secs =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    iso(secs)
}

/// RFC 3339 in UTC without a date library: seconds since the epoch, converted by hand.
pub fn iso(secs: i64) -> String {
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}
