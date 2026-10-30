//! The fields of an issue that sync both ways, each as one JSON value, so a merge, the outbox and
//! the base all speak the same language.
//!
//! | Field          | Value                                                           |
//! | -------------- | --------------------------------------------------------------- |
//! | `title`        | string                                                          |
//! | `body`         | string                                                          |
//! | `state`        | `{"state": "OPEN" \| "CLOSED", "reason": string \| null}`       |
//! | `labels`       | `[{id, name, color}]`, a set by `id`                            |
//! | `assignees`    | `[{id, login}]`, a set by `id`                                  |
//! | `blockedBy`    | `[node id]`, a set: the issues that block this one              |
//! | `parent`       | node id or null                                                 |
//! | `field:<id>`   | an issue field's value or null (see [`field_value`])            |

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::Result;

pub fn is_set(field: &str) -> bool {
    matches!(field, "labels" | "assignees" | "blockedBy")
}

/// What identifies an element of a set field.
fn key(element: &Value) -> String {
    match element {
        Value::String(s) => s.clone(),
        other => other.get("id").and_then(Value::as_str).unwrap_or_default().to_string(),
    }
}

/// A set field in its one canonical form: deduplicated and sorted by key, so equal sets compare equal.
pub fn normalize(field: &str, value: Value) -> Value {
    if !is_set(field) {
        return value;
    }
    let mut by_key = BTreeMap::new();
    for element in value.as_array().cloned().unwrap_or_default() {
        by_key.insert(key(&element), element);
    }
    Value::Array(by_key.into_values().collect())
}

