//! The sync against a fake GitHub: what a pull merges, and what a push sends.

use std::cell::RefCell;

use np_core::github::{self, Client, Transport};
use np_core::{Cache, Result, fields};
use serde_json::{Value, json};

/// Answers queries from a state the test sets, and records every mutation.
#[derive(Default)]
struct Fake {
    issues: RefCell<Vec<Value>>,
    mutations: RefCell<Vec<(String, Value)>>,
}

impl Transport for &Fake {
    fn post(&self, body: &Value) -> Result<Value> {
        let query = body["query"].as_str().unwrap();
        let data = if query.contains("query RepoMeta") {
            json!({"viewer": {"login": "me"}, "repository": {
                "id": "R1", "url": "https://github.com/o/r", "isArchived": false,
                "owner": {"__typename": "Organization", "login": "o", "issueFields": {"nodes": [
                    {"__typename": "IssueFieldSingleSelect", "id": "F_status", "name": "Status",
                     "options": [{"id": "O_todo", "name": "Todo"}, {"id": "O_doing", "name": "Doing"}]}]}},
                "labels": {"nodes": [{"id": "L_a", "name": "a"}, {"id": "L_b", "name": "b"}, {"id": "L_c", "name": "c"}]},
                "assignableUsers": {"nodes": [{"id": "U_me", "login": "me"}]}}})
        } else if query.contains("query RepoIssues") {
            json!({"repository": {"issues": {"pageInfo": {"hasNextPage": false},
                "nodes": self.issues.borrow().clone()}}})
        } else {
            let name = query.split_whitespace().nth(1).unwrap().split('(').next().unwrap().to_string();
            let mut answer = json!({});
            if name == "CreateIssue" {
                answer = json!({"createIssue": {"issue": {"id": "I_new", "number": 9, "url": "u", "createdAt": "t", "updatedAt": "t"}}});
            }
            self.mutations.borrow_mut().push((name, body["variables"]["input"].clone()));
            answer
        };
        Ok(json!({"data": data}))
    }
}

fn issue(id: &str, title: &str, labels: &[&str], blocked_by: &[&str], updated: &str) -> Value {
    json!({
        "id": id, "number": id.trim_start_matches("I").len(), "url": "u", "title": title, "body": "",
        "state": "OPEN", "stateReason": null, "createdAt": "t0", "updatedAt": updated,
        "author": {"login": "me"}, "comments": {"totalCount": 0},
        "labels": {"nodes": labels.iter().map(|l| json!({"id": l, "name": l, "color": null})).collect::<Vec<_>>()},
        "assignees": {"nodes": []},
        "blockedBy": {"nodes": blocked_by.iter().map(|b| json!({"id": b, "number": 1, "title": "x", "state": "OPEN",
            "url": "u", "repository": {"nameWithOwner": "o/r"}})).collect::<Vec<_>>()},
        "parent": null,
        "issueFieldValues": {"nodes": [{"__typename": "IssueFieldSingleSelectValue", "optionId": "O_todo",
            "name": "Todo", "field": {"id": "F_status"}}]},
    })
}

fn setup(fake: &Fake, issues: Vec<Value>) -> Cache {
    *fake.issues.borrow_mut() = issues;
    let mut cache = Cache::open_in_memory().unwrap();
    github::pull(&mut cache, &Client::new(fake), "o", "r").unwrap();
    cache
}

fn pull(cache: &mut Cache, fake: &Fake, issues: Vec<Value>) -> github::PullReport {
    *fake.issues.borrow_mut() = issues;
    github::pull(cache, &Client::new(fake), "o", "r").unwrap()
}

#[test]
fn a_local_edit_survives_a_pull_and_is_pushed() {
    let fake = Fake::default();
    let mut cache = setup(&fake, vec![issue("I1", "Old", &[], &[], "t1")]);
    cache.set_field("I1", "title", json!("New")).unwrap();
    // GitHub still has the old title: the edit stands.
    pull(&mut cache, &fake, vec![issue("I1", "Old", &[], &[], "t1")]);
    assert_eq!(fields::read(cache.conn(), "I1", "title").unwrap(), json!("New"));

    let report = github::push(&mut cache, &Client::new(&fake)).unwrap();
    assert_eq!(report.sent, 1);
    assert_eq!(fake.mutations.borrow()[0], ("UpdateIssue".into(), json!({"id": "I1", "title": "New"})));
    assert!(cache.outbox().unwrap().is_empty());
    assert_eq!(fields::base(cache.conn(), "I1", "title").unwrap(), Some(json!("New")));
}

