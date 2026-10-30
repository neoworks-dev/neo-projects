//! Local issues: the issues of a project without a repository, one Markdown file each in the
//! project's `Issues` folder (`Issues/12.md`). The front matter holds what GitHub would (title,
//! status, open or closed, priority, effort, labels, assignees, what blocks it, its parent and
//! the comments) and the body is the description, so they read fine in any Markdown editor.
//!
//! The window knows one by its node id, `issue:<workspace-relative path>`, and gets the same
//! shapes as for a GitHub issue ([`card`], [`detail`], [`board`]), so one board and one panel
//! serve both. Relationships name issues by number, within the project.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use super::note::{create_note, parse_note, set_prop, write_body};
use super::project::{Project, project_at};
use super::{Workspace, io};
use crate::cache::now;
use crate::{Error, Result};

/// The folder in a project that holds its issues.
pub const ISSUES_DIR: &str = "Issues";
/// The prefix of a local issue's node id.
pub const NODE_PREFIX: &str = "issue:";

/// The board's columns unless the project note lists its own under `statuses`. The last is done.
const STATUSES: [&str; 4] = ["Backlog", "Todo", "In Progress", "Done"];
const PRIORITIES: [&str; 4] = ["Urgent", "High", "Medium", "Low"];
const EFFORTS: [&str; 3] = ["High", "Medium", "Low"];
/// Offered in the label picker before a project has labels of its own.
const LABELS: [&str; 4] = ["bug", "feature", "design", "docs"];

#[derive(Debug, Clone, PartialEq)]
pub struct LocalIssue {
    pub path: PathBuf,
    pub number: i64,
    pub title: String,
    pub props: Map<String, Value>,
    pub body: String,
}

impl LocalIssue {
    pub fn is_open(&self) -> bool {
        self.props.get("state").and_then(Value::as_str) != Some("closed")
    }

    fn str(&self, key: &str) -> Option<&str> {
        self.props.get(key).and_then(Value::as_str).filter(|s| !s.is_empty())
    }

    fn strings(&self, key: &str) -> Vec<String> {
        match self.props.get(key) {
            Some(Value::Array(items)) => items.iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
            Some(Value::String(s)) => vec![s.clone()],
            _ => Vec::new(),
        }
    }

    fn numbers(&self, key: &str) -> Vec<i64> {
        match self.props.get(key) {
            Some(Value::Array(items)) => items.iter().filter_map(Value::as_i64).collect(),
            Some(Value::Number(n)) => n.as_i64().into_iter().collect(),
            _ => Vec::new(),
        }
    }

    fn comments(&self) -> Vec<Value> {
        self.props.get("comments").and_then(Value::as_array).cloned().unwrap_or_default()
    }
}

/// Whether a node id names a local issue rather than a GitHub one.
pub fn is_local(node: &str) -> bool {
    node.starts_with(NODE_PREFIX)
}

pub fn read_issue(path: &Path) -> Result<LocalIssue> {
    let text = io(std::fs::read_to_string(path), path)?;
    let note = parse_note(path, &text);
    if note.props.get("type").and_then(Value::as_str) != Some("issue") {
        return Err(Error::Other(format!("{} is not an issue", path.display())));
    }
    let number = note.props.get("number").and_then(Value::as_i64).unwrap_or(0);
    let title = note.props.get("title").and_then(Value::as_str).map(str::to_string).unwrap_or(note.title);
    Ok(LocalIssue { path: path.to_path_buf(), number, title, props: note.props, body: note.body })
}

/// The project's statuses, in board order; the last one is done.
pub fn statuses(project: &Project) -> Vec<String> {
    let listed = std::fs::read_to_string(&project.note)
        .ok()
        .map(|text| parse_note(&project.note, &text).props)
        .and_then(|props| props.get("statuses").and_then(Value::as_array).cloned())
        .map(|items| items.iter().filter_map(|v| v.as_str().map(str::to_string)).collect::<Vec<_>>())
        .filter(|list| !list.is_empty());
    listed.unwrap_or_else(|| STATUSES.iter().map(|s| s.to_string()).collect())
}

