//! What the window draws, read from the cache as JSON: a repository's summary for the overview,
//! the blocks between repositories, a board's columns of cards, one issue in full.

use std::collections::BTreeMap;

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::Result;

/// The organization's single-select issue field named "Status", whose options are the columns.
#[derive(Debug, Clone)]
pub struct StatusField {
    pub id: String,
    pub options: Vec<Value>,
}

/// The repository's owner's issue fields, by id: `{name, kind, options}`.
pub fn field_defs(conn: &Connection, repo: &str) -> Result<BTreeMap<String, Value>> {
    let mut stmt = conn.prepare(
        "SELECT f.node_id, f.name, f.kind, f.options FROM gh_fields f JOIN gh_repos r ON r.owner = f.owner
         WHERE r.node_id = ?1",
    )?;
    let rows = stmt.query_map([repo], |r| {
        let options: String = r.get(3)?;
        Ok((
            r.get::<_, String>(0)?,
            json!({"id": r.get::<_, String>(0)?, "name": r.get::<_, String>(1)?, "kind": r.get::<_, String>(2)?,
                   "options": serde_json::from_str::<Value>(&options).unwrap_or(json!([]))}),
        ))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn status_field(conn: &Connection, repo: &str) -> Result<Option<StatusField>> {
    Ok(field_defs(conn, repo)?
        .into_values()
        .find(|f| f["kind"] == "single_select" && f["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case("status")))
        .map(|f| StatusField {
            id: f["id"].as_str().unwrap_or_default().to_string(),
            options: f["options"].as_array().cloned().unwrap_or_default(),
        }))
}

/// One issue as a card: what a board column, a canvas node or a search result shows.
pub fn card(conn: &Connection, node: &str) -> Result<Option<Value>> {
    let row = conn
        .query_row(
            "SELECT i.node_id, i.repo, r.owner || '/' || r.name, i.number, i.title, i.state, i.state_reason, i.labels,
                    i.assignees, i.fields, i.url, i.updated, i.comment_count
             FROM gh_issues i JOIN gh_repos r ON r.node_id = i.repo WHERE i.node_id = ?1",
            [node],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, String>(7)?,
                    r.get::<_, String>(8)?,
                    r.get::<_, String>(9)?,
                    r.get::<_, Option<String>>(10)?,
                    r.get::<_, Option<String>>(11)?,
                    r.get::<_, i64>(12)?,
                ))
            },
        )
        .optional()?;
    let Some((id, repo, repo_name, number, title, state, reason, labels, assignees, fields, url, updated, comments)) =
        row
    else {
        return Ok(None);
    };
    let defs = field_defs(conn, &repo)?;
    let fields: Value = serde_json::from_str(&fields)?;
    let mut named = serde_json::Map::new();
    for (field_id, value) in fields.as_object().into_iter().flatten() {
        if let Some(def) = defs.get(field_id) {
            let shown = value.get("name").or(value.get("value")).cloned().unwrap_or(Value::Null);
            named.insert(def["name"].as_str().unwrap_or_default().to_string(), shown);
        }
    }
    let labels: Value = serde_json::from_str(&labels)?;
    let assignees: Value = serde_json::from_str(&assignees)?;
    Ok(Some(json!({
        "node": id,
        "repo": repo,
        "repoName": repo_name,
        "number": number,
        "title": title,
        "state": state,
        "stateReason": reason,
        "labels": labels.as_array().into_iter().flatten().map(|l| json!({"name": l["name"], "color": l["color"]})).collect::<Vec<_>>(),
        "assignees": assignees.as_array().into_iter().flatten().map(|u| u["login"].clone()).collect::<Vec<_>>(),
        "fields": named,
        "blockedBy": open_blockers(conn, &id)?,
        "blocking": count(conn, "SELECT count(*) FROM gh_links l JOIN gh_issues t ON t.node_id = l.to_node
                                WHERE l.kind = 'blocks' AND l.from_node = ?1 AND t.state = 'OPEN'", &id)?,
        "comments": comments,
        "url": url,
        "updated": updated,
        "pending": count(conn, "SELECT count(*) FROM outbox WHERE target = ?1", &id)? > 0,
        "conflict": count(conn, "SELECT count(*) FROM conflicts WHERE target = ?1", &id)? > 0,
    })))
}

fn count(conn: &Connection, sql: &str, arg: &str) -> Result<i64> {
    Ok(conn.query_row(sql, [arg], |r| r.get(0))?)
}

/// How many of the issue's blockers are still open (mirrored or only referenced).
fn open_blockers(conn: &Connection, node: &str) -> Result<i64> {
    count(
        conn,
        "SELECT count(*) FROM gh_links l
           LEFT JOIN gh_issues i ON i.node_id = l.from_node
           LEFT JOIN gh_refs f ON f.node_id = l.from_node
         WHERE l.kind = 'blocks' AND l.to_node = ?1 AND coalesce(i.state, f.state, 'OPEN') = 'OPEN'",
        node,
    )
}

/// Ranks a Priority value for sorting cards: Urgent first, no priority last.
fn priority_rank(card: &Value) -> usize {
    match card["fields"]["Priority"].as_str() {
        Some("Urgent") => 0,
        Some("High") => 1,
        Some("Medium") => 2,
        Some("Low") => 3,
        _ => 4,
    }
}

fn cards_of(conn: &Connection, sql: &str, repo: &str) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(sql)?;
    let ids: Vec<String> = stmt.query_map([repo], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    let mut cards = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(card) = card(conn, &id)? {
            cards.push(card);
        }
    }
    Ok(cards)
}

/// How many closed issues a board shows in its last column: the most recent ones.
const CLOSED_SHOWN: usize = 30;

/// The board of a repository. With a Status field the columns are its options (open issues
/// without a status first, in "No status"; closed issues in "Done"); without one, Open and Closed.
pub fn board(conn: &Connection, repo: &str) -> Result<Value> {
    let open = cards_of(conn, "SELECT node_id FROM gh_issues WHERE repo = ?1 AND state = 'OPEN'", repo)?;
    let closed = cards_of(
        conn,
        "SELECT node_id FROM gh_issues WHERE repo = ?1 AND state = 'CLOSED' ORDER BY updated DESC LIMIT 30",
        repo,
    )?;
    let sort = |mut cards: Vec<Value>| {
        cards.sort_by_key(|c| (priority_rank(c), std::cmp::Reverse(c["number"].as_i64().unwrap_or(i64::MAX))));
        cards
    };
    let Some(status) = status_field(conn, repo)? else {
        return Ok(json!({"statusField": null, "columns": [
            {"key": "open", "name": "Open", "color": "GREEN", "cards": sort(open)},
            {"key": "closed", "name": "Closed", "color": "PURPLE", "cards": closed},
        ]}));
    };
    let done = status
        .options
        .iter()
        .rposition(|o| o["name"].as_str().is_some_and(|n| n.eq_ignore_ascii_case("done")))
        .unwrap_or(status.options.len().saturating_sub(1));
    let mut columns: Vec<(Value, Vec<Value>)> = status.options.iter().map(|o| (o.clone(), Vec::new())).collect();
    let mut none = Vec::new();
    let status_of = |card: &Value| card["fields"]["Status"].as_str().map(str::to_string);
    for card in open {
        let at = status_of(&card).and_then(|s| status.options.iter().position(|o| o["name"] == s.as_str()));
        match at {
            Some(i) => columns[i].1.push(card),
            None => none.push(card),
        }
    }
    if let Some(column) = columns.get_mut(done) {
        column.1.extend(closed.into_iter().take(CLOSED_SHOWN));
    }
    let mut out = Vec::new();
    if !none.is_empty() {
        out.push(json!({"key": "none", "name": "No status", "color": "GRAY", "cards": sort(none)}));
    }
    for (i, (option, cards)) in columns.into_iter().enumerate() {
        let cards = if i == done { cards } else { sort(cards) };
        out.push(json!({"key": option["id"], "name": option["name"], "color": option["color"], "done": i == done, "cards": cards}));
    }
    Ok(json!({"statusField": status.id, "columns": out}))
}

/// A repository on the overview: counts, the Status split and what is in progress.
pub fn repo_summary(conn: &Connection, repo: &str) -> Result<Value> {
    let open = count(conn, "SELECT count(*) FROM gh_issues WHERE repo = ?1 AND state = 'OPEN'", repo)?;
    let closed = count(conn, "SELECT count(*) FROM gh_issues WHERE repo = ?1 AND state = 'CLOSED'", repo)?;
    let open_cards = cards_of(conn, "SELECT node_id FROM gh_issues WHERE repo = ?1 AND state = 'OPEN'", repo)?;
    let blocked = open_cards.iter().filter(|c| c["blockedBy"].as_i64().unwrap_or(0) > 0).count();
    let status = status_field(conn, repo)?;
    let split: Vec<Value> = match &status {
        Some(status) => {
            let mut split: Vec<Value> = status
                .options
                .iter()
                .map(|o| {
                    let name = o["name"].as_str().unwrap_or_default();
                    let n = open_cards.iter().filter(|c| c["fields"]["Status"] == name).count();
                    json!({"name": name, "color": o["color"], "count": n})
                })
                .collect();
            let none = open_cards.iter().filter(|c| c["fields"]["Status"].is_null()).count();
            if none > 0 {
                split.insert(0, json!({"name": "No status", "color": "GRAY", "count": none}));
            }
            split
        }
        None => vec![json!({"name": "Open", "color": "GREEN", "count": open})],
    };
    let mut in_progress: Vec<&Value> = open_cards
        .iter()
        .filter(|c| c["fields"]["Status"].as_str().is_some_and(|s| s.eq_ignore_ascii_case("in progress")))
        .collect();
    in_progress.sort_by(|a, b| b["updated"].as_str().cmp(&a["updated"].as_str()));
    let pulled: Option<String> =
        conn.query_row("SELECT pulled_at FROM gh_repos WHERE node_id = ?1", [repo], |r| r.get(0)).optional()?.flatten();
    Ok(json!({
        "open": open,
        "closed": closed,
        "blocked": blocked,
        "hasStatus": status.is_some(),
        "split": split,
        "inProgress": in_progress.into_iter().take(3).map(|c| json!({"node": c["node"], "number": c["number"], "title": c["title"]})).collect::<Vec<_>>(),
        "pulledAt": pulled,
    }))
}

/// Open issues blocked by open issues of another repository, counted per pair of repositories:
/// the arrows between projects on the overview.
pub fn cross_repo_blocks(conn: &Connection) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(
        "SELECT b.repo, t.repo, count(*) FROM gh_links l
           JOIN gh_issues b ON b.node_id = l.from_node
           JOIN gh_issues t ON t.node_id = l.to_node
         WHERE l.kind = 'blocks' AND b.repo != t.repo AND b.state = 'OPEN' AND t.state = 'OPEN'
         GROUP BY b.repo, t.repo",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(json!({"from": r.get::<_, String>(0)?, "to": r.get::<_, String>(1)?, "count": r.get::<_, i64>(2)?}))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// A linked issue, mirrored or only referenced: enough to list it.
fn linked(conn: &Connection, node: &str) -> Result<Value> {
    if let Some(card) = card(conn, node)? {
        return Ok(card);
    }
    Ok(conn
        .query_row(
            "SELECT repo, number, title, state, url FROM gh_refs WHERE node_id = ?1",
            [node],
            |r| {
                Ok(json!({"node": node, "repoName": r.get::<_, String>(0)?, "number": r.get::<_, i64>(1)?,
                          "title": r.get::<_, String>(2)?, "state": r.get::<_, String>(3)?, "url": r.get::<_, String>(4)?}))
            },
        )
        .optional()?
        .unwrap_or_else(|| json!({"node": node, "title": "(not mirrored)", "state": "OPEN"})))
}

fn links(conn: &Connection, sql: &str, node: &str) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(sql)?;
    let ids: Vec<String> = stmt.query_map([node], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    ids.iter().map(|id| linked(conn, id)).collect()
}

/// One issue with everything its panel shows and the choices its editors offer.
#[allow(clippy::type_complexity)] // one row of eight columns, read once
pub fn issue(conn: &Connection, node: &str) -> Result<Option<Value>> {
    let Some(mut detail) = card(conn, node)? else { return Ok(None) };
    let (body, author, created, milestone, issue_type, labels, assignees, fields): (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        String,
        String,
    ) = conn.query_row(
        "SELECT body, author, created, milestone, issue_type, labels, assignees, fields FROM gh_issues WHERE node_id = ?1",
        [node],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?)),
    )?;
    let repo = detail["repo"].as_str().unwrap_or_default().to_string();
    let field_values: Value = serde_json::from_str(&fields)?;
    let defs: Vec<Value> = field_defs(conn, &repo)?
        .into_values()
        .map(|mut def| {
            def["value"] = field_values.get(def["id"].as_str().unwrap_or_default()).cloned().unwrap_or(Value::Null);
            def
        })
        .collect();
    let mut stmt = conn.prepare("SELECT node_id, name, color FROM gh_labels WHERE repo = ?1 ORDER BY name")?;
    let repo_labels: Vec<Value> = stmt
        .query_map([&repo], |r| {
            Ok(json!({"id": r.get::<_, String>(0)?, "name": r.get::<_, String>(1)?, "color": r.get::<_, Option<String>>(2)?}))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut stmt = conn.prepare(
        "SELECT u.node_id, u.login FROM gh_repo_users ru JOIN gh_users u ON u.node_id = ru.user WHERE ru.repo = ?1 ORDER BY u.login",
    )?;
    let users: Vec<Value> = stmt
        .query_map([&repo], |r| Ok(json!({"id": r.get::<_, String>(0)?, "login": r.get::<_, String>(1)?})))?
        .collect::<rusqlite::Result<_>>()?;
    let mut stmt =
        conn.prepare("SELECT node_id, author, body, created FROM gh_comments WHERE issue = ?1 ORDER BY created")?;
    let comments: Vec<Value> = stmt
        .query_map([node], |r| {
            let id: String = r.get(0)?;
            Ok(json!({"id": id, "author": r.get::<_, Option<String>>(1)?, "body": r.get::<_, String>(2)?,
                      "created": r.get::<_, Option<String>>(3)?, "pending": id.starts_with("local:")}))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut stmt = conn.prepare("SELECT field, local, remote FROM conflicts WHERE target = ?1")?;
    let conflicts: Vec<Value> = stmt
        .query_map([node], |r| {
            Ok(json!({"field": r.get::<_, String>(0)?, "local": r.get::<_, String>(1)?, "remote": r.get::<_, String>(2)?}))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let parent: Option<String> = conn
        .query_row("SELECT from_node FROM gh_links WHERE kind = 'parent' AND to_node = ?1", [node], |r| r.get(0))
        .optional()?;
    detail["body"] = json!(body);
    detail["author"] = json!(author);
    detail["created"] = json!(created);
    detail["milestone"] = json!(milestone);
    detail["issueType"] = json!(issue_type);
    detail["labelsFull"] = serde_json::from_str(&labels)?;
    detail["assigneesFull"] = serde_json::from_str(&assignees)?;
    detail["fieldDefs"] = json!(defs);
    detail["repoLabels"] = json!(repo_labels);
    detail["repoUsers"] = json!(users);
    detail["blockedByList"] =
        json!(links(conn, "SELECT from_node FROM gh_links WHERE kind = 'blocks' AND to_node = ?1", node)?);
    detail["blockingList"] =
        json!(links(conn, "SELECT to_node FROM gh_links WHERE kind = 'blocks' AND from_node = ?1", node)?);
    detail["parent"] = match parent {
        Some(p) => linked(conn, &p)?,
        None => Value::Null,
    };
    detail["subIssues"] =
        json!(links(conn, "SELECT to_node FROM gh_links WHERE kind = 'parent' AND from_node = ?1", node)?);
    detail["commentList"] = json!(comments);
    detail["conflicts"] = json!(conflicts);
    detail["statusField"] = json!(status_field(conn, &repo)?.map(|s| s.id));
    Ok(Some(detail))
}

/// Issues whose title contains `query`, or whose number it is (`#12` or `12`), in every mirrored
/// repository: for linking one issue to another.
pub fn search(conn: &Connection, query: &str, limit: usize) -> Result<Vec<Value>> {
    let q = query.trim().trim_start_matches('#');
    let number: Option<i64> = q.parse().ok();
    let mut stmt = conn.prepare(
        "SELECT node_id FROM gh_issues
         WHERE (?1 IS NOT NULL AND number = ?1) OR title LIKE '%' || ?2 || '%'
         ORDER BY state DESC, updated DESC LIMIT ?3",
    )?;
    let ids: Vec<String> =
        stmt.query_map(params![number, q, limit as i64], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
    let mut out = Vec::new();
    for id in ids {
        if let Some(card) = card(conn, &id)? {
            out.push(card);
        }
    }
    Ok(out)
}
