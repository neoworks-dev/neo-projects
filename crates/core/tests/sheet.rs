use std::path::Path;

use np_core::sheet;

#[test]
fn text_and_code_are_sheets_and_notes_and_pictures_are_not() {
    assert!(sheet::is_sheet(Path::new("server.log")));
    assert!(sheet::is_sheet(Path::new("src/main.rs")));
    assert!(sheet::is_sheet(Path::new("Cargo.toml")));
    assert!(sheet::is_sheet(Path::new("data.csv")));
    assert!(!sheet::is_sheet(Path::new("Plan.md")));
    assert!(!sheet::is_sheet(Path::new("photo.png")));
    assert!(!sheet::is_sheet(Path::new("archive.zip")));
}

#[test]
fn code_is_highlighted_by_its_extension_and_plain_text_not_at_all() {
    assert_eq!(sheet::lang(Path::new("src/main.RS")), "rs");
    assert_eq!(sheet::lang(Path::new("app.ts")), "ts");
    assert_eq!(sheet::lang(Path::new("server.log")), "");
    assert_eq!(sheet::lang(Path::new("data.csv")), "");
}

#[test]
fn a_sheet_holds_the_first_lines_each_cut_short() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("long.txt");
    let lines: Vec<String> = (0..100).map(|i| format!("{i} {}", "x".repeat(240))).collect();
    std::fs::write(&file, lines.join("\n")).unwrap();

    let head = sheet::head(&file).unwrap();
    let shown: Vec<&str> = head.lines().collect();
    assert_eq!(shown.len(), 60);
    assert!(shown[0].starts_with("0 x"));
    assert_eq!(shown[0].chars().count(), 200);
}

#[test]
fn a_binary_file_has_no_sheet() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("blob.txt");
    std::fs::write(&file, b"MZ\0\0binary").unwrap();
    assert_eq!(sheet::head(&file), None);
}
