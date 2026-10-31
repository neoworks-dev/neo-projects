//! Text and code files as the canvas shows them: a sheet of their first lines, highlighted by the
//! window through [`crate::highlight`].

use std::io::Read;
use std::path::Path;

use crate::workspace::mime_for;

/// How much of a file is read for its sheet, so a large log costs no more than a small one.
const READ_BYTES: u64 = 16 * 1024;
/// How many lines a sheet shows at most, and how much of each: a minified file is one long line.
const LINES: usize = 60;
const LINE_CHARS: usize = 200;

/// Whether the file at `path` is shown as a sheet: plain text and code, but not notes, which
/// have a card of their own.
pub fn is_sheet(path: &Path) -> bool {
    let mime = mime_for(path);
    if mime == "text/markdown" {
        return false;
    }
    mime.starts_with("text/") || ["application/json", "application/yaml", "application/toml"].contains(&mime)
}

/// The grammar the window highlights the file with: its extension, or none for plain text.
pub fn lang(path: &Path) -> String {
    let mime = mime_for(path);
    if mime == "text/plain" || mime == "text/csv" {
        return String::new();
    }
    path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// The first lines of the file at `path`, or none when it can't be read or isn't text.
pub fn head(path: &Path) -> Option<String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path).ok()?.take(READ_BYTES).read_to_end(&mut bytes).ok()?;
    if bytes.contains(&0) {
        return None;
    }
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text.lines().take(LINES).map(|line| line.chars().take(LINE_CHARS).collect()).collect();
    Some(lines.join("\n"))
}
