//! The window's side of folders (`np_core::workspace::folders`): each change notes the files it
//! wrote, so the watcher's report of them is dropped.

use std::path::{Path, PathBuf};

use np_core::Result;
use np_core::workspace::folders;

use crate::state::Open;

/// Moves the node `id` from the canvas at `from` onto the canvas of the folder `dir`.
pub fn put(open: &Open, from: &Path, id: &str, dir: &Path) -> Result<()> {
    let ws = open.ws().clone();
    let touched = folders::put(&ws, &open.cache(), from, id, dir)?;
    wrote(open, &touched);
    Ok(())
}

/// Renames the folder `dir` to `name`; answers with its new directory.
pub fn rename(open: &Open, dir: &Path, name: &str) -> Result<PathBuf> {
    let ws = open.ws().clone();
    let (to, touched) = folders::rename(&ws, &open.cache(), dir, name)?;
    wrote(open, &touched);
    Ok(to)
}

fn wrote(open: &Open, paths: &[PathBuf]) {
    for path in paths {
        open.wrote(path);
    }
}
