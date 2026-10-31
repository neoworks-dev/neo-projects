//! Link previews: reading a page's head, and what the cache keeps of a fetch.

use np_core::Cache;
use np_core::previews::{self, Page, Preview};

#[test]
fn open_graph_tags_win_over_twitter_and_the_title() {
    let html = r#"<html><head>
        <title>Plain title</title>
        <meta name="twitter:title" content="Twitter title">
        <meta property="og:title" content="Open Graph &amp; friends">
        <meta name="description" content="  A page
            about things.  ">
        <meta property="og:site_name" content="Example">
        <meta name="twitter:image" content="/twitter.png">
        <meta property="og:image" content="/images/cover.png">
        <meta property="og:image" content="/images/second.png">
        <link rel="apple-touch-icon" href="/touch.png">
        <link rel="shortcut icon" href="icon.svg">
    </head><body></body></html>"#;
    assert_eq!(
        previews::parse(html, "https://example.com/blog/post"),
        Page {
            title: Some("Open Graph & friends".into()),
            description: Some("A page about things.".into()),
            site: Some("Example".into()),
            image: Some("https://example.com/images/cover.png".into()),
            favicon: Some("https://example.com/blog/icon.svg".into()),
        }
    );
}

#[test]
fn a_bare_page_falls_back_to_its_title_and_favicon_ico() {
    let html = "<html><head><title> Hello </title></head><body><p>Hi</p></body></html>";
    assert_eq!(
        previews::parse(html, "https://example.com/a/b"),
        Page {
            title: Some("Hello".into()),
            description: None,
            site: None,
            image: None,
            favicon: Some("https://example.com/favicon.ico".into()),
        }
    );
}

#[test]
fn an_empty_open_graph_title_falls_back_to_the_title() {
    let html = r#"<title>Rust Programming Language</title><meta property="og:title" content="" />"#;
    assert_eq!(previews::parse(html, "https://www.rust-lang.org/").title, Some("Rust Programming Language".into()));
}

#[test]
fn image_urls_that_are_not_on_the_web_are_dropped() {
    let html = r#"<meta property="og:image" content="data:image/png;base64,AAAA">"#;
    assert_eq!(previews::parse(html, "https://example.com/").image, None);
}

#[test]
fn a_url_is_fetched_once_and_a_failure_shows_no_preview() {
    let cache = Cache::open_in_memory().unwrap();
    let ok = "https://example.com/";
    let broken = "https://broken.example/";
    assert!(previews::wanted(&cache, ok).unwrap());

    let preview = Preview { title: Some("Example".into()), image: Some("abc-image.png".into()), ..Default::default() };
    previews::store(&cache, ok, &preview, None, None).unwrap();
    previews::store(&cache, broken, &Preview::default(), Some("connection refused"), None).unwrap();

    assert!(!previews::wanted(&cache, ok).unwrap());
    assert!(!previews::wanted(&cache, broken).unwrap(), "a fresh failure waits before the next try");
    assert_eq!(previews::get_preview(&cache, ok).unwrap(), Some(preview));
    assert_eq!(previews::get_preview(&cache, broken).unwrap(), None);
}

#[test]
fn a_page_whose_image_failed_is_fetched_again_a_day_later() {
    let cache = Cache::open_in_memory().unwrap();
    let url = "https://github.com/neoworks-dev/neo-projects";
    let preview = Preview { title: Some("neo-projects".into()), ..Default::default() };
    previews::store(&cache, url, &preview, None, Some("429 Too Many Requests")).unwrap();

    assert!(!previews::wanted(&cache, url).unwrap(), "a fresh failure waits before the next try");
    assert_eq!(previews::get_preview(&cache, url).unwrap(), Some(preview), "the page still shows meanwhile");

    let two_days = 2 * 24 * 60 * 60 * 1000;
    cache.conn().execute("UPDATE link_previews SET fetched = fetched - ?1", [two_days]).unwrap();
    assert!(previews::wanted(&cache, url).unwrap());
}

#[test]
fn a_version_3_cache_gains_the_image_error_column() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache.db");
    let cache = Cache::open(&path).unwrap();
    cache.conn().execute_batch("ALTER TABLE link_previews DROP COLUMN image_error; PRAGMA user_version = 3;").unwrap();
    drop(cache);

    let cache = Cache::open(&path).unwrap();
    previews::store(&cache, "https://example.com/", &Preview::default(), None, Some("timed out")).unwrap();
    assert!(!previews::wanted(&cache, "https://example.com/").unwrap());
}

#[test]
fn a_version_2_cache_gains_the_previews_table() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cache.db");
    let cache = Cache::open(&path).unwrap();
    cache.conn().execute_batch("DROP TABLE link_previews; PRAGMA user_version = 2;").unwrap();
    drop(cache);

    let cache = Cache::open(&path).unwrap();
    assert!(previews::wanted(&cache, "https://example.com/").unwrap());
}
