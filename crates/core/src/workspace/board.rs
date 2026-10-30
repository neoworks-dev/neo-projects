//! Local boards: `.board` files, YAML, over notes grouped by a front matter field.
//!
//! The file says which notes, which field makes the columns, and the manual order inside a
//! column (by note id). Moving a card writes the field into the note, not here.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use super::note::Note;
use super::{atomic_write, io};
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Board {
    #[serde(default = "one")]
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default)]
    pub filter: Filter,
    /// The front matter field whose value is the column.
    #[serde(rename = "groupBy", default)]
    pub group_by: String,
    #[serde(default)]
    pub columns: Vec<Column>,
    /// Fields to show on a card.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub show: Vec<String>,
    /// Column value to note ids, top to bottom.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub order: BTreeMap<String, Vec<String>>,
}

fn one() -> u32 {
    1
}

impl Default for Board {
    fn default() -> Self {
        Board {
            version: 1,
            title: None,
            filter: Filter::default(),
            group_by: String::new(),
            columns: Vec::new(),
            show: Vec::new(),
            order: BTreeMap::new(),
        }
    }
}

/// Which notes a board shows. Everything given must hold.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Filter {
    /// Below this folder, relative to the workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// Front matter keys the note must have.
    #[serde(default, deserialize_with = "one_or_many", skip_serializing_if = "Vec::is_empty")]
    pub has: Vec<String>,
    /// Tags (the `tags` list) the note must all carry.
    #[serde(default, deserialize_with = "one_or_many", skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

fn one_or_many<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Repr {
        One(String),
        Many(Vec<String>),
    }
    Ok(match Option::<Repr>::deserialize(d)? {
        None => Vec::new(),
        Some(Repr::One(s)) => vec![s],
        Some(Repr::Many(v)) => v,
    })
}

/// A column is written as a bare value or as `{value, label}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "ColumnRepr", into = "ColumnRepr")]
pub struct Column {
    pub value: String,
    pub label: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum ColumnRepr {
    Plain(String),
    Full {
        value: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
}

impl From<ColumnRepr> for Column {
    fn from(r: ColumnRepr) -> Column {
        match r {
            ColumnRepr::Plain(value) => Column { value, label: None },
            ColumnRepr::Full { value, label } => Column { value, label },
        }
    }
}

impl From<Column> for ColumnRepr {
    fn from(c: Column) -> ColumnRepr {
        match c.label {
            None => ColumnRepr::Plain(c.value),
            label => ColumnRepr::Full { value: c.value, label },
        }
    }
}

/// One column of a board with its notes in order.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardColumn<'a> {
    /// The field value; `None` for the "Other" column.
    pub value: Option<String>,
    pub label: String,
    pub notes: Vec<&'a Note>,
}

pub fn read_board(path: &Path) -> Result<Board> {
    let text = io(std::fs::read_to_string(path), path)?;
    if text.trim().is_empty() {
        return Ok(Board::default());
    }
    serde_yaml_ng::from_str(&text).map_err(|e| Error::Other(format!("{}: {e}", path.display())))
}

pub fn write_board(path: &Path, board: &Board) -> Result<()> {
    let text = serde_yaml_ng::to_string(board).map_err(|e| Error::Other(e.to_string()))?;
    atomic_write(path, text.as_bytes())
}

/// A front matter value as the text a column is named by.
fn column_value(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

impl Board {
    /// Whether the board's filter lets a note in. `workspace` is the workspace folder.
    pub fn matches(&self, note: &Note, workspace: &Path) -> bool {
        let f = &self.filter;
        if let Some(folder) = &f.folder
            && !note.path.starts_with(workspace.join(folder.trim_matches('/')))
        {
            return false;
        }
        if !f.has.iter().all(|k| note.props.get(k).is_some_and(|v| !v.is_null())) {
            return false;
        }
        f.tags.iter().all(|t| match note.props.get("tags") {
            Some(Value::Array(a)) => a.iter().any(|v| v.as_str() == Some(t)),
            Some(Value::String(s)) => s == t,
            _ => false,
        })
    }
}

/// The notes in the board's columns. A column lists its notes in the board's `order` first, then
/// the unlisted ones by title; notes whose value is no column (or missing) go last in "Other",
/// which only appears when something is in it. Filtering is [`Board::matches`]'s job.
pub fn board_notes<'a>(board: &Board, notes: &'a [Note]) -> Vec<BoardColumn<'a>> {
    let mut cols: Vec<BoardColumn> = board
        .columns
        .iter()
        .map(|c| BoardColumn {
            value: Some(c.value.clone()),
            label: c.label.clone().unwrap_or_else(|| c.value.clone()),
            notes: Vec::new(),
        })
        .collect();
    let mut other = BoardColumn { value: None, label: "Other".into(), notes: Vec::new() };

