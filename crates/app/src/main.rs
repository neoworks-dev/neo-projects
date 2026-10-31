//! Neoworks Projects: the window. The UI is a Svelte app (`ui/`) run by svelte-gpui; this program
//! owns the workspace, the cache and the GitHub sync, and offers them to the UI as commands.
//!
//! ```text
//! bun build.mjs
//! cargo run --release -p neoworks-projects -- --bundle dist/app.js
//! ```

mod canvas;
mod commands;
mod folders;
mod math;
mod previews;
mod state;
mod sync;

use std::sync::Arc;

use svelte_gpui::gpui::AppContext as _;
use svelte_gpui::{Commands, Elements, Options};
use svelte_gpui_editor::{Editor, NoLanguageService, SharedDocuments, Signals};

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
    // The note sheet writes Markdown in svelte-gpui's editor, whose documents live here.
    let documents = SharedDocuments::new(commands.emitter());
    let commands = documents.register(commands::all(commands, app));
    let math = math::Math::start(options.bundle.with_file_name("mathjax.js"));
    let commands = commands.command("math", move |args| {
        let tex = args["tex"].as_str().ok_or("math needs the tex")?;
        let display = args["display"].as_bool().unwrap_or(false);
        let formula = math.typeset(tex, display)?;
        serde_json::to_value(formula).map_err(|e| e.to_string())
    });
    let signals = Signals::default();
    let elements = Elements::new().register("editor", move |host, cx| {
        let documents = documents.clone();
        let signals = signals.clone();
        cx.new(|cx| Editor::new(documents, NoLanguageService::shared(), &signals, host, cx))
    });
    svelte_gpui::run(options.commands(commands).elements(elements));
}