/// The GitHub colour name a status is drawn in.
fn status_color(name: &str, done: bool) -> &'static str {
    if done {
        return "PURPLE";
    }
    match name.to_lowercase().as_str() {
        "backlog" => "GRAY",
        "todo" | "to do" => "BLUE",
        "in progress" | "doing" => "YELLOW",
        "review" | "in review" => "ORANGE",
        _ => "GREEN",
    }
}

/// A label's colour, six hex digits as GitHub gives them, picked by its name.
fn label_color(name: &str) -> String {
    const COLORS: [&str; 8] = ["d73a4a", "a2eeef", "7057ff", "0075ca", "e4e669", "008672", "d876e3", "fbca04"];
    let hash = name.bytes().fold(0usize, |h, b| h.wrapping_mul(31).wrapping_add(b as usize));
    COLORS[hash % COLORS.len()].to_string()
}

impl Workspace {
    fn node_of(&self, path: &Path) -> String {
        format!("{NODE_PREFIX}{}", path.strip_prefix(self.root()).unwrap_or(path).to_string_lossy())
    }

    /// The file a local issue's node id names.
    pub fn issue_path(&self, node: &str) -> Result<PathBuf> {
        let rel = node.strip_prefix(NODE_PREFIX).ok_or_else(|| Error::Other(format!("{node} is not a local issue")))?;
        let path = self.root().join(rel);
        if rel.contains("..") || !path.exists() {
            return Err(Error::Other(format!("no issue at {rel}")));
        }
        Ok(path)
    }

    /// The project a local issue belongs to: the folder above its `Issues` folder.
    pub fn issue_project(&self, path: &Path) -> Result<Project> {
        path.parent()
            .and_then(Path::parent)
            .and_then(project_at)
            .ok_or_else(|| Error::Other(format!("{} is in no project", path.display())))
    }

    /// The project's issues, by number.
    pub fn local_issues(&self, project: &Project) -> Vec<LocalIssue> {
        let dir = project.folder.join(ISSUES_DIR);
        let Ok(read) = std::fs::read_dir(&dir) else { return Vec::new() };
        let mut issues: Vec<LocalIssue> = read
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .filter_map(|p| read_issue(&p).ok())
            .collect();
        issues.sort_by_key(|i| i.number);
        issues
    }

    /// A new open issue, numbered after the project's highest, in `status` or the first column.
    pub fn create_issue(&self, project: &Project, title: &str, status: Option<&str>) -> Result<String> {
        let number = self.local_issues(project).iter().map(|i| i.number).max().unwrap_or(0) + 1;
        let first = statuses(project).into_iter().next().unwrap_or_default();
        let mut props = Map::new();
        props.insert("type".into(), json!("issue"));
        props.insert("number".into(), json!(number));
        props.insert("title".into(), json!(title.trim()));
        props.insert("status".into(), json!(status.unwrap_or(&first)));
        props.insert("created".into(), json!(now()));
        let path = create_note(&project.folder.join(ISSUES_DIR), &number.to_string(), &props)?;
        Ok(self.node_of(&path))
    }

