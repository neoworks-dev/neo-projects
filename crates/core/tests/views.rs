//! What the window draws from the cache: boards by the Status field, and blocks across repos.

use np_core::{Cache, views};
use rusqlite::params;
use serde_json::json;

fn repo(cache: &Cache, id: &str, owner: &str, name: &str) {
    cache
        .conn()
        .execute(
            "INSERT INTO gh_repos (node_id, owner, name, url, owner_kind) VALUES (?1, ?2, ?3, '', 'Organization')",
            params![id, owner, name],
        )
        .unwrap();
}

fn issue(cache: &Cache, id: &str, repo: &str, number: i64, state: &str, fields: serde_json::Value) {
    cache
        .conn()
        .execute(
            "INSERT INTO gh_issues (node_id, repo, number, title, state, fields, updated) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, repo, number, format!("Issue {number}"), state, fields.to_string(), format!("2026-10-{number:02}")],
        )
        .unwrap();
}

fn status(option: &str, name: &str) -> serde_json::Value {
    json!({"F_status": {"kind": "single_select", "optionId": option, "name": name}})
}

#[test]
fn a_board_has_a_column_per_status_and_closed_issues_are_done() {
    let cache = Cache::open_in_memory().unwrap();
    repo(&cache, "R1", "o", "r");
    cache
        .conn()
        .execute(
            "INSERT INTO gh_fields (node_id, owner, name, kind, options) VALUES ('F_status', 'o', 'Status', 'single_select', ?1)",
            [json!([{"id": "O_b", "name": "Backlog"}, {"id": "O_t", "name": "Todo"},
                    {"id": "O_p", "name": "In Progress"}, {"id": "O_d", "name": "Done"}])
            .to_string()],
        )
        .unwrap();
    issue(&cache, "I1", "R1", 1, "OPEN", status("O_t", "Todo"));
    issue(&cache, "I2", "R1", 2, "OPEN", json!({}));
    issue(&cache, "I3", "R1", 3, "CLOSED", json!({}));
    issue(&cache, "I4", "R1", 4, "OPEN", status("O_p", "In Progress"));

    let board = views::board(cache.conn(), "R1").unwrap();
    let columns: Vec<(String, Vec<i64>)> = board["columns"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["name"].as_str().unwrap().to_string(),
                c["cards"].as_array().unwrap().iter().map(|card| card["number"].as_i64().unwrap()).collect(),
            )
        })
        .collect();
    assert_eq!(
        columns,
        vec![
            ("No status".into(), vec![2]),
            ("Backlog".into(), vec![]),
            ("Todo".into(), vec![1]),
            ("In Progress".into(), vec![4]),
            ("Done".into(), vec![3]),
        ]
    );
    let summary = views::repo_summary(cache.conn(), "R1").unwrap();
    assert_eq!(summary["open"], 3);
    assert_eq!(summary["inProgress"][0]["number"], 4);
}

#[test]
fn without_a_status_field_the_board_is_open_and_closed() {
    let cache = Cache::open_in_memory().unwrap();
    repo(&cache, "R1", "me", "r");
    issue(&cache, "I1", "R1", 1, "OPEN", json!({}));
    issue(&cache, "I2", "R1", 2, "CLOSED", json!({}));
    let board = views::board(cache.conn(), "R1").unwrap();
    assert_eq!(board["statusField"], serde_json::Value::Null);
    assert_eq!(board["columns"][0]["key"], "open");
    assert_eq!(board["columns"][1]["cards"][0]["number"], 2);
}

#[test]
fn open_blocks_between_repositories_are_counted_per_pair() {
    let cache = Cache::open_in_memory().unwrap();
    repo(&cache, "RA", "o", "a");
    repo(&cache, "RB", "o", "b");
    issue(&cache, "A1", "RA", 1, "OPEN", json!({}));
    issue(&cache, "A2", "RA", 2, "OPEN", json!({}));
    issue(&cache, "A3", "RA", 3, "CLOSED", json!({}));
    issue(&cache, "B1", "RB", 1, "OPEN", json!({}));
    for (from, to) in [("A1", "B1"), ("A2", "B1"), ("A3", "B1"), ("A1", "A2")] {
        cache
            .conn()
            .execute("INSERT INTO gh_links (kind, from_node, to_node) VALUES ('blocks', ?1, ?2)", params![from, to])
            .unwrap();
    }
    let blocks = views::cross_repo_blocks(cache.conn()).unwrap();
    assert_eq!(blocks, vec![json!({"from": "RA", "to": "RB", "count": 2})]);
    let b1 = views::card(cache.conn(), "B1").unwrap().unwrap();
    assert_eq!(b1["blockedBy"], 2, "the closed blocker doesn't count");
}

#[test]
fn a_card_carries_the_start_of_its_description() {
    let cache = Cache::open_in_memory().unwrap();
    repo(&cache, "R1", "o", "r");
    issue(&cache, "I1", "R1", 1, "OPEN", json!({}));
    let long = format!("  First line.\n{}", "x".repeat(700));
    cache.conn().execute("UPDATE gh_issues SET body = ?1 WHERE node_id = 'I1'", [&long]).unwrap();
    let card = views::card(cache.conn(), "I1").unwrap().unwrap();
    let excerpt = card["excerpt"].as_str().unwrap();
    assert!(excerpt.starts_with("First line.\nx"));
    assert_eq!(excerpt.chars().count(), 600);
}
