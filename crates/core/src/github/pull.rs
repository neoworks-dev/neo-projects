//! Bringing GitHub's changes into the cache, merged with the edits still in the outbox.

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use super::{Client, Transport};
use crate::cache::now;
use crate::fields::{self, field_value, is_set, merge_sets, normalize};
use crate::{Cache, Error, Result};

const META: &str = include_str!("meta.graphql");
const ISSUES: &str = include_str!("issues.graphql");
const COMMENTS: &str = include_str!("comments.graphql");

#[derive(Debug, Default, PartialEq)]
pub struct PullReport {
    /// Issues GitHub changed since the last pull (all of them on the first).
    pub issues: usize,
    /// Fields both sides changed, now waiting in `conflicts`.
    pub conflicts: usize,
}

fn s(v: &Value) -> Option<&str> {
    v.as_str()
}

/// Pulls one repository: its labels, assignable users and issue fields, then every issue
/// updated since the last pull. Each page of issues is its own transaction, so an interrupted
/// pull keeps what it got.
pub fn pull<T: Transport>(cache: &mut Cache, client: &Client<T>, owner: &str, name: &str) -> Result<PullReport> {
    let meta = client.run(META, json!({"owner": owner, "name": name}))?;
    let repo = &meta["repository"];
    let repo_id = s(&repo["id"]).ok_or_else(|| Error::Other(format!("{owner}/{name} not found")))?.to_string();
    store_meta(cache, &meta, &repo_id, owner, name)?;

    let since: Option<String> = cache
        .conn
        .query_row("SELECT pulled_since FROM gh_repos WHERE node_id = ?1", [&repo_id], |r| r.get(0))
        .optional()?
        .flatten();
    let mut report = PullReport::default();
    let mut newest = since.clone();
    let mut after: Option<String> = None;
    loop {
        let data = client.run(ISSUES, json!({"owner": owner, "name": name, "since": since, "after": after}))?;
        let issues = &data["repository"]["issues"];
        let tx = cache.conn.transaction()?;
        for issue in issues["nodes"].as_array().into_iter().flatten() {
            report.conflicts += apply_issue(&tx, &repo_id, issue)?;
            report.issues += 1;
            if let Some(updated) = s(&issue["updatedAt"])
                && newest.as_deref().is_none_or(|n| updated > n)
            {
                newest = Some(updated.to_string());
            }
        }
        tx.execute(
            "UPDATE gh_repos SET pulled_since = ?2, pulled_at = ?3 WHERE node_id = ?1",
            params![repo_id, newest, now()],
        )?;
        tx.commit()?;
        let page = &issues["pageInfo"];
        if page["hasNextPage"].as_bool() != Some(true) {
            break;
        }
        after = s(&page["endCursor"]).map(str::to_string);
    }
    Ok(report)
}

