//! Folders on a canvas: a directory with a canvas of its own inside, named after it.

use std::path::Path;

use np_core::workspace::{folder_canvas_path, folder_of};

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