    /// Sets one field of a local issue, taking values in the shapes the window sends for GitHub
    /// issues: `title`, `body`, `state`, `labels`, `assignees`, `blockedBy` and `parent` (node
    /// ids), and `field:<status|priority|effort>`.
    pub fn set_issue_field(&self, node: &str, field: &str, value: &Value) -> Result<()> {
        let path = self.issue_path(node)?;
        let names = |v: &Value, key: &str| -> Vec<Value> {
            v.as_array().into_iter().flatten().filter_map(|x| x[key].as_str().map(|s| json!(s))).collect()
        };
        match field {
            "title" => set_prop(&path, "title", Some(&json!(value.as_str().unwrap_or_default().trim()))),
            "body" => write_body(&path, value.as_str().unwrap_or_default()),
            "state" => self.set_state(&path, value["state"] == "OPEN", value["reason"].as_str()),
            "labels" => set_list(&path, "labels", names(value, "name")),
            "assignees" => set_list(&path, "assignees", names(value, "login")),
            "blockedBy" => {
                let numbers = value.as_array().into_iter().flatten().filter_map(|n| self.number_of(n)).collect();
                set_list(&path, "blockedBy", numbers)
            }
            "parent" => set_prop(&path, "parent", self.number_of(value).as_ref()),
            "field:status" => match value["name"].as_str().or(value["optionId"].as_str()) {
                Some(status) => self.move_issue(node, status),
                None => set_prop(&path, "status", None),
            },
            _ => {
                let key = field
                    .strip_prefix("field:")
                    .filter(|k| matches!(*k, "priority" | "effort"))
                    .ok_or_else(|| Error::Other(format!("a local issue has no field {field}")))?;
                let name = value["name"].as_str().or(value["optionId"].as_str()).map(|s| json!(s));
                set_prop(&path, key, name.as_ref())
            }
        }
    }

    /// Moves an issue to a column: its status, closing it in the last column and reopening it
    /// when it leaves.
    pub fn move_issue(&self, node: &str, status: &str) -> Result<()> {
        let path = self.issue_path(node)?;
        let project = self.issue_project(&path)?;
        let all = statuses(&project);
        if !all.iter().any(|s| s == status) {
            return Err(Error::Other(format!("no column {status}")));
        }
        set_prop(&path, "status", Some(&json!(status)))?;
        let done = all.last().is_some_and(|last| last == status);
        let issue = read_issue(&path)?;
        if done && issue.is_open() {
            set_prop(&path, "state", Some(&json!("closed")))?;
            set_prop(&path, "closed", Some(&json!("done")))?;
        } else if !done && !issue.is_open() {
            set_prop(&path, "state", None)?;
            set_prop(&path, "closed", None)?;
        }
        Ok(())
    }

    /// Opens or closes an issue. Closing moves it to the done column; reopening one that was
    /// there moves it back to the first.
    fn set_state(&self, path: &Path, open: bool, reason: Option<&str>) -> Result<()> {
        let project = self.issue_project(path)?;
        let all = statuses(&project);
        if open {
            set_prop(path, "state", None)?;
            set_prop(path, "closed", None)?;
            let issue = read_issue(path)?;
            if issue.str("status") == all.last().map(String::as_str) {
                set_prop(path, "status", all.first().map(|s| json!(s)).as_ref())?;
            }
            return Ok(());
        }
        let reason = if reason == Some("NOT_PLANNED") { "not-planned" } else { "done" };
        set_prop(path, "state", Some(&json!("closed")))?;
        set_prop(path, "closed", Some(&json!(reason)))?;
        set_prop(path, "status", all.last().map(|s| json!(s)).as_ref())
    }

    pub fn add_issue_comment(&self, node: &str, author: &str, body: &str) -> Result<()> {
        let path = self.issue_path(node)?;
        let issue = read_issue(&path)?;
        let mut comments = issue.comments();
        comments.push(json!({"author": author, "created": now(), "body": body}));
        set_prop(&path, "comments", Some(&Value::Array(comments)))
    }

    /// The number of the issue a node id names, when it is a local issue.
    fn number_of(&self, node: &Value) -> Option<Value> {
        let path = self.issue_path(node.as_str()?).ok()?;
        Some(json!(read_issue(&path).ok()?.number))
    }

