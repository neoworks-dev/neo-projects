# Architecture and data model — proposal, round 3

A visual project management tool, open source: canvases, boards and a dependency graph over your
notes, documents, images, links, GitHub issues and Projects boards, and AI chats. It is used by a
person first; agents come in through neo. It is not a code editor: repositories appear as their
issues, pull requests and boards, and files from them as documents and images.

Points marked **Decided** came from you. The rest is proposal.

## Decided

- **Disk is the truth** for everything persistent: Markdown notes, canvases, images, documents.
  SQLite is a cache for GitHub and agent state, plus indexes, and can be rebuilt.
- **Files are always linked, never copied.** A file from a project folder shows where it is, and
  a change on either side is the same file.
- **An arrow is a real relationship** between two items: a refined image traces back to its
  original, an issue blocks another.
- **GitHub syncs both ways and writes as you**, including Projects (v2) boards.
- **One device.** Sync between devices is out of scope.
- **Plugins are built in at first.** Third-party plugins come later through the same
  extension-system API.
- **The window's Rust host owns the SQLite cache.** The UI reaches it through `invoke` and `listen`.
- **AI goes through the neo harness**, which runs any model on any harness.
- **Local relationships live in one file**, `.neoworks-projects/relations.jsonl`, whatever their ends are.
- **Generated files go next to their original**, in the linked folder it came from.
- **Every card is a note.** A card made on a canvas is a Markdown file, placed on the canvas as a
  JSON Canvas `file` node, so every card is searchable, linkable and editable on its own.
- **Local boards are needed now**, over notes, next to the GitHub project boards.
- **A repository belongs to one project.**

## Layers

```
 ┌─ UI · Svelte in QuickJS (svelte-gpui) ─────────────────────────────────────────┐
 │  extension-system kernel: slots, keymap, commands, services                    │
 │  plugins: canvas · board · table · graph · notes · files · github · neo · comfy │
 └──────────────────────────── invoke / listen ───────────────────────────────────┘
 ┌─ Rust host (GPUI) ─────────────────────────────────────────────────────────────┐
 │  workspace: read, write and watch files · parse front matter and .canvas        │
 │  relations: the relationship graph, endpoints resolved and kept up to date      │
 │  cache.db (SQLite): GitHub mirror, outbox, file index, search, thumbnails, jobs │
 │  github: GraphQL pull, push from the outbox, Projects v2                        │
 │  neo: sdk-rs presence and tools, so the agent can work with the workspace       │
 └────────────────────────────────────────────────────────────────────────────────┘
      disk (truth)            GitHub (truth for issues, PRs, boards)     neo daemon, ComfyUI
```

## On disk

```
My Workspace/
  .neoworks-projects/         the app's own folder (Neoworks Projects)
    workspace.json            linked roots, connected GitHub account, settings
    relations.jsonl           every local relationship, one per line, sorted, diffable
    cache.db                  SQLite; delete it and it is rebuilt (except unsent GitHub edits)
  Harness/                    a project: a folder with a note of the same name
    Harness.md                front matter: repos (each in this project only), GitHub projects
    Roadmap.canvas            JSON Canvas
    Planning.board            a local board over notes (format below)
    Split the daemon.md       a card made on Roadmap.canvas: a note in the project's folder
    Ideas.md
  Inbox.md
```

A workspace can be an Obsidian vault, and the reverse: notes are plain Markdown with front matter,
and canvases are [JSON Canvas](https://jsoncanvas.org), so Obsidian opens both.

**Linked roots.** `workspace.json` names folders outside the workspace, such as your repository
checkouts: `{"harness": "~/Documents/neoworks/neoworks-harness"}`. A linked file is written as
`harness:docs/design.md`, so moving a folder means changing one line, not every reference.

## Items and how they are identified

Everything that can be on a canvas or at the end of an arrow is an item. Its identity is a URI:

| Item                    | Lives in                                     | Identity                                                |
| ----------------------- | -------------------------------------------- | ------------------------------------------------------- |
| Note                    | a `.md` file in the workspace                | `note:<uuid>`, the `id` in its front matter             |
| File (image, PDF, doc…) | where it is, in the workspace or a linked root | `file:<root>/<path>`                                  |
| Web link                | inside a `.canvas` file (JSON Canvas `link`) | its URL                                                 |
| Issue, pull request     | GitHub, mirrored in `cache.db`               | `https://github.com/<owner>/<repo>/issues/<n>`          |
| Draft issue             | a GitHub project, mirrored                   | `github-project:<owner>/<number>/item/<item id>`        |
| GitHub project          | GitHub, mirrored                             | `https://github.com/orgs/<owner>/projects/<number>`     |
| Repository              | GitHub, mirrored                             | `https://github.com/<owner>/<repo>`                     |
| AI chat                 | the neo daemon's session log                 | `neo:session/<id>`                                      |
| Project                 | a folder with its project note               | the note's `note:<uuid>`                                |

A card is a note: typing on an empty spot of a canvas creates `<title>.md` in the folder of the
canvas's project (for a canvas outside any project, the folder the canvas is in), with its `id`,
and places it as a `file` node. Dragging an existing note onto a
canvas places that note. The canvas never holds text of its own, so JSON Canvas `text` nodes only
appear in canvases made elsewhere; the app offers to turn them into notes.

A project is the folder of its project note. The note's front matter lists its repositories, and
a repository may be listed by one project only, so every issue and pull request belongs to
exactly one project through its repository.

Notes get their `id` in front matter when the app creates them or first links to them, so a
rename keeps their relationships. Files outside the workspace can't carry an id, so they are known
by path. The watcher follows renames while the app runs; a file renamed while it was closed is
found again by its content hash, kept in the cache.

On a canvas, anything that is not a file or text is a JSON Canvas `link` node whose URL is the
item's identity (an issue's GitHub URL, `neo:session/…`). Obsidian then shows it as a link, and
this app draws the issue card or the chat.

