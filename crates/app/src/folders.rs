//! Folders on a project canvas: a directory with a canvas of its own inside, named after it. A
//! file or folder that sits directly in the directory of the canvas it is dropped from moves into
//! the folder on disk; anything else (a link, an issue, a file from elsewhere) moves only its node.

use std::path::{Path, PathBuf};

use np_core::workspace::{self, folder_canvas_path, folder_of, read_canvas, write_canvas};
use np_core::{Error, Result};

use crate::canvas::{self, free_spot, rel};
use crate::state::Open;

/// Moves the node `id` from the canvas at `from` onto the canvas of the folder `dir`.
pub fn put(open: &Open, from: &Path, id: &str, dir: &Path) -> Result<()> {
    let ws = open.ws().clone();
    let mut source = read_canvas(from)?;
    let Some(index) = source.nodes.iter().position(|n| n.id == id) else {
        return Err(Error::Other(format!("no node {id} on {}", rel(&ws, from))));
    };
    let file = source.nodes[index].file.as_ref().map(|f| ws.root().join(f));
    // A folder's node names its canvas; the folder itself is the directory around it.
    let thing = file.as_deref().map(|f| folder_of(f).unwrap_or(f).to_path_buf());
    // A project's note stays where it is: it is what makes its folder a project.
    let notes: Vec<PathBuf> = ws.projects().0.into_iter().map(|p| p.note).collect();
    let moving = thing.filter(|t| t.parent() == from.parent() && !notes.contains(t));
    let target = dir.join(moving.as_deref().and_then(Path::file_name).unwrap_or_default());
    if let Some(thing) = &moving {
        if dir.starts_with(thing) {
            return Err(Error::Other("A folder can't go into itself".into()));
        }
        if target.exists() {
            return Err(Error::Other(format!("{} already exists", rel(&ws, &target))));
        }
    }

    let mut node = source.nodes.remove(index);
    source.edges.retain(|e| e.from_node != id && e.to_node != id);
    open.wrote(from);
    write_canvas(from, &source)?;

    if let (Some(thing), Some(file)) = (&moving, &file) {
        move_path(open, thing, &target)?;
        let moved = if folder_of(file).is_some() { folder_canvas_path(&target) } else { target.clone() };
        node.file = Some(rel(&ws, &moved));
    }

    let into = folder_canvas_path(dir);
    let mut canvas = if into.exists() { read_canvas(&into)? } else { Default::default() };
    (node.x, node.y) = free_spot(&canvas, node.width, node.height);
    canvas.nodes.push(node);
    open.wrote(&into);
    write_canvas(&into, &canvas)
}

/// Renames the folder `dir` to `name`, its canvas with it. Answers with its new directory.
pub fn rename(open: &Open, dir: &Path, name: &str) -> Result<PathBuf> {
    let ws = open.ws().clone();
    let to = dir.with_file_name(name);
    if to == dir {
        return Ok(to);
    }
    if to.exists() {
        return Err(Error::Other(format!("{} already exists", rel(&ws, &to))));
    }
    let old_canvas = folder_canvas_path(dir);
    move_path(open, dir, &to)?;
    // The canvas inside still has the old name.
    move_path(open, &to.join(old_canvas.file_name().unwrap_or_default()), &folder_canvas_path(&to))?;
    Ok(to)
}

/// Moves a file or directory, and everything that names it: the index, relations and canvases.
fn move_path(open: &Open, from: &Path, to: &Path) -> Result<()> {
    let ws = open.ws().clone();
    open.wrote(from);
    open.wrote(to);
    std::fs::rename(from, to).map_err(|e| Error::Other(format!("{}: {e}", rel(&ws, from))))?;
    workspace::rename_path(&ws, &open.cache(), from, to)?;
    for path in canvas::rename_in_canvases(&ws, &rel(&ws, from), &rel(&ws, to))? {
        open.wrote(&path);
    }
    Ok(())
}