/// Three-way merge of a set: what either side added is in, what either side removed is out.
pub fn merge_sets(base: &Value, local: &Value, remote: &Value) -> Value {
    let keys = |v: &Value| -> BTreeMap<String, Value> {
        v.as_array().into_iter().flatten().map(|e| (key(e), e.clone())).collect()
    };
    let (b, l, r) = (keys(base), keys(local), keys(remote));
    let mut out = BTreeMap::new();
    for (k, v) in r.iter().chain(l.iter()) {
        let keep = if b.contains_key(k) { l.contains_key(k) && r.contains_key(k) } else { true };
        if keep {
            out.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    Value::Array(out.into_values().collect())
}

/// An issue field's value as stored: `{kind: "single_select", optionId, name}`, `{kind: "text" |
/// "date", value}`, `{kind: "number", value}`.
pub fn field_value(kind: &str, value: Value) -> Value {
    let mut v = value;
    if let Some(obj) = v.as_object_mut() {
        obj.insert("kind".into(), json!(kind));
    }
    v
}

/// The mirror's current value of `field` for the issue.
pub fn read(conn: &Connection, node: &str, field: &str) -> Result<Value> {
    let column = |col: &str| -> Result<Option<String>> {
        Ok(conn
            .query_row(&format!("SELECT {col} FROM gh_issues WHERE node_id = ?1"), [node], |r| r.get(0))
            .optional()?
            .flatten())
    };
    Ok(match field {
        "title" | "body" => json!(column(field)?.unwrap_or_default()),
        "state" => {
            let (state, reason): (String, Option<String>) = conn
                .query_row("SELECT state, state_reason FROM gh_issues WHERE node_id = ?1", [node], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })
                .optional()?
                .unwrap_or(("OPEN".into(), None));
            json!({"state": state, "reason": reason})
        }
        "labels" | "assignees" => serde_json::from_str(&column(field)?.unwrap_or_else(|| "[]".into()))?,
        "blockedBy" => {
            let mut stmt = conn
                .prepare("SELECT from_node FROM gh_links WHERE kind = 'blocks' AND to_node = ?1 ORDER BY from_node")?;
            let ids = stmt.query_map([node], |r| r.get::<_, String>(0))?;
            Value::Array(ids.map(|id| id.map(Value::String)).collect::<rusqlite::Result<_>>()?)
        }
        "parent" => conn
            .query_row("SELECT from_node FROM gh_links WHERE kind = 'parent' AND to_node = ?1", [node], |r| {
                r.get::<_, String>(0)
            })
            .optional()?
            .map_or(Value::Null, Value::String),
        f => match f.strip_prefix("field:") {
            Some(id) => {
                let fields: Value = serde_json::from_str(&column("fields")?.unwrap_or_else(|| "{}".into()))?;
                fields.get(id).cloned().unwrap_or(Value::Null)
            }
            None => return Err(crate::Error::Other(format!("not a synced field: {f}"))),
        },
    })
}

/// Writes `field` into the mirror. The issue's row must exist.
pub fn write(conn: &Connection, node: &str, field: &str, value: &Value) -> Result<()> {
    match field {
        "title" | "body" => {
            conn.execute(
                &format!("UPDATE gh_issues SET {field} = ?2 WHERE node_id = ?1"),
                params![node, value.as_str().unwrap_or_default()],
            )?;
        }
        "state" => {
            conn.execute(
                "UPDATE gh_issues SET state = ?2, state_reason = ?3 WHERE node_id = ?1",
                params![
                    node,
                    value.get("state").and_then(Value::as_str).unwrap_or("OPEN"),
                    value.get("reason").and_then(Value::as_str)
                ],
            )?;
        }
        "labels" | "assignees" => {
            conn.execute(
                &format!("UPDATE gh_issues SET {field} = ?2 WHERE node_id = ?1"),
                params![node, normalize(field, value.clone()).to_string()],
            )?;
        }
        "blockedBy" => {
            conn.execute("DELETE FROM gh_links WHERE kind = 'blocks' AND to_node = ?1", [node])?;
            for id in value.as_array().into_iter().flatten().filter_map(Value::as_str) {
                conn.execute(
                    "INSERT OR IGNORE INTO gh_links (kind, from_node, to_node) VALUES ('blocks', ?1, ?2)",
                    params![id, node],
                )?;
            }
        }
        "parent" => {
            conn.execute("DELETE FROM gh_links WHERE kind = 'parent' AND to_node = ?1", [node])?;
            if let Some(parent) = value.as_str() {
                conn.execute(
                    "INSERT INTO gh_links (kind, from_node, to_node) VALUES ('parent', ?1, ?2)",
                    params![parent, node],
                )?;
            }
        }
        f => match f.strip_prefix("field:") {
            Some(id) => {
                let raw: String =
                    conn.query_row("SELECT fields FROM gh_issues WHERE node_id = ?1", [node], |r| r.get(0))?;
                let mut fields: serde_json::Map<String, Value> = serde_json::from_str(&raw)?;
                if value.is_null() {
                    fields.remove(id);
                } else {
                    fields.insert(id.to_string(), value.clone());
                }
                conn.execute(
                    "UPDATE gh_issues SET fields = ?2 WHERE node_id = ?1",
                    params![node, Value::Object(fields).to_string()],
                )?;
            }
            None => return Err(crate::Error::Other(format!("not a synced field: {f}"))),
        },
    }
    Ok(())
}

pub fn base(conn: &Connection, node: &str, field: &str) -> Result<Option<Value>> {
    let raw: Option<String> = conn
        .query_row("SELECT value FROM gh_base WHERE node_id = ?1 AND field = ?2", params![node, field], |r| r.get(0))
        .optional()?;
    Ok(raw.map(|s| serde_json::from_str(&s)).transpose()?)
}

pub fn set_base(conn: &Connection, node: &str, field: &str, value: &Value) -> Result<()> {
    conn.execute(
        "INSERT INTO gh_base (node_id, field, value) VALUES (?1, ?2, ?3)
         ON CONFLICT (node_id, field) DO UPDATE SET value = excluded.value",
        params![node, field, value.to_string()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_merge_both_sides() {
        let base = json!(["a", "b", "c"]);
        let local = json!(["a", "b", "d"]); // removed c, added d
        let remote = json!(["b", "c", "e"]); // removed a, added e
        assert_eq!(merge_sets(&base, &local, &remote), json!(["b", "d", "e"]));
    }

    #[test]
    fn normalize_sorts_and_dedups_by_id() {
        let v = json!([{"id": "L2", "name": "b"}, {"id": "L1", "name": "a"}, {"id": "L2", "name": "b"}]);
        assert_eq!(normalize("labels", v), json!([{"id": "L1", "name": "a"}, {"id": "L2", "name": "b"}]));
    }
}
