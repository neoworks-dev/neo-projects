-- cache.db, version 1. Everything here is rebuilt from GitHub except outbox and conflicts.
-- GitHub ids are GraphQL node ids; an issue created here and not yet pushed is `local:<uuid>`.

CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);

CREATE TABLE gh_repos (
  node_id TEXT PRIMARY KEY,
  owner TEXT NOT NULL,
  name TEXT NOT NULL,
  url TEXT NOT NULL,
  owner_kind TEXT NOT NULL,              -- Organization or User: only organizations have issue fields
  archived INTEGER NOT NULL DEFAULT 0,
  pulled_at TEXT,
  pulled_since TEXT,                     -- updatedAt of the newest issue pulled
  UNIQUE (owner, name)
);

CREATE TABLE gh_labels (node_id TEXT PRIMARY KEY, repo TEXT NOT NULL, name TEXT NOT NULL, color TEXT);
CREATE TABLE gh_users (node_id TEXT PRIMARY KEY, login TEXT NOT NULL);
CREATE TABLE gh_repo_users (repo TEXT NOT NULL, user TEXT NOT NULL, PRIMARY KEY (repo, user));

-- An organization's issue fields (Priority, Effort, Status…), options as JSON.
CREATE TABLE gh_fields (
  node_id TEXT PRIMARY KEY,
  owner TEXT NOT NULL,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,                    -- single_select, text, date, number
  options TEXT NOT NULL DEFAULT '[]'
);

CREATE TABLE gh_issues (
  node_id TEXT PRIMARY KEY,
  repo TEXT NOT NULL,
  number INTEGER,                        -- null until a local issue is pushed
  url TEXT,
  title TEXT NOT NULL DEFAULT '',
  body TEXT NOT NULL DEFAULT '',
  state TEXT NOT NULL DEFAULT 'OPEN',
  state_reason TEXT,
  author TEXT,
  issue_type TEXT,
  labels TEXT NOT NULL DEFAULT '[]',     -- [{id, name, color}], sorted by id
  assignees TEXT NOT NULL DEFAULT '[]',  -- [{id, login}], sorted by id
  milestone TEXT,
  fields TEXT NOT NULL DEFAULT '{}',     -- field id -> value
  comment_count INTEGER NOT NULL DEFAULT 0,
  created TEXT,
  updated TEXT
);
CREATE INDEX gh_issues_repo ON gh_issues (repo, number);

-- blocks: from blocks to. parent: from is the parent of to (sub-issues).
CREATE TABLE gh_links (
  kind TEXT NOT NULL,
  from_node TEXT NOT NULL,
  to_node TEXT NOT NULL,
  PRIMARY KEY (kind, from_node, to_node)
);
CREATE INDEX gh_links_to ON gh_links (to_node, kind);

-- Issues named by a link whose repository may not be mirrored: enough to draw them.
CREATE TABLE gh_refs (
  node_id TEXT PRIMARY KEY,
  repo TEXT NOT NULL,                    -- owner/name
  number INTEGER NOT NULL,
  title TEXT NOT NULL,
  state TEXT NOT NULL,
  url TEXT NOT NULL
);

CREATE TABLE gh_comments (
  node_id TEXT PRIMARY KEY,
  issue TEXT NOT NULL,
  author TEXT,
  body TEXT NOT NULL,
  created TEXT,
  updated TEXT
);
CREATE INDEX gh_comments_issue ON gh_comments (issue, created);

-- The value of each synced field as GitHub last had it: the base of three-way merges.
CREATE TABLE gh_base (
  node_id TEXT NOT NULL,
  field TEXT NOT NULL,
  value TEXT NOT NULL,
  PRIMARY KEY (node_id, field)
);

-- Edits waiting for GitHub. Not a cache: deleting it loses them.
-- op: set (field gets value), create (value: {repo, title, body}), comment (value: {id, body}).
CREATE TABLE outbox (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  created TEXT NOT NULL,
  target TEXT NOT NULL,
  op TEXT NOT NULL,
  field TEXT,
  value TEXT NOT NULL,
  state TEXT NOT NULL DEFAULT 'waiting', -- waiting, failed, conflict
  attempts INTEGER NOT NULL DEFAULT 0,
  last_error TEXT
);
CREATE INDEX outbox_target ON outbox (target, field);

-- A field both sides changed. The local value stays in the mirror until it is resolved.
CREATE TABLE conflicts (
  target TEXT NOT NULL,
  field TEXT NOT NULL,
  base TEXT,
  local TEXT NOT NULL,
  remote TEXT NOT NULL,
  detected TEXT NOT NULL,
  PRIMARY KEY (target, field)
);