    /// One issue as a card, as [`crate::views::card`] gives a GitHub one. `siblings` are the
    /// project's issues, for what blocks what.
    pub fn issue_card(&self, project: &Project, issue: &LocalIssue, siblings: &[LocalIssue]) -> Value {
        let open_with = |numbers: Vec<i64>| {
            siblings.iter().filter(|s| numbers.contains(&s.number) && s.is_open()).count()
        };
        let blocking = siblings.iter().filter(|s| s.is_open() && s.numbers("blockedBy").contains(&issue.number)).count();
        let mut fields = Map::new();
        for (key, name) in [("status", "Status"), ("priority", "Priority"), ("effort", "Effort")] {
            if let Some(v) = issue.str(key) {
                fields.insert(name.into(), json!(v));
            }
        }
        let reason = match issue.str("closed") {
            _ if issue.is_open() => Value::Null,
            Some("not-planned") => json!("NOT_PLANNED"),
            _ => json!("COMPLETED"),
        };
        json!({
            "node": self.node_of(&issue.path),
            "repo": self.node_of(&project.folder),
            "repoName": project.name,
            "number": issue.number,
            "title": issue.title,
            "state": if issue.is_open() { "OPEN" } else { "CLOSED" },
            "stateReason": reason,
            "labels": issue.strings("labels").iter().map(|l| json!({"name": l, "color": label_color(l)})).collect::<Vec<_>>(),
            "assignees": issue.strings("assignees"),
            "fields": fields,
            "blockedBy": open_with(issue.numbers("blockedBy")),
            "blocking": blocking,
            "comments": issue.comments().len(),
            "url": null,
            "updated": issue.str("created"),
            "pending": false,
            "conflict": false,
            "local": true,
        })
    }

    /// The project's board: a column per status, open issues by priority then newest, closed
    /// ones in the last column.
    pub fn issue_board(&self, project: &Project) -> Value {
        let issues = self.local_issues(project);
        let all = statuses(project);
        let last = all.len().saturating_sub(1);
        let mut columns: Vec<Vec<&LocalIssue>> = vec![Vec::new(); all.len()];
        for issue in &issues {
            let at = match issue.str("status").and_then(|s| all.iter().position(|a| a == s)) {
                _ if !issue.is_open() => last,
                Some(i) => i,
                None => 0,
            };
            columns[at].push(issue);
        }
        let out: Vec<Value> = columns
            .into_iter()
            .enumerate()
            .map(|(i, mut list)| {
                list.sort_by_key(|issue| (priority_rank(issue.str("priority")), std::cmp::Reverse(issue.number)));
                let cards: Vec<Value> = list.into_iter().map(|issue| self.issue_card(project, issue, &issues)).collect();
                json!({"key": all[i], "name": all[i], "color": status_color(&all[i], i == last), "done": i == last, "cards": cards})
            })
            .collect();
        json!({"statusField": "status", "local": true, "columns": out})
    }

