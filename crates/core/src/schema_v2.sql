-- cache.db, version 2: the index of the disk. Everything here is rebuilt from the workspace.

-- Every file in the workspace and the linked roots. hash is blake3 (hex); mtime is Unix ms.
CREATE TABLE files (
  uri TEXT PRIMARY KEY,                  -- file:<root>/<percent-encoded path>
  root TEXT NOT NULL,
  path TEXT NOT NULL,                    -- below the root, `/` separated
  hash TEXT NOT NULL,
  size INTEGER NOT NULL,
  mtime INTEGER NOT NULL,
  mime TEXT NOT NULL,
  note_id TEXT                           -- the front matter id of a note
);
CREATE INDEX files_hash ON files (hash);

-- Markdown notes. id is the front matter id, or the file's uri while it has none.
CREATE TABLE notes (
  id TEXT PRIMARY KEY,
  uri TEXT NOT NULL,
  path TEXT NOT NULL,                    -- absolute
  title TEXT NOT NULL,
  props TEXT NOT NULL DEFAULT '{}',      -- front matter as JSON
  project TEXT,                          -- name of the project folder it is in
  updated TEXT
);
CREATE INDEX notes_uri ON notes (uri);
CREATE INDEX notes_project ON notes (project);

-- Local relationships from relations.jsonl (source 'file'). GitHub ones join them later.
CREATE TABLE relations (
  id TEXT PRIMARY KEY,
  type TEXT NOT NULL,
  from_uri TEXT NOT NULL,
  to_uri TEXT NOT NULL,
  label TEXT,
  data TEXT,
  source TEXT NOT NULL DEFAULT 'file'
);
CREATE INDEX relations_from ON relations (from_uri, type);
CREATE INDEX relations_to ON relations (to_uri, type);

CREATE VIRTUAL TABLE search USING fts5 (uri UNINDEXED, title, body);
