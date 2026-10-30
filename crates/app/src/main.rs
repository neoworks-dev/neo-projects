//! Neoworks Projects: the window. The UI is a Svelte app (`ui/`) run by svelte-gpui; this program
//! owns the workspace, the cache and the GitHub sync, and offers them to the UI as commands.
//!
//! ```text
//! bun build.mjs
//! cargo run --release -p neoworks-projects -- --bundle dist/app.js
//! ```

mod canvas;
mod commands;
mod state;
mod sync;

use std::sync::Arc;

use svelte_gpui::{Commands, Options};

fn main() {
    let options = Options::from_command_line().unwrap_or_else(|| {
        eprintln!("usage: neoworks-projects --bundle dist/app.js");
        std::process::exit(2);
    });
    let commands = Commands::new();
    let app = Arc::new(state::App::new(commands.emitter()));
    if let Some(root) = state::last_workspace()
        && let Err(e) = app.open(&root)
    {
        eprintln!("couldn't open {}: {e}", root.display());
    }
    svelte_gpui::run(options.commands(commands::all(commands, app)));
}
