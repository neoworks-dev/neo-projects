//! PDF and video previews: reading the tools' reports, and making posters with the real tools
//! where they are installed.

use std::path::Path;
use std::process::Command;

use np_core::media::{self, Kind};

fn installed(tool: &str) -> bool {
    Command::new(tool).arg("-v").output().is_ok()
}

/// Two empty A4 pages. Poppler rebuilds the cross-reference table, so it needn't be exact.
const PDF: &str = "%PDF-1.4
1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj
2 0 obj << /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >> endobj
3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >> endobj
4 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >> endobj
trailer << /Root 1 0 R >>
%%EOF
";

#[test]
fn pdfinfo_and_ffprobe_reports_are_read() {
    assert_eq!(media::pdf_pages("Title:  x\nPages:          12\nEncrypted: no\n"), Some(12));
    assert_eq!(media::pdf_pages("Title: x\n"), None);
    assert_eq!(media::video_duration("83.416000\n"), Some(83.416));
    assert_eq!(media::video_duration("N/A\n"), None);
}

#[test]
fn pdfs_and_videos_get_previews_and_nothing_else_does() {
    assert_eq!(media::kind(Path::new("a/report.pdf")), Some(Kind::Pdf));
    assert_eq!(media::kind(Path::new("clip.MP4")), Some(Kind::Video));
    assert_eq!(media::kind(Path::new("clip.webm")), Some(Kind::Video));
    assert_eq!(media::kind(Path::new("clip.avi")), Some(Kind::Video));
    assert_eq!(media::kind(Path::new("clip.mkv")), Some(Kind::Video));
    assert_eq!(media::kind(Path::new("notes.md")), None);
}

#[test]
fn a_pdf_gets_its_first_page_and_page_count() {
    if !installed("pdftoppm") {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("two.pdf");
    std::fs::write(&pdf, PDF).unwrap();
    let previews = dir.path().join("previews");

    let made = media::make(&previews, &pdf).unwrap();
    assert_eq!(made.error, None);
    assert_eq!(made.pages, Some(2));
    assert!(previews.join(made.poster.clone().unwrap()).exists());
    assert_eq!(media::get(&previews, &pdf), Some(made));
}

#[test]
fn a_video_gets_a_frame_and_its_length() {
    if !installed("ffmpeg") {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let video = dir.path().join("clip.mp4");
    let status = Command::new("ffmpeg")
        .args(["-v", "error", "-f", "lavfi", "-i", "testsrc=duration=2:size=320x240:rate=10"])
        .arg(&video)
        .status()
        .unwrap();
    assert!(status.success());
    let previews = dir.path().join("previews");

    let made = media::make(&previews, &video).unwrap();
    assert_eq!(made.error, None);
    assert!((made.duration.unwrap() - 2.0).abs() < 0.1);
    assert!(previews.join(made.poster.unwrap()).exists());
}

#[test]
fn a_broken_file_keeps_its_error_and_a_changed_file_is_made_again() {
    let dir = tempfile::tempdir().unwrap();
    let pdf = dir.path().join("broken.pdf");
    std::fs::write(&pdf, "not a pdf").unwrap();
    let previews = dir.path().join("previews");

    let made = media::make(&previews, &pdf).unwrap();
    assert!(made.error.is_some());
    assert_eq!(media::get(&previews, &pdf), Some(made));

    std::fs::write(&pdf, "still not a pdf, but longer").unwrap();
    assert_eq!(media::get(&previews, &pdf), None, "a changed file has no preview yet");
}
