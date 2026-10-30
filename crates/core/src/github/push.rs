//! Sending the outbox to GitHub, oldest first, as you.

use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use super::{Client, Transport};
use crate::fields;
use crate::{Cache, Error, Result};

#[derive(Debug, Default, PartialEq)]
pub struct PushReport {
    pub sent: usize,
    pub failed: usize,
    /// Left for later: conflicts, and edits to issues that don't exist on GitHub yet.
    pub waiting: usize,
}

/// Sends every entry that can go now. A failed entry stays with its error and is tried again on the
/// next push; when the network itself is gone the push stops there.
pub fn push<T: Transport>(cache: &mut Cache, client: &Client<T>) -> Result<PushReport> {
    let mut report = PushReport::default();
    let mut cursor = 0;
    loop {
        // Read one at a time: sending a `create` renames its issue in the entries behind it.
        let next: Option<(i64, String, String, Option<String>, String, String)> = cache
            .conn
            .query_row(
                "SELECT seq, target, op, field, value, state FROM outbox WHERE seq > ?1 ORDER BY seq LIMIT 1",
                [cursor],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .optional()?;
        let Some((seq, target, op, field, value, state)) = next else { break };
        cursor = seq;
        let value: Value = serde_json::from_str(&value)?;
        let blocked = state == "conflict"
            || (op != "create" && target.starts_with("local:"))
            || (matches!(field.as_deref(), Some("blockedBy" | "parent")) && mentions_local(&value));
        if blocked {
            report.waiting += 1;
            continue;
        }
        let sent = match op.as_str() {
            "create" => send_create(cache, client, &target, &value),
            "comment" => send_comment(cache, client, &target, &value),
            "set" => send_set(cache, client, &target, field.as_deref().unwrap_or_default(), &value),
            other => Err(Error::Other(format!("unknown outbox op {other}"))),
        };
        match sent {
            Ok(()) => {
                cache.conn.execute("DELETE FROM outbox WHERE seq = ?1", [seq])?;
                report.sent += 1;
            }
            Err(e) => {
                cache.conn.execute(
                    "UPDATE outbox SET state = 'failed', attempts = attempts + 1, last_error = ?2 WHERE seq = ?1",
                    params![seq, e.to_string()],
                )?;
                report.failed += 1;
                if matches!(e, Error::Http(_)) {
                    break;
                }
            }
        }
    }
    Ok(report)
}

fn mentions_local(value: &Value) -> bool {
    match value {
        Value::String(s) => s.starts_with("local:"),
        Value::Array(items) => items.iter().any(mentions_local),
        _ => false,
    }
}

/// Runs `mutation { name(input: $input) { selection } }` and returns its result object.
fn mutate<T: Transport>(
    client: &Client<T>,
    name: &str,
    input_type: &str,
    input: Value,
    selection: &str,
) -> Result<Value> {
    let query = format!(
        "mutation {upper}($input: {input_type}!) {{ {name}(input: $input) {{ {selection} }} }}",
        upper = name[..1].to_uppercase() + &name[1..]
    );
    let data = client.run(&query, json!({"input": input}))?;
    Ok(data[name].clone())
}

fn send_set<T: Transport>(cache: &mut Cache, client: &Client<T>, node: &str, field: &str, value: &Value) -> Result<()> {
    let ids = |v: &Value| -> Vec<Value> { v.as_array().into_iter().flatten().map(|e| e["id"].clone()).collect() };
    match field {
        "title" | "body" => {
            let mut input = json!({"id": node});
            input[field] = value.clone();
            mutate(client, "updateIssue", "UpdateIssueInput", input, "clientMutationId")?;
        }
        "labels" => {
            mutate(
                client,
                "updateIssue",
                "UpdateIssueInput",
                json!({"id": node, "labelIds": ids(value)}),
                "clientMutationId",
            )?;
        }
        "assignees" => {
            let input = json!({"id": node, "assigneeIds": ids(value)});
            mutate(client, "updateIssue", "UpdateIssueInput", input, "clientMutationId")?;
        }
        "state" => {
            if value["state"] == "CLOSED" {
                let reason = value["reason"].as_str().unwrap_or("COMPLETED");
                let input = json!({"issueId": node, "stateReason": reason});
                mutate(client, "closeIssue", "CloseIssueInput", input, "clientMutationId")?;
            } else {
                mutate(client, "reopenIssue", "ReopenIssueInput", json!({"issueId": node}), "clientMutationId")?;
            }
        }
        "blockedBy" => return send_blocked_by(cache, client, node, value),
        "parent" => {
            let old = fields::base(&cache.conn, node, "parent")?.unwrap_or(Value::Null);
            match value.as_str() {
                Some(parent) => {
                    let input = json!({"issueId": parent, "subIssueId": node, "replaceParent": true});
                    mutate(client, "addSubIssue", "AddSubIssueInput", input, "clientMutationId")?;
                }
                None => {
                    if let Some(old) = old.as_str() {
                        let input = json!({"issueId": old, "subIssueId": node});
                        mutate(client, "removeSubIssue", "RemoveSubIssueInput", input, "clientMutationId")?;
                    }
                }
            }
        }
        f => {
            let Some(id) = f.strip_prefix("field:") else {
                return Err(Error::Other(format!("not a synced field: {f}")));
            };
            let mut update = json!({"fieldId": id});
            match value["kind"].as_str() {
                _ if value.is_null() => update["delete"] = json!(true),
                Some("single_select") => update["singleSelectOptionId"] = value["optionId"].clone(),
                Some("text") => update["textValue"] = value["value"].clone(),
                Some("date") => update["dateValue"] = value["value"].clone(),
                Some("number") => update["numberValue"] = value["value"].clone(),
                other => return Err(Error::Other(format!("can't set an issue field of kind {other:?}"))),
            }
            let input = json!({"issueId": node, "issueFields": [update]});
            mutate(client, "setIssueFieldValue", "SetIssueFieldValueInput", input, "clientMutationId")?;
        }
    }
    fields::set_base(&cache.conn, node, field, value)
}

/// Blocked-by is a set GitHub changes one link at a time; the base follows each step, so a retry
/// after a failure in the middle doesn't send the first ones again.
fn send_blocked_by<T: Transport>(cache: &mut Cache, client: &Client<T>, node: &str, value: &Value) -> Result<()> {
    let strings = |v: &Value| -> Vec<String> {
        v.as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect()
    };
    let wanted = strings(value);
    let mut now = strings(&fields::base(&cache.conn, node, "blockedBy")?.unwrap_or(json!([])));
    for id in wanted.iter().filter(|id| !now.contains(id)).cloned().collect::<Vec<_>>() {
        let input = json!({"issueId": node, "blockingIssueId": id});
        mutate(client, "addBlockedBy", "AddBlockedByInput", input, "clientMutationId")?;
        now.push(id);
        fields::set_base(&cache.conn, node, "blockedBy", &fields::normalize("blockedBy", json!(now)))?;
    }
    for id in now.clone().into_iter().filter(|id| !wanted.contains(id)) {
        let input = json!({"issueId": node, "blockingIssueId": id});
        mutate(client, "removeBlockedBy", "RemoveBlockedByInput", input, "clientMutationId")?;
        now.retain(|n| n != &id);
        fields::set_base(&cache.conn, node, "blockedBy", &fields::normalize("blockedBy", json!(now)))?;
    }
    Ok(())
}

fn send_create<T: Transport>(cache: &mut Cache, client: &Client<T>, local: &str, value: &Value) -> Result<()> {
    let input = json!({"repositoryId": value["repo"], "title": value["title"], "body": value["body"]});
    let created =
        mutate(client, "createIssue", "CreateIssueInput", input, "issue { id number url createdAt updatedAt }")?;
    let issue = &created["issue"];
    let id = issue["id"].as_str().ok_or_else(|| Error::Other("GitHub returned no issue id".into()))?;
    let tx = cache.conn.transaction()?;
    rename(&tx, local, id)?;
    tx.execute(
        "UPDATE gh_issues SET number = ?2, url = ?3, created = ?4, updated = ?5 WHERE node_id = ?1",
        params![
            id,
            issue["number"].as_i64(),
            issue["url"].as_str(),
            issue["createdAt"].as_str(),
            issue["updatedAt"].as_str()
        ],
    )?;
    // What GitHub has now. Edits queued after the create are sent against this.
    for (field, v) in [
        ("title", value["title"].clone()),
        ("body", value["body"].clone()),
        ("state", json!({"state": "OPEN", "reason": null})),
        ("labels", json!([])),
        ("assignees", json!([])),
        ("blockedBy", json!([])),
        ("parent", Value::Null),
    ] {
        fields::set_base(&tx, id, field, &v)?;
    }
    tx.commit()?;
    Ok(())
}

/// Gives a `local:` issue its GitHub id everywhere the cache names it.
fn rename(tx: &Connection, old: &str, new: &str) -> Result<()> {
    tx.execute("UPDATE gh_issues SET node_id = ?2 WHERE node_id = ?1", params![old, new])?;
    tx.execute("UPDATE gh_links SET from_node = ?2 WHERE from_node = ?1", params![old, new])?;
    tx.execute("UPDATE gh_links SET to_node = ?2 WHERE to_node = ?1", params![old, new])?;
    tx.execute("UPDATE gh_comments SET issue = ?2 WHERE issue = ?1", params![old, new])?;
    tx.execute("UPDATE gh_base SET node_id = ?2 WHERE node_id = ?1", params![old, new])?;
    tx.execute("UPDATE outbox SET target = ?2 WHERE target = ?1", params![old, new])?;
    let quoted = (json!(old).to_string(), json!(new).to_string());
    tx.execute(
        "UPDATE outbox SET value = replace(value, ?1, ?2) WHERE op = 'set' AND field IN ('blockedBy', 'parent')",
        params![quoted.0, quoted.1],
    )?;
    Ok(())
}

fn send_comment<T: Transport>(cache: &mut Cache, client: &Client<T>, issue: &str, value: &Value) -> Result<()> {
    let input = json!({"subjectId": issue, "body": value["body"]});
    let added = mutate(client, "addComment", "AddCommentInput", input, "commentEdge { node { id createdAt } }")?;
    let comment = &added["commentEdge"]["node"];
    if let (Some(local), Some(id)) = (value["id"].as_str(), comment["id"].as_str()) {
        cache.conn.execute(
            "UPDATE gh_comments SET node_id = ?2, created = ?3, updated = ?3 WHERE node_id = ?1",
            params![local, id, comment["createdAt"].as_str()],
        )?;
    }
    Ok(())
}
