//! Previews of PDFs, videos and images the window can't draw: a poster image and what to say under
//! it (a PDF's page count, a video's length), made by poppler's `pdftoppm` and `pdfinfo` and by
//! `ffmpeg` and `ffprobe`. Each
//! is kept in the previews folder as `<key>.png` and `<key>.json`, the key hashing the file's path,
//! size and modification time, so a changed file gets a new one. Without the tools, a file keeps
//! its icon card.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::workspace::mime_for;
use crate::{Error, Result};

/// How wide a poster is drawn, in pixels: enough for a card at twice its size.
const POSTER_WIDTH: u32 = 800;

/// The image types the window decodes itself; the others are drawn from a PNG poster.
const DRAWN_IMAGES: [&str; 5] = ["image/png", "image/jpeg", "image/gif", "image/webp", "image/svg+xml"];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Pdf,
    Video,
    Image,
}

/// What kind of preview the file at `path` gets, if any.
pub fn kind(path: &Path) -> Option<Kind> {
    let mime = mime_for(path);
    if mime == "application/pdf" {
        return Some(Kind::Pdf);
    }
    if mime.starts_with("video/") {
        return Some(Kind::Video);
    }
    if mime.starts_with("image/") && !DRAWN_IMAGES.contains(&mime) {
        return Some(Kind::Image);
    }
    None
}

/// A made preview. `poster` is a file name in the previews folder; a failed one keeps its error.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Media {
    pub poster: Option<String>,
    pub pages: Option<u32>,
    /// In seconds.
    pub duration: Option<f64>,
    pub error: Option<String>,
}

/// The name the file's preview is kept under, or none when the file can't be read.
pub fn key(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_nanos();
    let seed = format!("{}\n{}\n{modified}", path.display(), meta.len());
    Some(blake3::hash(seed.as_bytes()).to_hex()[..16].to_string())
}

/// The preview made for the file at `path`, failed or not; none while it hasn't been made.
pub fn get(dir: &Path, path: &Path) -> Option<Media> {
    let text = std::fs::read_to_string(sidecar(dir, &key(path)?)).ok()?;
    serde_json::from_str(&text).ok()
}

/// Makes the preview of the file at `path` and keeps it in `dir`, a failure with its error.
pub fn make(dir: &Path, path: &Path) -> Result<Media> {
    let kind = kind(path).ok_or_else(|| Error::Other(format!("{} has no preview", path.display())))?;
    let key = key(path).ok_or_else(|| Error::Other(format!("can't read {}", path.display())))?;
    std::fs::create_dir_all(dir).map_err(|e| Error::Other(format!("{}: {e}", dir.display())))?;
    let poster = format!("{key}.png");
    let made = match kind {
        Kind::Pdf => pdf(path, &dir.join(&poster)),
        Kind::Video => video(path, &dir.join(&poster)),
        Kind::Image => image(path, &dir.join(&poster)),
    };
    let media = match made {
        Ok(media) => Media { poster: Some(poster), ..media },
        Err(e) => Media { error: Some(e.to_string()), ..Media::default() },
    };
    std::fs::write(sidecar(dir, &key), serde_json::to_string(&media)?)
        .map_err(|e| Error::Other(format!("{key}.json: {e}")))?;
    Ok(media)
}

fn sidecar(dir: &Path, key: &str) -> PathBuf {
    dir.join(format!("{key}.json"))
}

/// The first page as `poster`, and the page count.
fn pdf(path: &Path, poster: &Path) -> Result<Media> {
    let info = run(Command::new("pdfinfo").arg(path))?;
    // pdftoppm adds `.png` to the name it is given.
    let stem = poster.with_extension("");
    run(Command::new("pdftoppm")
        .args(["-png", "-singlefile", "-f", "1", "-l", "1", "-scale-to-x"])
        .arg(POSTER_WIDTH.to_string())
        .args(["-scale-to-y", "-1"])
        .arg(path)
        .arg(&stem))?;
    Ok(Media { pages: pdf_pages(&info), ..Media::default() })
}

/// A frame a tenth of the way in (at most a second, past a black first frame) as `poster`, and
/// the length.
fn video(path: &Path, poster: &Path) -> Result<Media> {
    let probed = run(Command::new("ffprobe")
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "default=noprint_wrappers=1:nokey=1"])
        .arg(path))?;
    let duration = video_duration(&probed);
    let at = duration.map_or(0.0, |d| (d / 10.0).min(1.0));
    run(Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-ss"])
        .arg(format!("{at:.3}"))
        .arg("-i")
        .arg(path)
        .args(["-frames:v", "1", "-vf"])
        .arg(format!("scale='min({POSTER_WIDTH},iw)':-2"))
        .arg(poster))?;
    Ok(Media { duration, ..Media::default() })
}

/// The image as a PNG `poster`, its first frame or layer for the types that hold several.
fn image(path: &Path, poster: &Path) -> Result<Media> {
    run(Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(path)
        .args(["-frames:v", "1", "-vf"])
        .arg(format!("scale='min({POSTER_WIDTH},iw)':-2"))
        .arg(poster))?;
    Ok(Media::default())
}

/// Runs a tool and answers with what it printed; a missing tool or a failure is an error.
fn run(command: &mut Command) -> Result<String> {
    let name = command.get_program().to_string_lossy().into_owned();
    let out = command.output().map_err(|e| Error::Other(format!("couldn't run {name}: {e}")))?;
    if !out.status.success() {
        let said = String::from_utf8_lossy(&out.stderr);
        return Err(Error::Other(format!("{name}: {}", said.trim())));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The `Pages:` line of `pdfinfo`'s report.
pub fn pdf_pages(info: &str) -> Option<u32> {
    let line = info.lines().find(|l| l.starts_with("Pages:"))?;
    line["Pages:".len()..].trim().parse().ok()
}

/// `ffprobe`'s duration, in seconds; `N/A` for a stream without one.
pub fn video_duration(probed: &str) -> Option<f64> {
    let seconds: f64 = probed.trim().parse().ok()?;
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    Some(seconds)
}
