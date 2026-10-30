# Neoworks Projects

A visual way to organise your projects, locally or around GitHub. A project is a GitHub
repository, or a local one whose issues are Markdown files in its folder; the app
opens on one canvas of all of them, with the blocks between repositories as arrows. Open a
project and its board is a region of its own canvas: drag an issue out of a column to give it a
place next to your notes, images and links, and draw arrows between anything. Everything syncs
with GitHub both ways, as you.

Your notes and canvases are plain files in a workspace folder (Markdown with front matter and
[JSON Canvas](https://jsoncanvas.org)), so an Obsidian vault works as one. Files are linked where
they live, never copied. GitHub stays the place for issues; the app keeps a cache of them.

## What it does

- **Overview:** every project as a card (open, blocked, in progress, the Status split), arranged
  freely, with cross-repository blocks between them.
- **Project canvas:** the repository's board, laid out by the organization's `Status` issue field
  (Open and Closed without one). Drag cards between columns to change their status, out onto the
  canvas to place them, back onto the board to return them. "+" in a column creates an issue.
- **Notes, files, links:** N or the toolbar for a note, drop files from your file manager,
  add web links. Notes boards group notes by a front matter field.
- **Relationships:** drag the handle of a selected node onto another. Between two issues that is
  a GitHub blocked-by or sub-issue; anything else is kept in `.neoworks-projects/relations.jsonl`.
- **Local projects:** no repository; each issue is `Issues/<n>.md` with its fields in the front
  matter, on the same board and in the same panel as GitHub's.
- **Issue panel:** title, description, Status, Priority, Effort and other issue fields, labels,
  assignees, close and reopen, blocked by and blocking, parent and sub-issues, comments.
- **Sync:** local edits show at once and wait in an outbox until GitHub has them; offline is fine.
  When GitHub and you changed the same field, the panel asks which to keep.

## Running it

Needs Rust, [Bun](https://bun.sh), the [GitHub CLI](https://cli.github.com) logged in
(`gh auth login`, with `gh auth refresh -s project,read:org` for organization fields), and
[svelte-gpui](https://github.com/neoworks-dev/svelte-gpui) and
[extension-system](https://github.com/neoworks-dev/extension-system) checked out next to this
repository.

```sh
bun install
scripts/run.sh                 # builds and opens the window
scripts/run.sh --watch         # reloads the window as you edit ui/
```

The first start asks for a workspace folder. `NP_WORKSPACE=~/Projects` opens one directly; with
several `gh` accounts, set `github.login` in `.neoworks-projects/workspace.json` (or
`NP_GH_USER`) to the one to work as.

## Layout

| Path              | What                                                                                   |
| ----------------- | -------------------------------------------------------------------------------------- |
| `crates/core`     | The cache (`cache.db`), GitHub pull and push, the workspace on disk, the views          |
| `crates/app`      | The window's host: commands for the UI, the sync thread, the file watcher               |
| `ui/`             | The UI in Svelte on svelte-gpui; each pane is an extension-system plugin (`ui/plugins`) |
| `docs/`           | Design notes                                                                            |

`cargo run -p np-core --bin np-sync -- --db cache.db pull owner/repo` drives the sync from a
terminal. `cargo test -p np-core` runs the tests.
