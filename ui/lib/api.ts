// The window's Rust commands, typed. Every read and write of the workspace and the GitHub mirror
// goes through here; the host answers on a thread and never blocks the window.
import { invoke, listen } from "@neoworks/svelte-gpui";

export interface Label {
  name: string;
  color: string | null;
}

/** One issue as a board column, a canvas node or a search result shows it. */
export interface Card {
  node: string;
  repo: string;
  repoName: string;
  number: number | null;
  title: string;
  state: "OPEN" | "CLOSED";
  stateReason: string | null;
  labels: Label[];
  assignees: string[];
  fields: Record<string, string | number | null>;
  blockedBy: number;
  blocking: number;
  comments: number;
  /** The start of the description, as Markdown. */
  excerpt: string;
  url: string | null;
  updated: string | null;
  pending: boolean;
  conflict: boolean;
  /** A local issue: a Markdown file in its project's Issues folder, `node` being `issue:<path>`. */
  local?: boolean;
}

export interface Column {
  key: string;
  name: string;
  color: string | null;
  done?: boolean;
  cards: Card[];
}

export interface Board {
  statusField: string | null;
  columns: Column[];
}

export interface Summary {
  open: number;
  closed: number;
  blocked: number;
  hasStatus: boolean;
  split: { name: string; color: string | null; count: number }[];
  inProgress: { node: string; number: number; title: string }[];
  pulledAt: string | null;
}

export interface Project {
  id: string | null;
  name: string;
  folder: string;
  note: string;
  repo: string | null;
  repoId: string | null;
  /** No repository: its issues are local files. */
  local: boolean;
  /** The project's folder on disk. */
  abs: string;
  /** Folders outside the workspace the project works in, by their root name. */
  roots: { name: string; path: string }[];
  summary: Summary | null;
}

/** A node of a JSON Canvas, as stored, plus what the host found it to be. */
export interface CanvasNode {
  id: string;
  type: "text" | "file" | "link" | "group";
  x: number;
  y: number;
  width: number;
  height: number;
  file?: string;
  url?: string;
  text?: string;
  label?: string;
  color?: string;
  neoworks?: Record<string, unknown>;
  item?: Item;
  [key: string]: unknown;
}

/** What a canvas node stands for. */
export type Item =
  | { kind: "note"; uri: string | null; path: string; abs: string; title: string; excerpt: string }
  | { kind: "image"; uri: string | null; path: string; title: string }
  | { kind: "file"; uri: string | null; path: string; title: string; mime: string | null; media?: FileMedia }
  | { kind: "issue"; uri: string; card: Card }
  | { kind: "link"; uri: string; title: string; host: string; preview?: LinkPreview }
  | { kind: "board"; uri: string }
  | { kind: "localboard"; uri: string; path: string; title: string; groupBy: string; columns: LocalColumn[] }
  | { kind: "project"; uri: string; project: Project }
  | { kind: "text"; uri: null; text: string | null }
  | { kind: "group"; uri: null; title: string | null }
  | { kind: "missing"; uri: string | null; title: string };

/** What a web page says about itself, fetched by the host. Images are absolute paths. */
export interface LinkPreview {
  title: string | null;
  description: string | null;
  site: string | null;
  image: string | null;
  /** The image's width divided by its height, when it could be read. */
  imageAspect: number | null;
  favicon: string | null;
}

/** A PDF's or video's preview, made by the host. No poster when it couldn't be made. */
export interface FileMedia {
  poster: string | null;
  /** The poster's width divided by its height. */
  posterAspect: number | null;
  pages: number | null;
  /** In seconds. */
  duration: number | null;
}

export interface LocalColumn {
  value: string | null;
  label: string;
  notes: { path: string; title: string; excerpt: string }[];
}

/** A relationship drawn between two nodes of the canvas. */
export interface Arrow {
  id: string;
  type: string;
  from: string;
  to: string;
  label?: string | null;
  source: "github" | "local" | "file" | "derived";
  count?: number;
  /** The two issues' node ids, for a GitHub relationship. */
  issues?: [string, string];
}

export interface CanvasView {
  path: string;
  nodes: CanvasNode[];
  arrows: Arrow[];
}

export interface Overview extends CanvasView {
  projects: Project[];
  errors: string[];
}

export interface ProjectView extends CanvasView {
  project: Project;
  board: Board;
}

export interface Comment {
  id: string;
  author: string | null;
  body: string;
  created: string | null;
  pending: boolean;
}

export interface FieldDef {
  id: string;
  name: string;
  kind: string;
  options: { id: string; name: string; color?: string }[];
  value: { kind: string; optionId?: string; name?: string; value?: unknown } | null;
}

export interface Linked {
  node: string;
  repoName?: string;
  number?: number;
  title: string;
  state: string;
}

