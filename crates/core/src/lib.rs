//! Neoworks Projects without the window: `cache.db` and the two-way GitHub sync.
//!
//! The window's Rust host owns a [`Cache`]. GitHub is the truth for issues; the cache mirrors
//! them, and every local edit is written to the mirror and to an outbox in one transaction
//! ([`edit`]). [`github::pull`] brings GitHub's changes in and merges them with what the outbox
//! still holds, field by field; [`github::push`] sends the outbox.
//!
//! Disk is the truth for everything else: [`workspace`] reads and writes notes, canvases, boards,
//! projects and relations, watches the folders, and keeps an index of them in the cache.

pub mod cache;
pub mod edit;
pub mod error;
pub mod fields;
pub mod github;
pub mod highlight;
pub mod media;
pub mod previews;
pub mod views;
pub mod workspace;

pub use cache::Cache;
pub use error::{Error, Result};