    let mut sorted: Vec<&Note> = notes.iter().collect();
    sorted.sort_by_key(|n| (n.title.to_lowercase(), n.path.clone()));
    for n in sorted {
        let value = n.props.get(&board.group_by).and_then(column_value);
        match cols.iter_mut().find(|c| c.value.is_some() && c.value == value) {
            Some(c) => c.notes.push(n),
            None => other.notes.push(n),
        }
    }
    for c in &mut cols {
        let ids = c.value.as_ref().and_then(|v| board.order.get(v));
        let Some(ids) = ids else { continue };
        let rank = |n: &Note| n.id.as_ref().and_then(|id| ids.iter().position(|x| x == id));
        // Stable: listed notes by their place, the rest keep their title order after them.
        c.notes.sort_by_key(|n| rank(n).unwrap_or(usize::MAX));
    }
    if !other.notes.is_empty() {
        cols.push(other);
    }
    cols
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, json};
    use std::path::PathBuf;

    fn note(title: &str, id: &str, status: Option<&str>) -> Note {
        let mut props = Map::new();
        props.insert("id".into(), json!(id));
        if let Some(s) = status {
            props.insert("status".into(), json!(s));
        }
        Note {
            id: Some(id.into()),
            title: title.into(),
            props,
            body: String::new(),
            path: PathBuf::from(format!("/w/Harness/{title}.md")),
        }
    }

    const FILE: &str = "version: 1\ntitle: Planning\nfilter: { folder: Harness, has: status, tags: [a] }\ngroupBy: status\ncolumns:\n  - backlog\n  - value: doing\n    label: In progress\n  - done\nshow: [priority]\norder:\n  doing: [n2, n1]\n";

    #[test]
    fn reads_plain_and_labelled_columns_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("P.board");
        std::fs::write(&p, FILE).unwrap();
        let b = read_board(&p).unwrap();
        assert_eq!(b.title.as_deref(), Some("Planning"));
        assert_eq!(b.filter.folder.as_deref(), Some("Harness"));
        assert_eq!(b.filter.has, ["status"]);
        assert_eq!(b.filter.tags, ["a"]);
        assert_eq!(b.columns[0], Column { value: "backlog".into(), label: None });
        assert_eq!(b.columns[1].label.as_deref(), Some("In progress"));
        assert_eq!(b.order["doing"], ["n2", "n1"]);

        write_board(&p, &b).unwrap();
        assert_eq!(read_board(&p).unwrap(), b);
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("- backlog"), "{text}");
    }

    #[test]
    fn minimal_board_reads() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("P.board");
        std::fs::write(&p, "filter: { has: status }\ngroupBy: status\ncolumns: [a, b]\n").unwrap();
        let b = read_board(&p).unwrap();
        assert_eq!(b.version, 1);
        assert_eq!(b.filter.has, ["status"]);
        assert_eq!(b.columns.len(), 2);
    }

    #[test]
    fn orders_listed_then_title_and_collects_other() {
        let board: Board = serde_yaml_ng::from_str(FILE).unwrap();
        let notes = vec![
            note("Zed", "n1", Some("doing")),
            note("Alpha", "n2", Some("doing")),
            note("Beta", "n3", Some("doing")),
            note("Carl", "n4", Some("doing")),
            note("Idle", "n5", Some("backlog")),
            note("Odd", "n6", Some("weird")),
            note("None", "n7", None),
        ];
        let cols = board_notes(&board, &notes);
        let titles = |i: usize| cols[i].notes.iter().map(|n| n.title.as_str()).collect::<Vec<_>>();
        assert_eq!(cols.len(), 4);
        assert_eq!(titles(0), ["Idle"]);
        assert_eq!(cols[1].label, "In progress");
        // Listed: n2 (Alpha) then n1 (Zed); then unlisted by title.
        assert_eq!(titles(1), ["Alpha", "Zed", "Beta", "Carl"]);
        assert!(titles(2).is_empty());
        assert_eq!(cols[3].label, "Other");
        assert_eq!(cols[3].value, None);
        assert_eq!(titles(3), ["None", "Odd"]);
    }

    #[test]
    fn other_is_absent_when_empty() {
        let board: Board = serde_yaml_ng::from_str(FILE).unwrap();
        let notes = [note("A", "n1", Some("done"))];
        let cols = board_notes(&board, &notes);
        assert_eq!(cols.len(), 3);
    }

    #[test]
    fn filter_matches() {
        let board: Board = serde_yaml_ng::from_str(FILE).unwrap();
        let mut n = note("A", "n1", Some("todo"));
        assert!(!board.matches(&n, Path::new("/w")), "no tag");
        n.props.insert("tags".into(), json!(["a", "b"]));
        assert!(board.matches(&n, Path::new("/w")));
        assert!(!board.matches(&n, Path::new("/other")), "wrong folder");
        n.props.remove("status");
        assert!(!board.matches(&n, Path::new("/w")), "no status");
    }
}
