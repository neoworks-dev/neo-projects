//! Folders on a project canvas: a directory with a canvas of its own inside, named after it (see
//! [`folder_canvas_path`]). A file or folder that sits directly in the directory of the canvas it
//! is taken from moves on disk into the directory of the canvas it goes to; anything else (a
//! link, an issue, a file from elsewhere) moves only its node.
//!
//! Each change answers with the files it wrote, so the window can tell its own writes from the
//! watcher's.

use std::path::{Path, PathBuf};

use super::{Canvas, Workspace, folder_canvas_path, folder_of, read_canvas, rename_path, write_canvas};
use crate::{Cache, Error, Result};

/// Moves the node `id` from the canvas at `from` onto the canvas at `into`: a folder's, or back
/// out onto the canvas around it. `at` places it; without, it goes where there is room.
pub fn put(ws: &Workspace, cache: &Cache, from: &Path, id: &str, into: &Path, at: Option<(f64, f64)>) -> Result<Vec<PathBuf>> {
    let Some(dir) = into.parent() else {
        return Err(Error::Other(format!("{} has no folder", ws.rel(into))));
    };
    let mut source = read_canvas(from)?;
    let Some(index) = source.nodes.iter().position(|n| n.id == id) else {
        return Err(Error::Other(format!("no node {id} on {}", ws.rel(from))));
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
            return Err(Error::Other(format!("{} already exists", ws.rel(&target))));
        }
    }

    let mut node = source.nodes.remove(index);
    source.edges.retain(|e| e.from_node != id && e.to_node != id);
    write_canvas(from, &source)?;
    let mut touched = vec![from.to_path_buf()];

    if let (Some(thing), Some(file)) = (&moving, &file) {
        touched.extend(move_path(ws, cache, thing, &target)?);
        let moved = if folder_of(file).is_some() { folder_canvas_path(&target) } else { target.clone() };
        node.file = Some(ws.rel(&moved));
    }

    let mut canvas = if into.exists() { read_canvas(into)? } else { Canvas::default() };
    (node.x, node.y) = match at {
        Some(at) => at,
        None => canvas.free_spot(node.width, node.height),
    };
    canvas.nodes.push(node);
    write_canvas(into, &canvas)?;
    touched.push(into.to_path_buf());
    Ok(touched)
}

/// Renames the folder `dir` to `name`, its canvas with it. Answers with its new directory.
pub fn rename(ws: &Workspace, cache: &Cache, dir: &Path, name: &str) -> Result<(PathBuf, Vec<PathBuf>)> {
    let to = dir.with_file_name(name);
    if to == dir {
        return Ok((to, Vec::new()));
    }
    if to.exists() {
        return Err(Error::Other(format!("{} already exists", ws.rel(&to))));
    }
    let old_canvas = folder_canvas_path(dir);
    let mut touched = move_path(ws, cache, dir, &to)?;
    // The canvas inside still has the old name.
    let inside = to.join(old_canvas.file_name().unwrap_or_default());
    if inside.exists() {
        touched.extend(move_path(ws, cache, &inside, &folder_canvas_path(&to))?);
    }
    Ok((to, touched))
}

/// Moves a file or directory, and everything that names it: the index, relations and canvases.
pub fn move_path(ws: &Workspace, cache: &Cache, from: &Path, to: &Path) -> Result<Vec<PathBuf>> {
    std::fs::rename(from, to).map_err(|e| Error::Other(format!("{}: {e}", ws.rel(from))))?;
    rename_path(ws, cache, from, to)?;
    let mut touched = vec![from.to_path_buf(), to.to_path_buf()];
    touched.extend(rename_in_canvases(ws, &ws.rel(from), &ws.rel(to))?);
    Ok(touched)
}

/// Every canvas in the workspace that names `old` (a workspace-relative path), or a file below it
/// when it is a directory, now names `new`. Answers with the canvases it changed.
pub fn rename_in_canvases(ws: &Workspace, old: &str, new: &str) -> Result<Vec<PathBuf>> {
    let mut touched = Vec::new();
    let below = format!("{old}/");
    for path in ws.files().into_iter().filter(|p| p.extension().is_some_and(|e| e == "canvas")) {
        let Ok(mut canvas) = read_canvas(&path) else { continue };
        let mut changed = false;
        for node in &mut canvas.nodes {
            let Some(file) = node.file.as_deref() else { continue };
            if file == old {
                node.file = Some(new.to_string());
                changed = true;
            } else if let Some(rest) = file.strip_prefix(&below) {
                node.file = Some(format!("{new}/{rest}"));
                changed = true;
            }
        }
        if changed {
            write_canvas(&path, &canvas)?;
            touched.push(path);
        }
    }
    Ok(touched)
}