#[test]
fn both_sides_changing_a_title_is_a_conflict() {
    let fake = Fake::default();
    let mut cache = setup(&fake, vec![issue("I1", "Old", &[], &[], "t1")]);
    cache.set_field("I1", "title", json!("Mine")).unwrap();
    let report = pull(&mut cache, &fake, vec![issue("I1", "Theirs", &[], &[], "t2")]);
    assert_eq!(report.conflicts, 1);
    assert_eq!(fields::read(cache.conn(), "I1", "title").unwrap(), json!("Mine"));
    // A conflict is not pushed until it is resolved.
    assert_eq!(github::push(&mut cache, &Client::new(&fake)).unwrap().waiting, 1);

    cache.resolve("I1", "title", false).unwrap();
    assert_eq!(fields::read(cache.conn(), "I1", "title").unwrap(), json!("Theirs"));
    assert!(cache.outbox().unwrap().is_empty() && cache.conflicts().unwrap().is_empty());
}

#[test]
fn labels_merge_as_sets() {
    let fake = Fake::default();
    let mut cache = setup(&fake, vec![issue("I1", "T", &["L_a", "L_b"], &[], "t1")]);
    let label = |id: &str| json!({"id": id, "name": id, "color": null});
    cache.set_field("I1", "labels", json!([label("L_a"), label("L_b"), label("L_c")])).unwrap(); // added c
    pull(&mut cache, &fake, vec![issue("I1", "T", &["L_b"], &[], "t2")]); // GitHub removed a
    let merged = fields::read(cache.conn(), "I1", "labels").unwrap();
    assert_eq!(merged, json!([label("L_b"), label("L_c")]));
    assert_eq!(cache.outbox().unwrap()[0].value, merged);
}

#[test]
fn a_remote_change_without_a_local_edit_just_lands() {
    let fake = Fake::default();
    let mut cache = setup(&fake, vec![issue("I1", "Old", &[], &[], "t1")]);
    pull(&mut cache, &fake, vec![issue("I1", "New", &[], &["I2"], "t2")]);
    assert_eq!(fields::read(cache.conn(), "I1", "title").unwrap(), json!("New"));
    assert_eq!(fields::read(cache.conn(), "I1", "blockedBy").unwrap(), json!(["I2"]));
    assert!(cache.outbox().unwrap().is_empty());
}

#[test]
fn a_new_issue_is_created_before_it_is_linked() {
    let fake = Fake::default();
    let mut cache = setup(&fake, vec![issue("I1", "T", &[], &[], "t1")]);
    let local = cache.create_issue("R1", "Blocker", "").unwrap();
    cache.set_field("I1", "blockedBy", json!([local])).unwrap();
    cache
        .set_field("I1", "field:F_status", json!({"kind": "single_select", "optionId": "O_doing", "name": "Doing"}))
        .unwrap();

    let report = github::push(&mut cache, &Client::new(&fake)).unwrap();
    assert_eq!((report.sent, report.failed, report.waiting), (3, 0, 0));
    let sent = fake.mutations.borrow().clone();
    assert_eq!(sent[0].0, "CreateIssue");
    assert_eq!(sent[1], ("AddBlockedBy".into(), json!({"issueId": "I1", "blockingIssueId": "I_new"})));
    assert_eq!(
        sent[2],
        (
            "SetIssueFieldValue".into(),
            json!({"issueId": "I1", "issueFields": [{"fieldId": "F_status", "singleSelectOptionId": "O_doing"}]})
        )
    );
    assert_eq!(cache.issue_id("R1", 9).unwrap().as_deref(), Some("I_new"));
}

#[test]
fn comments_go_out_as_you() {
    let fake = Fake::default();
    let mut cache = setup(&fake, vec![issue("I1", "T", &[], &[], "t1")]);
    cache.add_comment("I1", "looks good").unwrap();
    github::push(&mut cache, &Client::new(&fake)).unwrap();
    assert_eq!(fake.mutations.borrow()[0], ("AddComment".into(), json!({"subjectId": "I1", "body": "looks good"})));
}
