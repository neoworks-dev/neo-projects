//! A note's front matter as properties: read in the file's order, renamed in place. And the
//! files dropped into a note, copied next to it.

use std::path::PathBuf;

use np_core::workspace::{attach_file, read_props, rename_prop};
use serde_json::json;

fn file(text: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Note.md");
    std::fs::write(&path, text).unwrap();
    (dir, path)
}

#[test]
fn props_come_in_the_order_of_the_file() {
    let (_dir, path) = file("---\nzeta: 1\nid: abc\ntags:\n  - a\n  - b\ndone: true\nwhen: 2026-10-30\n---\nbody\n");
    let props = read_props(&path).unwrap();
    let keys: Vec<&str> = props.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(keys, ["zeta", "id", "tags", "done", "when"]);
    assert_eq!(props[2].1, json!(["a", "b"]));
    assert_eq!(props[3].1, json!(true));
    assert_eq!(props[4].1, json!("2026-10-30"));
}

#[test]
fn no_or_broken_front_matter_has_no_props() {
    let (_dir, path) = file("just text\n");
    assert!(read_props(&path).unwrap().is_empty());
    let (_dir, path) = file("---\na: [unclosed\n---\nbody");
    assert!(read_props(&path).unwrap().is_empty());
}

#[test]
fn rename_keeps_the_value_and_its_comment() {
    let (_dir, path) = file("---\n# mine\nstatus:  todo # why\ntags:\n  - a\n---\nbody\n");
    rename_prop(&path, "status", "state").unwrap();
    rename_prop(&path, "tags", "labels").unwrap();
    let got = std::fs::read_to_string(&path).unwrap();
    assert_eq!(got, "---\n# mine\nstate:  todo # why\nlabels:\n  - a\n---\nbody\n");
}

#[test]
fn rename_refuses_a_taken_or_missing_key() {
    let text = "---\na: 1\nb: 2\n---\n";
    let (_dir, path) = file(text);
    assert!(rename_prop(&path, "a", "b").is_err());
    assert!(rename_prop(&path, "missing", "c").is_err());
    assert!(rename_prop(&path, "a", "bad: key").is_err());
    rename_prop(&path, "a", "a").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
}

#[test]
fn an_attached_file_is_copied_next_to_the_note_under_a_free_name() {
    let (dir, _note) = file("body\n");
    let outside = tempfile::tempdir().unwrap();
    let photo = outside.path().join("photo.png");
    std::fs::write(&photo, b"pixels").unwrap();

    let first = attach_file(dir.path(), &photo).unwrap();
    let second = attach_file(dir.path(), &photo).unwrap();
    assert_eq!(first, dir.path().join("photo.png"));
    assert_eq!(second, dir.path().join("photo 2.png"));
    assert_eq!(std::fs::read(&second).unwrap(), b"pixels");

    // A file already beside the note is linked where it is, not copied again.
    assert_eq!(attach_file(dir.path(), &first).unwrap(), first);
    assert!(!dir.path().join("photo 3.png").exists());
}