## Relationships

```jsonc
// .neoworks-projects/relations.jsonl, one per line
{"id":"r_01J…","type":"derived-from","from":"file:art/logo-v2.png","to":"file:art/logo.png","data":{"job":"comfy.refine","prompt":"…"},"at":"2026-10-30T16:02:11Z"}
{"id":"r_01J…","type":"relates","from":"note:5f1c…","to":"https://github.com/neoworks-dev/harness/issues/51","label":"design for"}
```

| Type           | Stored where                                                                  |
| -------------- | ----------------------------------------------------------------------------- |
| `blocks`       | on GitHub when both ends are issues or PRs (blocked-by); else `relations.jsonl` |
| `parent-of`    | on GitHub when both are issues (sub-issues); else `relations.jsonl`           |
| `derived-from` | `relations.jsonl` (plugins write it: a ComfyUI refinement, a summary). A generated file is written next to its original |
| `relates`      | `relations.jsonl`; drawing an arrow makes one unless you pick another type    |
| `mentions`     | not stored: read from `[[links]]` and URLs in Markdown (backlinks)            |
| `closes`       | not stored: read from a pull request's closing references                     |

A relationship has one home: GitHub when GitHub can hold it, else the file. The relations service
in Rust reads both, keeps an index in the cache and resolves endpoints. An arrow between two items
on a canvas is drawn from the relationship; the `.canvas` file also gets the edge, tagged with the
relationship's id, so other JSON Canvas readers see it too. Deleting the arrow asks: only from
this canvas, or the relationship itself.

## GitHub

**Mirrored in `cache.db`:** repositories, issues, pull requests (summary, checks, review state),
labels, milestones, issue types, the org's issue fields (your Priority and Effort), blocked-by and
sub-issues, Projects v2 boards with their fields (Status, iterations, dates…), items (issues, PRs,
draft issues) with their field values, and views (board, table and roadmap layout, group-by, sort,
filter).

**Pull:** one batched GraphQL query per repository and per project, for what changed since the
last pull. It polls on an interval and when the window gets focus.

**Push:** an edit to a mirrored field writes to an outbox in the cache, in the same transaction as
the mirror. The outbox is sent in order (`updateIssue`, `addBlockedBy`, `addSubIssue`,
`setIssueFieldValue`, `updateProjectV2ItemFieldValue`, `updateProjectV2ItemPosition`, …), with
retries, and survives restarts and being offline. It is the one thing in the cache that is not a
cache: deleting `cache.db` loses unsent edits, so the app refuses to rebuild while it holds any.

**Conflicts:** three-way per field against the last pulled value. A field only one side changed
takes that side; labels and assignees merge as sets; a title or body both sides changed shows on
the card for you to resolve.

**Auth:** your token. `gh auth token` when `gh` is logged in (it needs the `project` scope:
`gh auth refresh -s project`), otherwise GitHub's device flow through the project's OAuth app.
Kept in the system keyring.

**Boards and statuses:** a Projects v2 board is drawn natively as a board view, and dragging a card
between columns sets its Status on GitHub. The statuses (the "workflow") are each project's own
Status field, as GitHub defines them. A note can be put on a board as a draft issue, and a draft
issue can be turned into a real issue (`convertProjectV2DraftIssueItemToIssue`).

## Views

| View     | Shows                                                                   | Stored in                                                     |
| -------- | ----------------------------------------------------------------------- | ------------------------------------------------------------- |
| Canvas   | any items placed freely, arrows between them, groups, nested canvases   | `.canvas` files (JSON Canvas)                                 |
| Board    | a GitHub project's view, or local notes grouped by a front matter field | GitHub (the project's view), or a `.board` file (below)       |
| Table    | the same items as rows with fields                                      | as above                                                      |
| Graph    | items and their relationships, laid out automatically (dependencies)   | not stored; a query                                           |
| Document | a note, with its backlinks and relationships beside it                 | the `.md` file                                                |

