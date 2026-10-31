//! The preview thread: fetches the pages of web links and makes the posters of PDFs, videos and
//! images, one at a time, with a connection of its own, and tells the window when a preview is
//! ready so its card fills in.

use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};

use np_core::workspace::Workspace;
use np_core::{Cache, Result, media, previews};
use serde_json::{Value, json};
use svelte_gpui::Emitter;

enum Job {
    Link(String),
    Media(PathBuf),
}

pub struct Previews {
    tx: Sender<Job>,
}

impl Previews {
    pub fn start(ws: &Workspace, emitter: Emitter) -> Result<Previews> {
        let (tx, rx) = mpsc::channel::<Job>();
        let cache = Cache::open(&ws.cache_path())?;
        let dir = dir(ws);
        std::thread::spawn(move || {
            // A job asked for twice is done once: the first leaves its row or its file.
            for job in rx {
                let done = match job {
                    Job::Link(url) => previews::wanted(&cache, &url).unwrap_or(false)
                        && previews::fetch(&cache, &dir, &url).is_ok(),
                    Job::Media(path) => media::get(&dir, &path).is_none() && media::make(&dir, &path).is_ok(),
                };
                if done {
                    emitter.emit("changed", json!({"reason": "previews"}));
                }
            }
        });
        Ok(Previews { tx })
    }

    /// Asks for the preview of every web link, PDF and video among a view's nodes.
    pub fn want_view(&self, view: &Value) {
        for node in view["nodes"].as_array().into_iter().flatten() {
            let item = &node["item"];
            if item["kind"] == "link"
                && let Some(url) = item["uri"].as_str()
            {
                self.want(url);
            }
            if item["kind"] == "file"
                && item["media"].is_null()
                && let Some(path) = item["path"].as_str()
            {
                self.want_media(PathBuf::from(path));
            }
        }
    }

    /// Asks for the preview of `url` if it is a web page.
    pub fn want(&self, url: &str) {
        if url.starts_with("http://") || url.starts_with("https://") {
            let _ = self.tx.send(Job::Link(url.to_string()));
        }
    }

    /// Asks for the poster of the file at `path` if it gets one.
    fn want_media(&self, path: PathBuf) {
        if media::kind(&path).is_some() {
            let _ = self.tx.send(Job::Media(path));
        }
    }
}

/// Where the preview images are kept: `.neoworks-projects/previews`.
pub fn dir(ws: &Workspace) -> PathBuf {
    ws.dir().join("previews")
}