fn store_meta(cache: &mut Cache, meta: &Value, repo_id: &str, owner: &str, name: &str) -> Result<()> {
    let repo = &meta["repository"];
    let tx = cache.conn.transaction()?;
    if let Some(login) = s(&meta["viewer"]["login"]) {
        tx.execute(
            "INSERT INTO meta (key, value) VALUES ('login', ?1) ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            [login],
        )?;
    }
    tx.execute(
        "INSERT INTO gh_repos (node_id, owner, name, url, owner_kind, archived) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (node_id) DO UPDATE SET owner = excluded.owner, name = excluded.name, url = excluded.url,
           owner_kind = excluded.owner_kind, archived = excluded.archived",
        params![
            repo_id,
            owner,
            name,
            s(&repo["url"]).unwrap_or_default(),
            s(&repo["owner"]["__typename"]).unwrap_or("User"),
            repo["isArchived"].as_bool().unwrap_or(false)
        ],
    )?;
    tx.execute("DELETE FROM gh_labels WHERE repo = ?1", [repo_id])?;
    for label in repo["labels"]["nodes"].as_array().into_iter().flatten() {
        tx.execute(
            "INSERT OR REPLACE INTO gh_labels (node_id, repo, name, color) VALUES (?1, ?2, ?3, ?4)",
            params![s(&label["id"]), repo_id, s(&label["name"]), s(&label["color"])],
        )?;
    }
    tx.execute("DELETE FROM gh_repo_users WHERE repo = ?1", [repo_id])?;
    for user in repo["assignableUsers"]["nodes"].as_array().into_iter().flatten() {
        tx.execute(
            "INSERT OR REPLACE INTO gh_users (node_id, login) VALUES (?1, ?2)",
            params![s(&user["id"]), s(&user["login"])],
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO gh_repo_users (repo, user) VALUES (?1, ?2)",
            params![repo_id, s(&user["id"])],
        )?;
    }
    if let Some(defs) = repo["owner"]["issueFields"]["nodes"].as_array() {
        let org = s(&repo["owner"]["login"]).unwrap_or(owner);
        tx.execute("DELETE FROM gh_fields WHERE owner = ?1", [org])?;
        for def in defs {
            let Some(id) = s(&def["id"]) else { continue };
            tx.execute(
                "INSERT OR REPLACE INTO gh_fields (node_id, owner, name, kind, options) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    id,
                    org,
                    s(&def["name"]),
                    field_kind(s(&def["__typename"]).unwrap_or_default()),
                    def["options"].as_array().map_or("[]".into(), |o| Value::Array(o.clone()).to_string())
                ],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// `IssueFieldSingleSelect` → `single_select`, and the values' `IssueFieldSingleSelectValue` too.
fn field_kind(typename: &str) -> &'static str {
    let t = typename.trim_start_matches("IssueField").trim_end_matches("Value");
    match t {
        "SingleSelect" => "single_select",
        "MultiSelect" => "multi_select",
        "Text" => "text",
        "Date" => "date",
        "Number" => "number",
        _ => "unknown",
    }
}

/// Writes one issue from GitHub into the cache. Returns the number of new conflicts.
fn apply_issue(tx: &Connection, repo_id: &str, issue: &Value) -> Result<usize> {
    let node = s(&issue["id"]).ok_or_else(|| Error::Other("issue without an id".into()))?;
    tx.execute(
        "INSERT INTO gh_issues (node_id, repo, number, url, author, issue_type, milestone, comment_count, created, updated)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT (node_id) DO UPDATE SET repo = excluded.repo, number = excluded.number, url = excluded.url,
           author = excluded.author, issue_type = excluded.issue_type, milestone = excluded.milestone,
           comment_count = excluded.comment_count, created = excluded.created, updated = excluded.updated",
        params![
            node,
            repo_id,
            issue["number"].as_i64(),
            s(&issue["url"]),
            s(&issue["author"]["login"]),
            s(&issue["issueType"]["name"]),
            s(&issue["milestone"]["title"]),
            issue["comments"]["totalCount"].as_i64().unwrap_or(0),
            s(&issue["createdAt"]),
            s(&issue["updatedAt"]),
        ],
    )?;
    let linked = issue["blockedBy"]["nodes"].as_array().into_iter().flatten().chain(Some(&issue["parent"]));
    for other in linked.filter(|o| o.is_object()) {
        tx.execute(
            "INSERT OR REPLACE INTO gh_refs (node_id, repo, number, title, state, url) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                s(&other["id"]),
                s(&other["repository"]["nameWithOwner"]),
                other["number"].as_i64(),
                s(&other["title"]),
                s(&other["state"]),
                s(&other["url"])
            ],
        )?;
    }
    let mut conflicts = 0;
    for (field, remote) in remote_fields(tx, node, issue)? {
        conflicts += merge(tx, node, &field, remote)?;
    }
    Ok(conflicts)
}

/// Every synced field of the issue as GitHub has it, in the forms of [`crate::fields`].
fn remote_fields(tx: &Connection, node: &str, issue: &Value) -> Result<Vec<(String, Value)>> {
    let list = |path: &Value, pick: &dyn Fn(&Value) -> Value| -> Value {
        Value::Array(path["nodes"].as_array().into_iter().flatten().map(pick).collect())
    };
    let mut out = vec![
        ("title".to_string(), json!(s(&issue["title"]).unwrap_or_default())),
        ("body".to_string(), json!(s(&issue["body"]).unwrap_or_default())),
        ("state".to_string(), json!({"state": s(&issue["state"]).unwrap_or("OPEN"), "reason": issue["stateReason"]})),
        (
            "labels".to_string(),
            list(&issue["labels"], &|l| json!({"id": l["id"], "name": l["name"], "color": l["color"]})),
        ),
        ("assignees".to_string(), list(&issue["assignees"], &|u| json!({"id": u["id"], "login": u["login"]}))),
        ("blockedBy".to_string(), list(&issue["blockedBy"], &|b| b["id"].clone())),
        ("parent".to_string(), issue["parent"]["id"].clone()),
    ];
    let mut seen = std::collections::BTreeSet::new();
    for value in issue["issueFieldValues"]["nodes"].as_array().into_iter().flatten() {
        let Some(id) = s(&value["field"]["id"]) else { continue };
        let kind = field_kind(s(&value["__typename"]).unwrap_or_default());
        let v = match kind {
            "single_select" => json!({"optionId": value["optionId"], "name": value["name"]}),
            _ => json!({"value": value["value"]}),
        };
        seen.insert(id.to_string());
        out.push((format!("field:{id}"), field_value(kind, v)));
    }
    // A field GitHub no longer has a value for, but the mirror or the base still does, was cleared.
    let mut known: Vec<String> = Vec::new();
    let fields_json: Option<String> =
        tx.query_row("SELECT fields FROM gh_issues WHERE node_id = ?1", [node], |r| r.get(0)).optional()?;
    if let Some(Value::Object(map)) = fields_json.map(|f| serde_json::from_str(&f)).transpose()? {
        known.extend(map.keys().cloned());
    }
    let mut stmt = tx.prepare("SELECT substr(field, 7) FROM gh_base WHERE node_id = ?1 AND field LIKE 'field:%'")?;
    known.extend(stmt.query_map([node], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?);
    for id in known {
        if seen.insert(id.clone()) {
            out.push((format!("field:{id}"), Value::Null));
        }
    }
    Ok(out
        .into_iter()
        .map(|(f, v)| {
            let v = normalize(&f, v);
            (f, v)
        })
        .collect())
}

/// Merges GitHub's value of one field with the mirror. Returns 1 when it made a conflict.
fn merge(tx: &Connection, node: &str, field: &str, remote: Value) -> Result<usize> {
    let pending: Option<(i64, String)> = tx
        .query_row(
            "SELECT seq, value FROM outbox WHERE target = ?1 AND op = 'set' AND field = ?2",
            params![node, field],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((seq, local)) = pending else {
        fields::write(tx, node, field, &remote)?;
        fields::set_base(tx, node, field, &remote)?;
        return Ok(0);
    };
    let local: Value = serde_json::from_str(&local)?;
    let base = fields::base(tx, node, field)?;
    if base.as_ref() == Some(&remote) {
        return Ok(0); // GitHub didn't change it: the local edit stands and still goes out.
    }
    if local == remote {
        tx.execute("DELETE FROM outbox WHERE seq = ?1", [seq])?;
        tx.execute("DELETE FROM conflicts WHERE target = ?1 AND field = ?2", params![node, field])?;
        fields::set_base(tx, node, field, &remote)?;
        return Ok(0);
    }
    if is_set(field) {
        let merged = merge_sets(&base.unwrap_or(json!([])), &local, &remote);
        fields::write(tx, node, field, &merged)?;
        fields::set_base(tx, node, field, &remote)?;
        if merged == remote {
            tx.execute("DELETE FROM outbox WHERE seq = ?1", [seq])?;
        } else {
            tx.execute("UPDATE outbox SET value = ?2 WHERE seq = ?1", params![seq, merged.to_string()])?;
        }
        return Ok(0);
    }
    tx.execute(
        "INSERT INTO conflicts (target, field, base, local, remote, detected) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (target, field) DO UPDATE SET remote = excluded.remote, detected = excluded.detected",
        params![node, field, base.map(|b| b.to_string()), local.to_string(), remote.to_string(), now()],
    )?;
    tx.execute("UPDATE outbox SET state = 'conflict' WHERE seq = ?1", [seq])?;
    Ok(1)
}

/// Pulls an issue's comments (the last 100), keeping yours that haven't been sent yet.
pub fn pull_comments<T: Transport>(cache: &mut Cache, client: &Client<T>, issue: &str) -> Result<usize> {
    let data = client.run(COMMENTS, json!({"id": issue}))?;
    let comments = data["node"]["comments"]["nodes"].as_array().cloned().unwrap_or_default();
    let tx = cache.conn.transaction()?;
    tx.execute("DELETE FROM gh_comments WHERE issue = ?1 AND node_id NOT LIKE 'local:%'", [issue])?;
    for c in &comments {
        tx.execute(
            "INSERT OR REPLACE INTO gh_comments (node_id, issue, author, body, created, updated) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![s(&c["id"]), issue, s(&c["author"]["login"]), s(&c["body"]), s(&c["createdAt"]), s(&c["updatedAt"])],
        )?;
    }
    tx.commit()?;
    Ok(comments.len())
}
