//! The preview thread: fetches the pages of web links one at a time, with a connection of its own,
//! and tells the window when a preview arrives so the link's card fills in.

use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};

use np_core::workspace::Workspace;
use np_core::{Cache, Result, previews};
use serde_json::{Value, json};
use svelte_gpui::Emitter;

pub struct Previews {
    tx: Sender<String>,
}

impl Previews {
    pub fn start(ws: &Workspace, emitter: Emitter) -> Result<Previews> {
        let (tx, rx) = mpsc::channel::<String>();
        let cache = Cache::open(&ws.cache_path())?;
        let dir = dir(ws);
        std::thread::spawn(move || {
            for url in rx {
                // A URL asked for twice is fetched once: the first fetch leaves its row.
                if !previews::wanted(&cache, &url).unwrap_or(false) {
                    continue;
                }
                if previews::fetch(&cache, &dir, &url).is_ok() {
                    emitter.emit("changed", json!({"reason": "previews"}));
                }
            }
        });
        Ok(Previews { tx })
    }

    /// Asks for the preview of every web link among a view's nodes.
    pub fn want_view(&self, view: &Value) {
        for node in view["nodes"].as_array().into_iter().flatten() {
            let item = &node["item"];
            if item["kind"] == "link"
                && let Some(url) = item["uri"].as_str()
            {
                self.want(url);
            }
        }
    }

    /// Asks for the preview of `url` if it is a web page.
    pub fn want(&self, url: &str) {
        if url.starts_with("http://") || url.starts_with("https://") {
            let _ = self.tx.send(url.to_string());
        }
    }
}

/// Where the preview images are kept: `.neoworks-projects/previews`.
pub fn dir(ws: &Workspace) -> PathBuf {
    ws.dir().join("previews")
}
