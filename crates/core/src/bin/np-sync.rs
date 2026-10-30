//! Try the cache and the GitHub sync from a terminal, without the window.
//!
//!   np-sync [--db FILE] pull OWNER/REPO…        pull with your gh token ($NP_GH_USER picks the account)
//!   np-sync [--db FILE] list OWNER/REPO         the mirrored issues
//!   np-sync [--db FILE] set OWNER/REPO#N FIELD JSON   a local edit (nothing is sent)
//!   np-sync [--db FILE] create OWNER/REPO TITLE  a new issue (sent on push)
//!   np-sync [--db FILE] comment OWNER/REPO#N BODY
//!   np-sync [--db FILE] status                 what waits in the outbox, and the conflicts
//!   np-sync [--db FILE] push                    send the outbox to GitHub, as you

use std::path::PathBuf;

use np_core::github::{self, Client, Http};
use np_core::{Cache, Result};
use serde_json::Value;

fn main() {
    if let Err(e) = run() {
        eprintln!("np-sync: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut db = PathBuf::from("cache.db");
    if args.first().map(String::as_str) == Some("--db") && args.len() > 1 {
        db = PathBuf::from(args.remove(1));
        args.remove(0);
    }
    let mut cache = Cache::open(&db)?;
    let command = args.first().cloned().unwrap_or_default();
    let rest = &args[args.len().min(1)..];
    match command.as_str() {
        "pull" => {
            let client = Client::new(Http::from_gh(std::env::var("NP_GH_USER").ok().as_deref())?);
            for repo in rest {
                let (owner, name) = split_repo(repo)?;
                let report = github::pull(&mut cache, &client, owner, name)?;
                println!("{repo}: {} issues changed, {} conflicts", report.issues, report.conflicts);
            }
        }
        "list" => {
            let (owner, name) = split_repo(rest.first().map(String::as_str).unwrap_or_default())?;
            let repo = cache.repo_id(owner, name)?.ok_or_else(|| err("not pulled yet"))?;
            let mut stmt = cache.conn().prepare(
                "SELECT i.number, i.state, i.title, i.fields,
                   (SELECT count(*) FROM gh_links l WHERE l.kind = 'blocks' AND l.to_node = i.node_id)
                 FROM gh_issues i WHERE i.repo = ?1 ORDER BY i.number DESC",
            )?;
            let rows = stmt.query_map([&repo], |r| {
                Ok((
                    r.get::<_, Option<i64>>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)?,
                ))
            })?;
            for row in rows {
                let (number, state, title, fields, blocked) = row?;
                let fields: Value = serde_json::from_str(&fields).unwrap_or_default();
                let names: Vec<&str> =
                    fields.as_object().into_iter().flatten().filter_map(|(_, v)| v["name"].as_str()).collect();
                let number = number.map_or("new".into(), |n| format!("#{n}"));
                let blocked = if blocked > 0 { format!("  blocked by {blocked}") } else { String::new() };
                println!("{number:>5} {state:<6} {title}  [{}]{blocked}", names.join(", "));
            }
        }
        "set" => {
            let [target, field, value] = rest else { return Err(err("set OWNER/REPO#N FIELD JSON")) };
            let node = issue_node(&cache, target)?;
            cache.set_field(&node, field, serde_json::from_str(value)?)?;
            println!("queued; `np-sync push` sends it");
        }
        "create" => {
            let [repo, title] = rest else { return Err(err("create OWNER/REPO TITLE")) };
            let (owner, name) = split_repo(repo)?;
            let repo = cache.repo_id(owner, name)?.ok_or_else(|| err("not pulled yet"))?;
            println!("queued as {}", cache.create_issue(&repo, title, "")?);
        }
        "comment" => {
            let [target, body] = rest else { return Err(err("comment OWNER/REPO#N BODY")) };
            let node = issue_node(&cache, target)?;
            cache.add_comment(&node, body)?;
            println!("queued");
        }
        "status" => {
            for e in cache.outbox()? {
                let error = e.last_error.map(|x| format!("  ({x})")).unwrap_or_default();
                println!("{:>4} {:<8} {} {} {}{error}", e.seq, e.state, e.op, e.target, e.field.unwrap_or_default());
            }
            for c in cache.conflicts()? {
                println!("conflict {} {}: local {} / GitHub {}", c.target, c.field, c.local, c.remote);
            }
        }
        "push" => {
            let report =
                github::push(&mut cache, &Client::new(Http::from_gh(std::env::var("NP_GH_USER").ok().as_deref())?))?;
            println!("sent {}, failed {}, waiting {}", report.sent, report.failed, report.waiting);
        }
        _ => return Err(err("commands: pull, list, set, create, comment, status, push")),
    }
    Ok(())
}

/// The node id of `OWNER/REPO#N` in the cache.
fn issue_node(cache: &Cache, target: &str) -> Result<String> {
    let (repo, number) = target.split_once('#').ok_or_else(|| err("expected OWNER/REPO#N"))?;
    let (owner, name) = split_repo(repo)?;
    let repo = cache.repo_id(owner, name)?.ok_or_else(|| err("not pulled yet"))?;
    let number: i64 = number.parse().map_err(|_| err("bad issue number"))?;
    cache.issue_id(&repo, number)?.ok_or_else(|| err("no such issue in the cache"))
}

fn split_repo(repo: &str) -> Result<(&str, &str)> {
    repo.split_once('/').ok_or_else(|| err("expected OWNER/REPO"))
}

fn err(message: &str) -> np_core::Error {
    np_core::Error::Other(message.into())
}