export interface IssueDetail extends Card {
  body: string;
  author: string | null;
  created: string | null;
  milestone: string | null;
  issueType: string | null;
  labelsFull: { id: string; name: string; color: string | null }[];
  assigneesFull: { id: string; login: string }[];
  fieldDefs: FieldDef[];
  repoLabels: { id: string; name: string; color: string | null }[];
  repoUsers: { id: string; login: string }[];
  blockedByList: Linked[];
  blockingList: Linked[];
  parent: Linked | null;
  subIssues: Linked[];
  commentList: Comment[];
  conflicts: { field: string; local: string; remote: string }[];
  statusField: string | null;
}

/** A typeset formula. Sizes are in ems of the text around it. */
export interface Formula {
  svg: string;
  width: number;
  height: number;
  /** How far it reaches below the baseline. */
  depth: number;
}

export interface Note {
  path: string;
  uri: string;
  title: string;
  body: string;
  /** The front matter, in the file's order. */
  props: NoteProp[];
}

export interface NoteProp {
  key: string;
  value: unknown;
}

export interface SyncStatus {
  running: boolean;
  pending: number;
  failed: { target: string; field: string | null; error: string }[];
  conflicts: number;
  lastSync: string | null;
  error: string | null;
  login: string | null;
}

export interface GithubRepo {
  nameWithOwner: string;
  description: string | null;
  updatedAt: string;
  isArchived: boolean;
  taken: boolean;
}

export interface DirEntry {
  name: string;
  path: string;
  dir: boolean;
}

export const api = {
  workspace: () => invoke<{ path: string | null }>("workspace"),
  openWorkspace: (path: string) => invoke<{ path: string }>("workspace_open", { path }),
  overview: () => invoke<Overview>("overview"),
  projectView: (project: string) => invoke<ProjectView>("project_view", { project }),
  createProject: (name: string, repo: string | null) => invoke<Project>("project_create", { name, repo }),
  githubRepos: () => invoke<GithubRepo[]>("github_repos"),
  saveCanvas: (path: string, nodes: CanvasNode[]) => invoke<null>("canvas_save", { path, nodes }),
  issue: (node: string) => invoke<IssueDetail | null>("issue", { node }),
  search: (query: string) => invoke<Card[]>("issue_search", { query }),
  setField: (node: string, field: string, value: unknown) => invoke<null>("set_field", { node, field, value }),
  moveCard: (node: string, column: string) => invoke<null>("move_card", { node, column }),
  createIssue: (repo: string, title: string, column: string | null) =>
    invoke<string>("create_issue", { repo, title, column }),
  createLocalIssue: (folder: string, title: string, column: string | null) =>
    invoke<string>("local_issue_create", { folder, title, column }),
  comment: (node: string, body: string) => invoke<null>("comment", { node, body }),
  resolve: (node: string, field: string, keepLocal: boolean) =>
    invoke<null>("resolve", { node, field, keepLocal }),
  relate: (from: string, to: string, type: string) => invoke<null>("relate", { from, to, type }),
  unrelate: (arrow: Arrow) => invoke<null>("unrelate", { arrow }),
  createNote: (folder: string, title: string, prop?: string, value?: string) =>
    invoke<Note>("note_create", { folder, title, prop, value }),
  createBoard: (folder: string, title: string) => invoke<CanvasNode>("board_create", { folder, title }),
  boardMove: (board: string, note: string, value: string | null) =>
    invoke<null>("board_move", { board, note, value }),
  note: (path: string) => invoke<Note>("note_read", { path }),
  writeNote: (path: string, body: string) => invoke<null>("note_write", { path, body }),
  renameNote: (path: string, title: string) => invoke<Note>("note_rename", { path, title }),
  /** Sets one front matter key; null removes it. */
  setNoteProp: (path: string, key: string, value: unknown) => invoke<Note>("note_prop", { path, key, value }),
  renameNoteProp: (path: string, from: string, to: string) => invoke<Note>("note_prop_rename", { path, from, to }),
  /** A LaTeX formula typeset as SVG; rejects with TeX's message when it doesn't parse. */
  math: (tex: string, display: boolean) => invoke<Formula>("math", { tex, display }),
  /** Copies a file into the note's folder; answers with the name the note links it by. */
  attachToNote: (path: string, file: string) => invoke<string>("note_attach", { path, file }),
  linkFiles: (paths: string[]) => invoke<CanvasNode[]>("files_link", { paths }),
  linkUrl: (url: string) => invoke<CanvasNode>("url_link", { url }),
  open: (target: string) => invoke<null>("open_external", { target }),
  listDir: (path: string) => invoke<DirEntry[]>("dir_list", { path }),
  highlight: (code: string, lang: string) => invoke<{ text: string; kind: string }[]>("highlight", { code, lang }),
  syncNow: () => invoke<null>("sync_now"),
  syncStatus: () => invoke<SyncStatus>("sync_status"),
};

/** The host says the workspace or the mirror changed: views read again. */
export function onChanged(run: (what: { reason: string }) => void): () => void {
  return listen<{ reason: string }>("changed", run);
}

export function onSync(run: (status: SyncStatus) => void): () => void {
  return listen<SyncStatus>("sync", run);
}