### Local boards

A local board shows notes only (issues stay on their GitHub project boards). It is a file that says which notes it shows and which front matter field makes its
columns. Moving a card to another column writes that field into the note; the order within a
column is kept in the board file, since it belongs to the board, not the note.

```yaml
# Harness/Planning.board
filter: { folder: "Harness", has: status }   # which notes
groupBy: status                              # front matter field
columns: [backlog, todo, doing, done]         # the statuses, in order: this board's workflow
order:                                       # manual order per column, by note id
  doing: [5f1c…, 9a02…]
```

Proposal: YAML of our own, not Obsidian's `.base`: Bases has no board layout today, and a file
Obsidian would half-read is worse than one it ignores. To check against the current Bases format
before deciding.

## The cache

| In `cache.db`                    | Rebuilt from                       |
| -------------------------------- | ---------------------------------- |
| GitHub mirror                    | GitHub                             |
| Outbox and conflicts             | nothing: real state, see above     |
| File index: path, hash, mtime, mime, size, dimensions | the disk          |
| Search index (FTS5) over notes, cards, issues | disk and mirror       |
| Relationship index               | `relations.jsonl`, GitHub, Markdown |
| Thumbnails and previews          | the files                          |
| Jobs (a ComfyUI render), progress | nothing; finished jobs are kept as `derived-from` data |
| Chats: session titles, last message | the neo daemon                  |

## Plugins

Built in at first, in the UI's extension-system kernel, the way `neoworks-harness/apps/gui`
mounts its panes. Third-party plugins later use the same services:

- `workspace`: read, write and watch files (through Rust, limited to the workspace and linked roots).
- `relations`: query, create and remove relationships, register relationship types.
- `items`: register item kinds (how a URI scheme is drawn and summarised) and file renderers by MIME type.
- `actions`: things to do with a selection, by item kind and MIME type, with a form for their
  parameters; long ones run as jobs with progress.
- `cache`: a table of its own in `cache.db` for a plugin that mirrors another service.
- `neo`: hand a selection to the agent, open a chat.

**Worked example.** You select `logo.png` on a canvas and choose *Refine with ComfyUI*. A job
runs, either straight against ComfyUI's API or as a task for neo's agent. The result is written
next to the original (`logo-v2.png`, a linked file like any other), a `derived-from` relationship
with the prompt and the job goes into `relations.jsonl`, and the new image is placed next to the
old one with the arrow between them.

## AI

The app is a neo app: it registers with the daemon (sdk-rs) and offers tools to read the
workspace, create notes and cards, relate items and run actions, so the agent can do what you can.
A chat on a canvas is a neo session; the daemon keeps the conversation, the canvas keeps
`neo:session/<id>`. Without neo running, chats and AI actions are unavailable and everything else
works.

## Correction (overrides everything above)

No GitHub Projects (v2): "GitHub project" always meant a **GitHub repo**. A project is a repo.
Boards, draft issues, Projects fields and views are out. A board's columns are the options of the
org's issue field "Status" (single-select, synced like Priority and Effort). Repos without issue
fields (personal repos) get the Open/Closed board. New cards on a board create real issues in the repo.

Built: `crates/core` (cache.db + pull/push), tried live on neoworks-dev/neo-projects on
2026-10-30. A blocked-by change moves the issue's `updatedAt`, so the incremental pull sees it.
The org has no Status field yet; `gh bot` can't create issue fields (no org permission).

## v1 scope

A visual way to organise projects, with GitHub (two-way sync, Projects boards) as the
centerpiece. ComfyUI, AI chats and agent actions come later. The app opens on one canvas of
all projects, with cross-repo relationships between them. Opening a project card enters its own
canvas (nested, like Heptabase); no semantic zoom. A project canvas holds its board as one
region: issues are laid out automatically in its Status columns, and any issue can be dragged
out onto the free canvas, where it keeps a position of its own. Only those dragged-out issues
are stored in the `.canvas`; the board region is a node pointing at the GitHub project view.
A dragged-out issue leaves its column (the column shows "+n on canvas"). A repo without a GitHub
project gets a plain Open/Closed board until one is linked.
An overview project card shows: name, open and blocked counts, the top 2–3 in progress, and a
bar of the Status split. Arrows between project cards are cross-repo blocks, labelled with a
count ("3 blocking").
Clicking an issue opens a side panel: title and body (editable), Status, labels, assignees,
Priority/Effort, blocks and blocked-by, and comments (read and reply) — all in v1.

## Settled in round 3

- The app is called **Neoworks Projects**; its folder in a workspace is `.neoworks-projects/`.
- **Issues stay on GitHub.** Local boards show notes only; issues are on their GitHub project boards.
- **Card notes go into the project's folder.**

The exact file formats, cache tables and plugin services are in [formats.md](formats.md).