    /// Counts and the status split for the overview, as [`crate::views::repo_summary`] gives.
    pub fn issue_summary(&self, project: &Project) -> Value {
        let issues = self.local_issues(project);
        let all = statuses(project);
        let open: Vec<&LocalIssue> = issues.iter().filter(|i| i.is_open()).collect();
        let blocked = open
            .iter()
            .filter(|i| i.numbers("blockedBy").iter().any(|n| issues.iter().any(|s| s.number == *n && s.is_open())))
            .count();
        let last = all.len().saturating_sub(1);
        let split: Vec<Value> = all
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != last)
            .map(|(i, name)| {
                let n = open.iter().filter(|o| o.str("status").unwrap_or(&all[0]) == name).count();
                json!({"name": name, "color": status_color(name, i == last), "count": n})
            })
            .collect();
        let in_progress: Vec<Value> = open
            .iter()
            .filter(|i| i.str("status").is_some_and(|s| s.eq_ignore_ascii_case("in progress")))
            .take(3)
            .map(|i| json!({"node": self.node_of(&i.path), "number": i.number, "title": i.title}))
            .collect();
        json!({
            "open": open.len(),
            "closed": issues.len() - open.len(),
            "blocked": blocked,
            "hasStatus": true,
            "split": split,
            "inProgress": in_progress,
            "pulledAt": null,
        })
    }

    /// One issue with everything its panel shows, as [`crate::views::issue`] gives a GitHub one.
    pub fn issue_detail(&self, node: &str) -> Result<Value> {
        let path = self.issue_path(node)?;
        let project = self.issue_project(&path)?;
        let issues = self.local_issues(&project);
        let issue = read_issue(&path)?;
        let mut detail = self.issue_card(&project, &issue, &issues);
        let linked = |numbers: Vec<i64>| -> Vec<Value> {
            issues.iter().filter(|s| numbers.contains(&s.number)).map(|s| self.issue_card(&project, s, &issues)).collect()
        };
        let select = |id: &str, name: &str, options: Vec<Value>| {
            let value = issue.str(id).map(|v| json!({"kind": "single_select", "optionId": v, "name": v}));
            json!({"id": id, "name": name, "kind": "single_select", "options": options, "value": value})
        };
        let all = statuses(&project);
        let last = all.len().saturating_sub(1);
        let status_options =
            all.iter().enumerate().map(|(i, s)| json!({"id": s, "name": s, "color": status_color(s, i == last)})).collect();
        let named = |names: &[&str]| names.iter().map(|n| json!({"id": n, "name": n})).collect::<Vec<_>>();
        let mut labels: Vec<String> = LABELS.iter().map(|l| l.to_string()).collect();
        let mut users: Vec<String> = Vec::new();
        for other in &issues {
            labels.extend(other.strings("labels"));
            users.extend(other.strings("assignees"));
        }
        if let Some(me) = self.config().github.login.clone() {
            users.push(me);
        }
        labels.sort();
        labels.dedup();
        users.sort();
        users.dedup();
        let parent = issue.props.get("parent").and_then(Value::as_i64);
        detail["body"] = json!(issue.body.trim_start_matches('\n'));
        detail["author"] = json!(issue.str("author"));
        detail["created"] = json!(issue.str("created"));
        detail["milestone"] = Value::Null;
        detail["issueType"] = Value::Null;
        detail["labelsFull"] =
            json!(issue.strings("labels").iter().map(|l| json!({"id": l, "name": l, "color": label_color(l)})).collect::<Vec<_>>());
        detail["assigneesFull"] = json!(issue.strings("assignees").iter().map(|u| json!({"id": u, "login": u})).collect::<Vec<_>>());
        detail["fieldDefs"] = json!([
            select("status", "Status", status_options),
            select("priority", "Priority", named(&PRIORITIES)),
            select("effort", "Effort", named(&EFFORTS)),
        ]);
        detail["repoLabels"] = json!(labels.iter().map(|l| json!({"id": l, "name": l, "color": label_color(l)})).collect::<Vec<_>>());
        detail["repoUsers"] = json!(users.iter().map(|u| json!({"id": u, "login": u})).collect::<Vec<_>>());
        detail["blockedByList"] = json!(linked(issue.numbers("blockedBy")));
        detail["blockingList"] = json!(
            issues
                .iter()
                .filter(|s| s.numbers("blockedBy").contains(&issue.number))
                .map(|s| self.issue_card(&project, s, &issues))
                .collect::<Vec<_>>()
        );
        detail["parent"] = parent.map(|p| linked(vec![p])).and_then(|l| l.into_iter().next()).unwrap_or(Value::Null);
        detail["subIssues"] = json!(
            issues
                .iter()
                .filter(|s| s.props.get("parent").and_then(Value::as_i64) == Some(issue.number))
                .map(|s| self.issue_card(&project, s, &issues))
                .collect::<Vec<_>>()
        );
        detail["commentList"] = json!(
            issue
                .comments()
                .iter()
                .enumerate()
                .map(|(i, c)| json!({"id": format!("{node}#{i}"), "author": c["author"], "body": c["body"],
                                     "created": c["created"], "pending": false}))
                .collect::<Vec<_>>()
        );
        detail["conflicts"] = json!([]);
        detail["statusField"] = json!("status");
        Ok(detail)
    }

    /// Local issues in every project whose title contains `query` or whose number it is.
    pub fn search_issues(&self, query: &str, limit: usize) -> Vec<Value> {
        let q = query.trim().trim_start_matches('#').to_lowercase();
        let number: Option<i64> = q.parse().ok();
        let (projects, _) = self.projects();
        let mut out = Vec::new();
        for project in projects.iter().filter(|p| p.repos.is_empty()) {
            let issues = self.local_issues(project);
            for issue in &issues {
                if Some(issue.number) == number || issue.title.to_lowercase().contains(&q) {
                    out.push(self.issue_card(project, issue, &issues));
                }
            }
        }
        out.truncate(limit);
        out
    }

    /// The blocks and parents between the given local issues: `(kind, from, to)` node ids, with
    /// `blocks` from the blocker and `parent` from the parent.
    pub fn issue_links(&self, nodes: &[String]) -> Vec<(String, String, String)> {
        let mut out = Vec::new();
        for node in nodes {
            let Ok(path) = self.issue_path(node) else { continue };
            let Ok(project) = self.issue_project(&path) else { continue };
            let Ok(issue) = read_issue(&path) else { continue };
            let by_number = |n: i64| {
                self.local_issues(&project).into_iter().find(|s| s.number == n).map(|s| self.node_of(&s.path))
            };
            for blocker in issue.numbers("blockedBy").into_iter().filter_map(by_number) {
                if nodes.contains(&blocker) {
                    out.push(("blocks".into(), blocker, node.clone()));
                }
            }
            if let Some(parent) = issue.props.get("parent").and_then(Value::as_i64).and_then(by_number)
                && nodes.contains(&parent)
            {
                out.push(("parent".into(), parent, node.clone()));
            }
        }
        out
    }
}

