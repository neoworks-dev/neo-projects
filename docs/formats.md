# Neoworks Projects — formats

Draft. Ids are UUIDv7. Times: ISO on disk, Unix ms in the cache.

## Identifiers

`note:<uuid>` · `file:<root>/<path>` (`workspace` or a linked root) · GitHub URL ·
`neo:session/<id>` · web URL

## `.neoworks-projects/workspace.json`

```jsonc
{ "version": 1,
  "roots": { "harness": "~/Documents/neoworks/neoworks-harness" },
  "github": { "login": "…", "pollSeconds": 120 },   // token lives in the keyring
  "neo": { "enabled": true } }
```

## Note front matter

```yaml
id: 0192f5c4-…          # set by the app
type: project           # project notes only:
repos: [neoworks-dev/harness]
roots: [harness]
```

Everything else is yours. A repo in two projects is an error. A project without `repos` is local;
`statuses: [Backlog, Todo, In Progress, Done]` (the default) are its board's columns, the last
one done.

## Local issues

`<project>/Issues/<number>.md`; the body is the description. Node id in the window:
`issue:<workspace-relative path>`.

```yaml
id: 01a18e09-…
type: issue
number: 2
title: Plant tomatoes
status: In Progress
state: closed            # absent while open
closed: done             # or not-planned
priority: High           # Urgent, High, Medium, Low
effort: Low              # High, Medium, Low
labels: [bug]
assignees: [Letsmoe]
blockedBy: [1]           # numbers in the same project
parent: 3
created: 2026-10-30T18:19:06Z
comments: [{"author": "Letsmoe", "created": "…", "body": "…"}]
```

## `relations.jsonl`

```jsonc
{"id":"…","type":"derived-from","from":"file:workspace/Harness/logo-v2.png","to":"file:workspace/Harness/logo.png","data":{"prompt":"…"},"created":"…","by":"plugin:comfy"}
```

Types: `relates`, `blocks`, `parent-of`, `derived-from`, `<plugin>:<name>`. Issue↔issue
`blocks`/`parent-of` go to GitHub instead.

## `.canvas` (JSON Canvas)

Notes and workspace files are `file` nodes; linked files, issues, chats and links are `link`
nodes with their URL. Extras go under `"neoworks": {…}` (e.g. the edge's relation id), as a hint only.

A folder is a directory in the project with a canvas inside named after it, and a `file` node
pointing at that canvas: `Garden/Seeds/Seeds.canvas`. The folder's canvas has no board.

Lines drawn on the background go under the canvas's own `neoworks` object, which other JSON
Canvas apps keep but don't draw. `points` runs x, y, x, y, … in canvas coordinates:

```json
"neoworks": {
  "strokes": [{ "id": "de2c83b106e7b", "color": "#f85149", "width": 2.5, "points": [0, 0, 12.5, 4] }]
}
```

## `.board`

```yaml
filter: { folder: Harness, has: status }
groupBy: status
columns: [backlog, todo, doing, done]
order: { doing: [<note id>, …] }
```

## `cache.db`

- **Index:** `files`, `notes`, `search` (FTS5), `thumbs`, `relations`
- **GitHub mirror:** `gh_repos`, `gh_issues`, `gh_comments`
- **Sync:** `gh_base`, `outbox`, `conflicts` (the only tables that can't be rebuilt)
- **Link previews:** `link_previews`, with their images in `.neoworks-projects/previews/`. A
  failed page (`error`) or a failed image or icon (`image_error`) is fetched again after a day.

PDF and video posters are not in `cache.db`: each is `<key>.png` with a `<key>.json` beside it
(page count or length, or the error) in `.neoworks-projects/previews/`, the key hashing the
file's path, size and modification time.
- **Other:** `jobs`, `chats`

## Plugin services

`workspace` (files and notes) · `relations` · `items` (how an item kind is drawn) ·
`actions` (what a selection can do) · `cache` (a plugin's own tables) · `neo` (chats, tasks)
