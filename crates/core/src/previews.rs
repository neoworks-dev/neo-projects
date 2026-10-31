//! Link previews: what a web page says about itself in its Open Graph and Twitter card tags, its
//! `<title>` and its icon, fetched once and kept in `link_previews` with the images beside the
//! cache.

use std::io::Read;
use std::path::Path;
use std::time::Duration;

use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use url::Url;

use crate::{Cache, Error, Result};

/// The most of a page or image that is read; past it the page's head is long over.
const MAX_PAGE: u64 = 2 << 20;
const MAX_IMAGE: u64 = 8 << 20;
/// How long a failed fetch waits before it is tried again.
const RETRY_MS: i64 = 24 * 60 * 60 * 1000;

/// What a page's head says about it. URLs are absolute.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Page {
    pub title: Option<String>,
    pub description: Option<String>,
    pub site: Option<String>,
    pub image: Option<String>,
    pub favicon: Option<String>,
}

/// A fetched preview as the cache holds it. `image` and `favicon` are file names in the
/// previews folder.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Preview {
    pub title: Option<String>,
    pub description: Option<String>,
    pub site: Option<String>,
    pub image: Option<String>,
    pub favicon: Option<String>,
}

/// Reads the page at `page_url`'s head. Open Graph tags win over Twitter's, and both over
/// `<title>` and `<meta name="description">`. A page without an icon link gets `/favicon.ico`.
pub fn parse(html: &str, page_url: &str) -> Page {
    let mut meta = Meta::default();
    let mut title = None;
    let mut icons: Vec<(u8, String)> = Vec::new();
    let Ok(dom) = tl::parse(html, tl::ParserOptions::default()) else {
        return Page::default();
    };
    let parser = dom.parser();
    for node in dom.nodes() {
        let Some(tag) = node.as_tag() else { continue };
        let attr = |name: &str| tag.attributes().get(name).flatten().map(|b| b.as_utf8_str().into_owned());
        match tag.name().as_utf8_str().to_ascii_lowercase().as_str() {
            "meta" => {
                let Some(content) = attr("content") else { continue };
                let key = attr("property").or_else(|| attr("name")).unwrap_or_default();
                meta.add(&key.to_ascii_lowercase(), content);
            }
            "title" if title.is_none() => title = Some(tag.inner_text(parser).into_owned()),
            "link" => {
                let rel = attr("rel").unwrap_or_default().to_ascii_lowercase();
                if let Some(href) = attr("href")
                    && let Some(rank) = icon_rank(&rel)
                {
                    icons.push((rank, href));
                }
            }
            _ => {}
        }
    }
    icons.sort_by_key(|(rank, _)| *rank);
    let mut favicon = icons.into_iter().next().map(|(_, href)| href);
    if favicon.is_none() {
        favicon = Some("/favicon.ico".into());
    }
    Page {
        title: clean(meta.og_title.or(meta.twitter_title).or(title)),
        description: clean(meta.og_description.or(meta.twitter_description).or(meta.description)),
        site: clean(meta.site),
        image: absolute(page_url, meta.og_image.or(meta.twitter_image)),
        favicon: absolute(page_url, favicon),
    }
}

#[derive(Default)]
struct Meta {
    og_title: Option<String>,
    twitter_title: Option<String>,
    og_description: Option<String>,
    twitter_description: Option<String>,
    description: Option<String>,
    site: Option<String>,
    og_image: Option<String>,
    twitter_image: Option<String>,
}

impl Meta {
    /// Keeps the first value of each tag, as a page lists its main image first. An empty tag
    /// (rust-lang.org has `og:title=""`) leaves room for the next fallback.
    fn add(&mut self, key: &str, content: String) {
        if content.trim().is_empty() {
            return;
        }
        let slot = match key {
            "og:title" => &mut self.og_title,
            "twitter:title" => &mut self.twitter_title,
            "og:description" => &mut self.og_description,
            "twitter:description" => &mut self.twitter_description,
            "description" => &mut self.description,
            "og:site_name" => &mut self.site,
            "og:image" | "og:image:url" | "og:image:secure_url" => &mut self.og_image,
            "twitter:image" | "twitter:image:src" => &mut self.twitter_image,
            _ => return,
        };
        if slot.is_none() {
            *slot = Some(content);
        }
    }
}

/// Which icon a `<link rel>` names, best first: the plain icon is drawn small, where a touch icon
/// would be scaled down from 180 pixels.
fn icon_rank(rel: &str) -> Option<u8> {
    let words: Vec<&str> = rel.split_ascii_whitespace().collect();
    if words.contains(&"icon") {
        return Some(0);
    }
    if words.contains(&"apple-touch-icon") {
        return Some(1);
    }
    None
}

/// Entities decoded and whitespace collapsed; empty text is none.
fn clean(text: Option<String>) -> Option<String> {
    let text = html_escape::decode_html_entities(&text?).split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return None;
    }
    Some(text)
}

