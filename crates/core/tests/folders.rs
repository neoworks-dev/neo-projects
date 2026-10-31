//! Folders on a canvas: a directory with a canvas of its own inside, named after it.

use std::path::Path;

use np_core::Cache;
use np_core::workspace::{Canvas, Node, Workspace, folder_canvas_path, folder_of, folders, read_canvas, write_canvas};
use serde_json::json;

#[test]
fn a_folders_canvas_is_named_after_its_directory() {
    assert_eq!(folder_canvas_path(Path::new("/ws/Garden/Seeds")), Path::new("/ws/Garden/Seeds/Seeds.canvas"));
}

#[test]
fn only_a_canvas_named_after_its_directory_is_a_folder() {
    assert_eq!(folder_of(Path::new("/ws/Garden/Seeds/Seeds.canvas")), Some(Path::new("/ws/Garden/Seeds")));
    assert_eq!(folder_of(Path::new("/ws/Garden/Seeds/Plan.canvas")), None);
    assert_eq!(folder_of(Path::new("/ws/Garden/Seeds/Seeds.md")), None);
}

const SEEDS: &str = "Garden/Seeds/Seeds.canvas";

fn file_node(id: &str, file: &str) -> Node {
    serde_json::from_value(json!({"id": id, "type": "file", "file": file, "x": 0, "y": 0, "width": 280, "height": 80}))
        .unwrap()
}

fn link_node(id: &str, url: &str) -> Node {
    serde_json::from_value(json!({"id": id, "type": "link", "url": url, "x": 0, "y": 0, "width": 280, "height": 80}))
        .unwrap()
}

