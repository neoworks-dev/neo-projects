## Code style

Code here is read far more often than it is written, so write the obvious
version rather than the clever one.

Names say what they hold, in camelCase unless the language or framework has its
own convention (snake_case in Rust): `recordId`, `customerAccount`,
`paymentMethod`.

Spell control flow out with `if`. Use `??`, `?:` or `&&` only when the explicit
form would repeat a non-trivial expression, or when avoiding `??` would mean
moving code out of its context into a helper.

```ts
let displayName = contact.nickname;
if (!displayName) {
  displayName = contact.fullName;
}
```

Keep function bodies at most two levels deep, with guard clauses and early
returns keeping the main path flat. Extract a helper when a piece has its own
name and job; a fragment cut out only to satisfy the limit makes the code
harder to follow.

Each function does one thing and stays short. A value used once is written
where it is used. Hoist it only when that saves repeated work, like a regex
compiled once at module level.

Comments explain what skimming the code won't tell you, in technical terms, and
describe the code as it is now. Git holds the history.

```ts
// The host rasterises a Canvas on the CPU and uploads it whole on every change.
```

A refactor keeps behaviour, public APIs, data shapes, validation rules and side
effects exactly as they were. When it can't, ask before changing them.

Copy the surface conventions of surrounding code: casing, quoting, import order,
formatting. Its structure is no precedent. When an existing pattern looks like
a mistake, ask me or check the framework's documentation.

There are no users yet. When the cache schema (`crates/core/src/schema*.sql`)
or a file format in `docs/formats.md` changes, ask whether existing caches and
workspaces should be rebuilt or migrated, and write the code for that.

Edit files with Read, Write and Edit rather than sed, awk or python.

## Design system

UI comes from the NeoWorks design system, `@neoworks/svelte-gpui-ui`, which
lives in `../svelte-gpui/packages/svelte-gpui-ui` and is aliased by
`build.mjs` and `tsconfig.json`. `ui/lib/theme.ts` maps its dark theme onto the
app's colours. Use its components first. When none fits and you build a new
one, say which ones you considered and why they didn't fit. Its examples, as
they look in a screen, are in `../neoworks.dev/packages/ui-gpui/examples`.

Change the design system or svelte-gpui itself only when I ask. If either
doesn't resolve, tell me and stop rather than copying components into the app.

## Running and screenshots

`scripts/run.sh` builds and opens the window. To see a change without touching
my screens, run the app on the offscreen display with
`../svelte-gpui/scripts/shot.sh` (see `../svelte-gpui/docs/testing.md` for the
`--do` steps), with `HOST=target/release/neoworks-projects`,
`NP_WORKSPACE=<a scratch workspace>` and `NP_GH_USER=Letsmoe`. Look at the
screenshot before saying a UI change works.

## Tests

Run type checks and tests once a stretch of work is done, before committing:

1. Add a test for every behaviour change and bug fix: Rust in
   `crates/*/tests`, UI logic kept free of Svelte (like
   `ui/lib/canvas/geometry.ts`) with a `bun test`. Refactors and style edits
   need none; say so in one sentence.
2. Run `cargo test -p np-core` and `bunx svelte-check --tsconfig ./tsconfig.json`,
   whether or not you wrote a test, and `cargo clippy --all-targets` on what
   you touched.
3. When a test fails, fix the code. Change the test only when the test itself
   is wrong, and then say why.

## Git

If the tree is dirty when you start, commit what's there first, so your changes
stand alone. Write a short title, and add a body when the title doesn't carry
the change. Ask if you're unsure what to write. On a feature branch, commit
only what that branch is for. Commits and PRs end with their last sentence:
no `Co-Authored-By` or "Generated with Claude Code" lines.

## GitHub

The repository is `neoworks-dev/neo-projects`. My account is Letsmoe; `gh`
holds more than one, so pick it explicitly (`gh auth token --user Letsmoe`,
`NP_GH_USER=Letsmoe`). The app writes to GitHub as me. Everything the model
writes to GitHub itself (issues, comments, PRs, labels) goes through `gh bot`,
as the parent folder's CLAUDE.md says.

## Writing

Commits and issues carry only what matters: say the thing, explain what a
reader won't see for themselves, and stop.

Write issues and their comments as you'd explain them to a colleague, in
complete sentences.

- Open with a 2–3 sentence summary of what happened, why, and the fix.
- Use short headings with short paragraphs of prose under them.
- Join ideas with words: "first X, then Y" rather than "X → Y", and "A and B"
  rather than "A / B".
- Use at most two code identifiers per sentence, and say what each one does the
  first time it appears.
- Use bullets for genuinely separate items, each a full sentence, and tables
  for numbers or timelines.
- Write dates as 2026-10-07 18:28.
- Put error messages, paths and commands in code formatting, and long logs in a
  collapsed `<details>` block.
- End with a short "Out of scope" section for side findings.

## How we work

I want to stay on top of everything that happens in this repo, so the work is
coordinated with me as it goes rather than done around me. One agent at a time,
with me giving feedback. Keep each step small enough for me to read in one
sitting, and stop to show me before the work piles up.

When the next step is a decision (an approach, a fix for something unexpected,
cleaning up or deleting something, splitting work up), bring it to me with
your recommendation and let me choose. When something turns out different from
what we expected, tell me before working around it.

`main` is what I've reviewed. Work happens in the repo root:

```sh
git switch -c 12-local-projects main   # issue 12
git switch -c canvas-performance main  # no issue
```

Typos, one-liners, small fixes and changes to how we work (this file,
`.claude/`, editor config) go straight to `main`. Everything else gets a
feature branch. Commit or stash before switching, so nothing carries over.

Sometimes I start without a plan and work towards something as we go. That
stays on one branch however it grows. When the work has a clear goal from the
start, the branch is that one thing, and unrelated findings become issues.

Before starting, check whether the work is already half-built with
`git branch` and `git worktree list`, and read what's on the branch. Sessions
end mid-feature; continuing on an existing branch keeps what the first attempt
learned.

Review happens here on the diff, so open a pull request only when I ask.

Worktrees are only for several agents editing at the same time:
`git worktree add .worktrees/<branch> -b <branch> main`.

## Issues

Issues on `neoworks-dev/neo-projects` carry goals across sessions, since a
session's context ends with it. Work that won't finish in this session gets an
issue, however loosely defined, and so does anything concrete found along the
way that isn't what we're doing now. Write them as soon as the list exists.

Every issue gets a type, priority and effort (GitHub's issue type and the
organization's issue fields):

```sh
gh bot issue create --type Bug --priority High --effort Low …
gh bot issue edit 12 --priority Urgent
```

- **Type**: `Bug` broken, `Feature` new, `Task` everything else.
- **Priority**: `Urgent` drop everything, `High` next up, `Medium` normal,
  `Low` nice to have.
- **Effort**: `Low` an hour or two, `Medium` about a day, `High` several days
  or needs splitting.

## Merging

A branch merges when I say so, after I've reviewed its diff, and only with the
tests green on the branch:

```sh
git switch main
git merge --no-ff 12-local-projects   # resolve conflicts in the merge commit
git push                              # never force
git branch -d 12-local-projects
gh bot issue close 12
```