fn absolute(page_url: &str, href: Option<String>) -> Option<String> {
    let href = html_escape::decode_html_entities(href?.trim()).into_owned();
    let url = Url::parse(page_url).ok()?.join(&href).ok()?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return None;
    }
    Some(url.to_string())
}

/// Fetches the page at `url` and the images its head names, writes the images into `dir` and
/// keeps the preview in the cache. A page that can't be read keeps its error instead; a missing
/// image or icon only leaves that part out.
pub fn fetch(cache: &Cache, dir: &Path, url: &str) -> Result<()> {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).build();
    let page = match fetch_page(&agent, url) {
        Ok(page) => page,
        Err(e) => {
            store(cache, url, &Preview::default(), Some(&e.to_string()))?;
            return Err(e);
        }
    };
    std::fs::create_dir_all(dir).map_err(|e| Error::Other(format!("{}: {e}", dir.display())))?;
    let stem = blake3::hash(url.as_bytes()).to_hex()[..16].to_string();
    let image = page.image.as_deref().and_then(|src| fetch_image(&agent, src, dir, &format!("{stem}-image")).ok());
    let favicon = page.favicon.as_deref().and_then(|src| fetch_image(&agent, src, dir, &format!("{stem}-icon")).ok());
    let preview = Preview { title: page.title, description: page.description, site: page.site, image, favicon };
    store(cache, url, &preview, None)
}

fn fetch_page(agent: &ureq::Agent, url: &str) -> Result<Page> {
    let response = get(agent, url)?;
    if !response.content_type().contains("html") {
        return Err(Error::Other(format!("{url} is {}, not a page", response.content_type())));
    }
    let final_url = response.get_url().to_string();
    let mut body = Vec::new();
    response
        .into_reader()
        .take(MAX_PAGE)
        .read_to_end(&mut body)
        .map_err(|e| Error::Other(format!("{url}: {e}")))?;
    Ok(parse(&String::from_utf8_lossy(&body), &final_url))
}

/// Saves the image at `src` as `<name>.<ext>` in `dir`; answers with the file name. Only the
/// formats the window draws are kept, which leaves out `.ico`.
fn fetch_image(agent: &ureq::Agent, src: &str, dir: &Path, name: &str) -> Result<String> {
    let response = get(agent, src)?;
    let ext = match response.content_type() {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        other => return Err(Error::Other(format!("{src} is {other}"))),
    };
    let mut body = Vec::new();
    response
        .into_reader()
        .take(MAX_IMAGE)
        .read_to_end(&mut body)
        .map_err(|e| Error::Other(format!("{src}: {e}")))?;
    let file = format!("{name}.{ext}");
    std::fs::write(dir.join(&file), body).map_err(|e| Error::Other(format!("{file}: {e}")))?;
    Ok(file)
}

fn get(agent: &ureq::Agent, url: &str) -> Result<ureq::Response> {
    agent
        .get(url)
        .set("User-Agent", "Mozilla/5.0 (compatible; neoworks-projects)")
        .set("Accept", "text/html,image/*;q=0.9,*/*;q=0.8")
        .call()
        .map_err(|e| Error::Other(format!("{url}: {e}")))
}

/// Keeps what a fetch of `url` found, or the error it ended with.
pub fn store(cache: &Cache, url: &str, preview: &Preview, error: Option<&str>) -> Result<()> {
    cache.conn().execute(
        "INSERT OR REPLACE INTO link_previews (url, title, description, site, image, favicon, error, fetched)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            url,
            preview.title,
            preview.description,
            preview.site,
            preview.image,
            preview.favicon,
            error,
            now_ms()
        ],
    )?;
    Ok(())
}

/// The preview fetched for `url`; none while it hasn't been, or when the fetch failed.
pub fn get_preview(cache: &Cache, url: &str) -> Result<Option<Preview>> {
    Ok(cache
        .conn()
        .query_row(
            "SELECT title, description, site, image, favicon FROM link_previews WHERE url = ?1 AND error IS NULL",
            [url],
            |r| {
                Ok(Preview {
                    title: r.get(0)?,
                    description: r.get(1)?,
                    site: r.get(2)?,
                    image: r.get(3)?,
                    favicon: r.get(4)?,
                })
            },
        )
        .optional()?)
}

/// Whether `url` should be fetched: it never was, or its last try failed a while ago.
pub fn wanted(cache: &Cache, url: &str) -> Result<bool> {
    let last: Option<(Option<String>, i64)> = cache
        .conn()
        .query_row("SELECT error, fetched FROM link_previews WHERE url = ?1", [url], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    let Some((error, fetched)) = last else {
        return Ok(true);
    };
    Ok(error.is_some() && now_ms() - fetched > RETRY_MS)
}

fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}