/// A project Garden whose canvas holds its note, `data.csv` beside the canvas, `src/main.rs`
/// below it, a link and the folder Seeds.
fn garden() -> (tempfile::TempDir, Workspace, Cache) {
    let dir = tempfile::tempdir().unwrap();
    let ws = Workspace::open(dir.path()).unwrap();
    let project = ws.create_project("Garden", None).unwrap();
    let root = ws.root().to_path_buf();
    std::fs::write(root.join("Garden/data.csv"), "a,b\n").unwrap();
    std::fs::create_dir_all(root.join("Garden/src")).unwrap();
    std::fs::write(root.join("Garden/src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::create_dir(root.join("Garden/Seeds")).unwrap();
    write_canvas(&root.join("Garden/Seeds/Seeds.canvas"), &Canvas::default()).unwrap();
    let canvas = Canvas {
        nodes: vec![
            file_node("note", &ws.rel(&project.note)),
            file_node("data", "Garden/data.csv"),
            file_node("main", "Garden/src/main.rs"),
            link_node("link", "https://example.com/"),
            file_node("seeds", "Garden/Seeds/Seeds.canvas"),
        ],
        ..Default::default()
    };
    write_canvas(&root.join("Garden/Garden.canvas"), &canvas).unwrap();
    let cache = Cache::open_in_memory().unwrap();
    (dir, ws, cache)
}

fn files(ws: &Workspace, canvas: &str) -> Vec<String> {
    read_canvas(&ws.root().join(canvas)).unwrap().nodes.into_iter().map(|n| n.file.or(n.url).unwrap_or_default()).collect()
}

#[test]
fn a_file_beside_the_canvas_moves_into_the_folder_on_disk() {
    let (_dir, ws, cache) = garden();
    let root = ws.root().to_path_buf();
    // Another canvas that names the file follows it.
    write_canvas(&root.join("Elsewhere.canvas"), &Canvas { nodes: vec![file_node("x", "Garden/data.csv")], ..Default::default() })
        .unwrap();

    folders::put(&ws, &cache, &root.join("Garden/Garden.canvas"), "data", &root.join(SEEDS), None).unwrap();

    assert!(!root.join("Garden/data.csv").exists());
    assert!(root.join("Garden/Seeds/data.csv").exists());
    assert!(!files(&ws, "Garden/Garden.canvas").contains(&"Garden/data.csv".to_string()));
    assert_eq!(files(&ws, "Garden/Seeds/Seeds.canvas"), ["Garden/Seeds/data.csv"]);
    assert_eq!(files(&ws, "Elsewhere.canvas"), ["Garden/Seeds/data.csv"]);
}

#[test]
fn a_file_taken_out_of_a_folder_goes_back_beside_the_canvas_where_it_was_dropped() {
    let (_dir, ws, cache) = garden();
    let root = ws.root().to_path_buf();
    let garden = root.join("Garden/Garden.canvas");
    folders::put(&ws, &cache, &garden, "data", &root.join(SEEDS), None).unwrap();

    folders::put(&ws, &cache, &root.join(SEEDS), "data", &garden, Some((120.0, -40.0))).unwrap();

    assert!(root.join("Garden/data.csv").exists());
    assert!(files(&ws, SEEDS).is_empty());
    let node = read_canvas(&garden).unwrap().nodes.into_iter().find(|n| n.id == "data").unwrap();
    assert_eq!((node.file.as_deref(), node.x, node.y), (Some("Garden/data.csv"), 120.0, -40.0));
}

#[test]
fn links_files_from_elsewhere_and_the_projects_note_move_only_their_nodes() {
    let (_dir, ws, cache) = garden();
    let root = ws.root().to_path_buf();
    let from = root.join("Garden/Garden.canvas");
    for id in ["link", "main", "note"] {
        folders::put(&ws, &cache, &from, id, &root.join(SEEDS), None).unwrap();
    }

    assert!(root.join("Garden/src/main.rs").exists());
    assert!(root.join("Garden/Garden.md").exists());
    assert_eq!(files(&ws, "Garden/Garden.canvas"), ["Garden/data.csv", "Garden/Seeds/Seeds.canvas"]);
    assert_eq!(files(&ws, "Garden/Seeds/Seeds.canvas"), ["https://example.com/", "Garden/src/main.rs", "Garden/Garden.md"]);
}

#[test]
fn a_folder_moves_into_another_with_what_it_holds() {
    let (_dir, ws, cache) = garden();
    let root = ws.root().to_path_buf();
    let from = root.join("Garden/Garden.canvas");
    folders::put(&ws, &cache, &from, "data", &root.join(SEEDS), None).unwrap();
    std::fs::create_dir(root.join("Garden/Beds")).unwrap();
    let mut canvas = read_canvas(&from).unwrap();
    canvas.nodes.push(file_node("beds", "Garden/Beds/Beds.canvas"));
    write_canvas(&from, &canvas).unwrap();

    folders::put(&ws, &cache, &from, "seeds", &root.join("Garden/Beds/Beds.canvas"), None).unwrap();

    assert!(root.join("Garden/Beds/Seeds/data.csv").exists());
    assert_eq!(files(&ws, "Garden/Beds/Beds.canvas"), ["Garden/Beds/Seeds/Seeds.canvas"]);
    assert_eq!(files(&ws, "Garden/Beds/Seeds/Seeds.canvas"), ["Garden/Beds/Seeds/data.csv"]);
}

#[test]
fn a_folder_cant_go_into_itself_and_nothing_changes() {
    let (_dir, ws, cache) = garden();
    let root = ws.root().to_path_buf();
    let before = files(&ws, "Garden/Garden.canvas");
    let put = folders::put(&ws, &cache, &root.join("Garden/Garden.canvas"), "seeds", &root.join(SEEDS), None);
    assert!(put.is_err());
    assert_eq!(files(&ws, "Garden/Garden.canvas"), before);
}

#[test]
fn a_name_taken_in_the_folder_is_refused() {
    let (_dir, ws, cache) = garden();
    let root = ws.root().to_path_buf();
    std::fs::write(root.join("Garden/Seeds/data.csv"), "older\n").unwrap();
    let put = folders::put(&ws, &cache, &root.join("Garden/Garden.canvas"), "data", &root.join(SEEDS), None);
    assert!(put.is_err());
    assert_eq!(std::fs::read_to_string(root.join("Garden/Seeds/data.csv")).unwrap(), "older\n");
    assert!(root.join("Garden/data.csv").exists());
}

#[test]
fn renaming_a_folder_renames_its_canvas_and_what_names_them() {
    let (_dir, ws, cache) = garden();
    let root = ws.root().to_path_buf();
    folders::put(&ws, &cache, &root.join("Garden/Garden.canvas"), "data", &root.join(SEEDS), None).unwrap();

    let (to, _) = folders::rename(&ws, &cache, &root.join("Garden/Seeds"), "Seedlings").unwrap();

    assert_eq!(to, root.join("Garden/Seedlings"));
    assert!(root.join("Garden/Seedlings/Seedlings.canvas").exists());
    assert!(!root.join("Garden/Seedlings/Seeds.canvas").exists());
    assert!(files(&ws, "Garden/Garden.canvas").contains(&"Garden/Seedlings/Seedlings.canvas".to_string()));
    assert_eq!(files(&ws, "Garden/Seedlings/Seedlings.canvas"), ["Garden/Seedlings/data.csv"]);
}

#[test]
fn renaming_onto_a_taken_name_is_refused() {
    let (_dir, ws, cache) = garden();
    let root = ws.root().to_path_buf();
    assert!(folders::rename(&ws, &cache, &root.join("Garden/Seeds"), "src").is_err());
    assert!(root.join("Garden/Seeds/Seeds.canvas").exists());
}