fn priority_rank(priority: Option<&str>) -> usize {
    priority.and_then(|p| PRIORITIES.iter().position(|q| *q == p)).unwrap_or(PRIORITIES.len())
}

/// Sets a list key, or removes it when the list is empty.
fn set_list(path: &Path, key: &str, items: Vec<Value>) -> Result<()> {
    if items.is_empty() {
        return set_prop(path, key, None);
    }
    set_prop(path, key, Some(&Value::Array(items)))
}


#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (tempfile::TempDir, Workspace, Project) {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        let project = ws.create_project("Garden", None).unwrap();
        (dir, ws, project)
    }

    #[test]
    fn creates_numbered_files_in_the_first_column() {
        let (_dir, ws, project) = setup();
        let a = ws.create_issue(&project, "Plant tomatoes", None).unwrap();
        let b = ws.create_issue(&project, "Build a shed", Some("Todo")).unwrap();
        assert_eq!(a, "issue:Garden/Issues/1.md");
        assert_eq!(b, "issue:Garden/Issues/2.md");
        let issue = read_issue(&ws.issue_path(&a).unwrap()).unwrap();
        assert_eq!(issue.title, "Plant tomatoes");
        assert_eq!(issue.str("status"), Some("Backlog"));
        let board = ws.issue_board(&project);
        assert_eq!(board["columns"][0]["cards"][0]["title"], "Plant tomatoes");
        assert_eq!(board["columns"][1]["cards"][0]["title"], "Build a shed");
    }

    #[test]
    fn the_last_column_closes_and_leaving_it_reopens() {
        let (_dir, ws, project) = setup();
        let a = ws.create_issue(&project, "Water", None).unwrap();
        ws.move_issue(&a, "Done").unwrap();
        let card = &ws.issue_board(&project)["columns"][3]["cards"][0];
        assert_eq!(card["state"], "CLOSED");
        assert_eq!(card["stateReason"], "COMPLETED");
        ws.move_issue(&a, "Todo").unwrap();
        assert_eq!(ws.issue_board(&project)["columns"][1]["cards"][0]["state"], "OPEN");
        ws.set_issue_field(&a, "state", &json!({"state": "CLOSED", "reason": "NOT_PLANNED"})).unwrap();
        let card = &ws.issue_board(&project)["columns"][3]["cards"][0];
        assert_eq!(card["stateReason"], "NOT_PLANNED");
        ws.set_issue_field(&a, "state", &json!({"state": "OPEN", "reason": null})).unwrap();
        assert_eq!(ws.issue_board(&project)["columns"][0]["cards"][0]["state"], "OPEN");
        assert!(ws.move_issue(&a, "Nowhere").is_err());
    }

    #[test]
    fn fields_labels_relationships_and_comments() {
        let (_dir, ws, project) = setup();
        let a = ws.create_issue(&project, "Dig", None).unwrap();
        let b = ws.create_issue(&project, "Plant", None).unwrap();
        ws.set_issue_field(&b, "title", &json!("Plant seeds ")).unwrap();
        ws.set_issue_field(&b, "body", &json!("In rows.\n")).unwrap();
        ws.set_issue_field(&b, "field:priority", &json!({"optionId": "High", "name": "High"})).unwrap();
        ws.set_issue_field(&b, "labels", &json!([{"id": "bug", "name": "bug"}])).unwrap();
        ws.set_issue_field(&b, "blockedBy", &json!([a])).unwrap();
        ws.set_issue_field(&a, "parent", &json!(b)).unwrap();
        ws.add_issue_comment(&b, "moritz", "Soon: \"after rain\"\nok").unwrap();

        let detail = ws.issue_detail(&b).unwrap();
        assert_eq!(detail["title"], "Plant seeds");
        assert_eq!(detail["body"], "In rows.\n");
        assert_eq!(detail["fields"]["Priority"], "High");
        assert_eq!(detail["labelsFull"][0]["name"], "bug");
        assert_eq!(detail["blockedBy"], 1);
        assert_eq!(detail["blockedByList"][0]["node"], a.as_str());
        assert_eq!(detail["subIssues"][0]["node"], a.as_str());
        assert_eq!(detail["commentList"][0]["body"], "Soon: \"after rain\"\nok");
        assert_eq!(ws.issue_detail(&a).unwrap()["blockingList"][0]["node"], b.as_str());
        assert_eq!(ws.issue_detail(&a).unwrap()["parent"]["node"], b.as_str());

        let links = ws.issue_links(&[a.clone(), b.clone()]);
        assert!(links.contains(&("blocks".into(), a.clone(), b.clone())));
        assert!(links.contains(&("parent".into(), b.clone(), a.clone())));

        ws.set_issue_field(&b, "blockedBy", &json!([])).unwrap();
        assert_eq!(ws.issue_detail(&b).unwrap()["blockedBy"], 0);
        assert_eq!(ws.search_issues("#1", 10)[0]["node"], a.as_str());
        assert_eq!(ws.search_issues("seeds", 10)[0]["node"], b.as_str());
        assert!(ws.issue_path("issue:../etc/passwd").is_err());
    }

    #[test]
    fn summary_counts_open_blocked_and_the_split() {
        let (_dir, ws, project) = setup();
        let a = ws.create_issue(&project, "A", None).unwrap();
        let b = ws.create_issue(&project, "B", Some("In Progress")).unwrap();
        let c = ws.create_issue(&project, "C", None).unwrap();
        ws.set_issue_field(&b, "blockedBy", &json!([a])).unwrap();
        ws.move_issue(&c, "Done").unwrap();
        let s = ws.issue_summary(&project);
        assert_eq!(s["open"], 2);
        assert_eq!(s["closed"], 1);
        assert_eq!(s["blocked"], 1);
        assert_eq!(s["split"][0], json!({"name": "Backlog", "color": "GRAY", "count": 1}));
        assert_eq!(s["inProgress"][0]["title"], "B");
    }
}
