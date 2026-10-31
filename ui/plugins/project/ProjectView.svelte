<script lang="ts">
  // A project's canvas: its GitHub board as one region, and around it whatever you place: issues
  // dragged out of the board, notes, files, links, local boards. Arrows are relationships: between
  // two issues they can be blocks or sub-issues on GitHub; anything else is kept in the workspace.
  import { clipboard } from "@neoworks/svelte-gpui";
  import { Button, ContextMenu, Presence, Text, View } from "@neoworks/svelte-gpui/ui";
  import { fade, fly } from "svelte/transition";
  import type { Arrow, CanvasNode, Card, ProjectView, Stroke } from "../../lib/api.ts";
  import { arranged, type Arrangement } from "../../lib/canvas/arrange.ts";
  import { contains, freeSpot, type Point } from "../../lib/canvas/geometry.ts";
  import { flatPoints, pointsOf } from "../../lib/canvas/strokes.ts";
  import { cubicBezier, glide, tween } from "../../lib/canvas/motion.ts";
  import { bounds, onTheWay, pushAside, stagger, unfoldLayout } from "../../lib/canvas/unfold.ts";
  import { pastedUrl } from "../../lib/canvas/paste.ts";
  import Toolbar, { type Tool } from "../../lib/canvas/Toolbar.svelte";
  import Viewport, { type DrawTool, type Move, type ViewState } from "../../lib/canvas/Viewport.svelte";
  import { relationOptions } from "../../lib/relations.ts";
  import { app } from "../../lib/state.svelte.ts";
  import { c, easing, eyebrow, ghost, radii, shadow, stickyTint, tones } from "../../lib/theme.ts";
  import BoardNode from "./BoardNode.svelte";
  import Files, { isFileDrag } from "./Files.svelte";
  import FolderName from "./FolderName.svelte";
  import ItemNode from "./ItemNode.svelte";
  import StickyNote, { stickyColors } from "./StickyNote.svelte";
  import { projectKeys } from "./keys.svelte.ts";
  import { icons } from "../../lib/icons.ts";
  const { board: kanban, files: treeView, note: notePencil } = icons;
  /** What the pen draws with: a few colours, and a thin and a thick line (canvas units). */
  const penColors = [c.text, tones.RED, tones.YELLOW, tones.GREEN, tones.BLUE];
  const penWidths = [2.5, 6];

  // `inside` is the directory of a folder open within the project, whose canvas shows instead.
  let { folder, inside }: { folder: string; inside?: string } = $props();

  let data = $state<ProjectView | null>(null);
  let view = $state<ViewState>({ x: -40, y: -60, zoom: 0.9 });
  let selected = $state<string[]>([]);
  let selectedArrow = $state<string | null>(null);
  let connecting = $state<{ from: string[]; to: string } | null>(null);
  let size = $state({ width: 800, height: 600 });

  /** Where new notes, boards and folders go: the open folder's directory, or the project's. */
  const here = $derived(inside ?? folder);

  async function newBoard() {
    if (!data) return;
    const node = await app.run(() => app.api.createBoard(here, "Notes board"));
    if (node) add(node, centre());
  }

  async function newFolder() {
    if (!data) return;
    const node = await app.run(() => app.api.createFolder(here, "New folder"));
    if (node) add(node, centre());
  }

  /** The file explorer at the left. */
  let filesOpen = $state(false);

  /** The drawing tool that is on, if any: a drag on the background draws or erases with it. */
  let tool = $state<DrawTool | null>(null);
  let pen = $state({ color: penColors[0], width: penWidths[0] });

  function toggleTool(next: DrawTool) {
    if (tool === next) tool = null;
    else tool = next;
  }

  const tools: Tool[] = $derived([
    { icon: treeView, label: "Files", key: "F", onpress: () => (filesOpen = !filesOpen) },
    { icon: notePencil, label: "Note", key: "N", onpress: () => void newNote(centre()) },
    { icon: icons.sticky, label: "Sticky note", key: "S", onpress: () => newSticky(centre()) },
    { icon: kanban, label: "Notes board", onpress: () => void newBoard() },
    { icon: icons.folder, label: "Folder", onpress: () => void newFolder() },
    { icon: icons.pen, label: "Pen", key: "P", active: tool === "pen", onpress: () => toggleTool("pen") },
    { icon: icons.eraser, label: "Eraser", key: "E", active: tool === "eraser", onpress: () => toggleTool("eraser") },
  ]);

  /** The strokes on the background, through world points. */
  const strokes = $derived((data?.strokes ?? []).map((s) => ({ ...s, points: pointsOf(s.points) })));

  function saveStrokes(next: Stroke[]) {
    if (!data) return;
    data = { ...data, strokes: next };
    const path = data.path;
    void app.run(() => app.api.saveStrokes(path, next));
  }

  function drew(points: Point[]) {
    if (!data) return;
    saveStrokes([...data.strokes, { id: newId(), color: pen.color, width: pen.width, points: flatPoints(points) }]);
  }

  function erased(ids: string[]) {
    if (!data) return;
    saveStrokes(data.strokes.filter((s) => !ids.includes(s.id)));
  }

  /** The folder whose contents are laid out beside it, by its node's id, while it folds too. */
  let unfolded = $state<string | null>(null);
  /** How far its contents have come out from under it: 0 folded, 1 in their rows. */
  let spread = $state(0);
  let stopSpread: (() => void) | null = null;
  /** The node being dragged onto a folder, and the folder it would go into if let go now. */
  let dropping = $state<{ node: string; folder: string } | null>(null);

  $effect(() => {
    void app.version;
    const f = folder;
    const i = inside;
    const u = unfolded;
    void app.run(async () => {
      const next = await app.api.projectView(f, i, u ? [u] : []);
      if (f === folder && i === inside && u === unfolded) data = next;
    });
  });

  interface Placed {
    id: string;
    x: number;
    y: number;
    w: number;
    h: number;
    raw: CanvasNode;
    /** Below 1 while it comes out of a folder or goes back in. */
    opacity?: number;
  }

  /** The design system's easing for things that arrive and leave, and how long each takes. */
  const easeOut = cubicBezier(easing.out);
  const UNFOLD_MS = 420;
  const FOLD_MS = 300;

  /** Fetches the folder's contents, then lets them come out from under it. */
  async function unfold(id: string) {
    stopSpread?.();
    const next = await app.run(() => app.api.projectView(folder, inside, [id]));
    if (!next) return;
    data = next;
    unfolded = id;
    spread = 0;
    stopSpread = tween(0, 1, UNFOLD_MS, (value) => (spread = value));
  }

  $effect(() => () => stopSpread?.());

  /** Takes the contents back under the folder, from wherever they are on their way. */
  function fold() {
    if (!unfolded) return;
    stopSpread?.();
    stopSpread = tween(spread, 0, FOLD_MS * spread, (value) => (spread = value), () => (unfolded = null));
  }

  /** What a node is called, for sorting an unfolded folder's rows. */
  function nameOf(n: CanvasNode): string {
    const item = n.item;
    if (item?.kind === "issue") return item.card.title;
    if (item && "title" in item && item.title) return item.title;
    return n.text ?? "";
  }

  // The canvas's own nodes where they are, then the unfolded folder's on their way from under it
  // to their rows beside it, one after the other in the rows' order. Own nodes in the rows' way
  // move aside while the folder is open; only a drag saves where they were pushed to.
  const nodes = $derived.by<Placed[]>(() => {
    const all = data?.nodes ?? [];
    const own = all
      .filter((n) => !n.in)
      .map((n) => ({ id: n.id, x: n.x, y: n.y, w: n.width, h: n.height, raw: n }));
    const folderNode = own.find((n) => n.id === unfolded);
    if (!folderNode) return own;
    const inner = all.filter((n) => n.in?.folder === unfolded);
    const spots = unfoldLayout(
      folderNode,
      inner.map((n) => ({ id: n.id, kind: n.item?.kind ?? n.type, title: nameOf(n), w: n.width, h: n.height })),
    );
    const order = [...spots.keys()];
    const laid = inner.map((n) => {
      const spot = spots.get(n.id) ?? { x: n.x, y: n.y };
      const t = easeOut(stagger(spread, order.indexOf(n.id), order.length));
      const at = onTheWay(folderNode, spot, n.width, n.height, t);
      return { id: n.id, x: at.x, y: at.y, w: n.width, h: n.height, raw: n, opacity: t };
    });
    const rows = bounds(inner.map((n) => ({ ...(spots.get(n.id) ?? n), w: n.width, h: n.height })));
    if (!rows) return [...own, ...laid];
    const offsets = pushAside(rows, own.filter((n) => n.id !== unfolded));
    const t = easeOut(spread);
    const pushed = own.map((n) => {
      const offset = offsets.get(n.id);
      if (!offset) return n;
      return { ...n, x: n.x + offset.x * t, y: n.y + offset.y * t };
    });
    return [...pushed, ...laid];
  });
  const boardNode = $derived(nodes.find((n) => n.raw.item?.kind === "board"));
  const arrows = $derived(
    (data?.arrows ?? []).map((a) => ({ ...a, color: a.type === "blocks" ? "#f2555a99" : undefined })),
  );

  /** Issues on the canvas, counted by the board column they'd be in. */
  const placed = $derived.by(() => {
    const counts: Record<string, number> = {};
    for (const n of data?.nodes ?? []) {
      if (n.item?.kind !== "issue") continue;
      const card = n.item.card;
      const column = data!.board.statusField
        ? data!.board.columns.find((col) => col.name === card.fields.Status) ?? data!.board.columns.find((col) => col.done)
        : data!.board.columns.find((col) => col.key === (card.state === "OPEN" ? "open" : "closed"));
      if (column) counts[column.key] = (counts[column.key] ?? 0) + 1;
    }
    return counts;
  });

  /**
   * Writes the canvas, without an unfolded folder's nodes, which live on the folder's. A node the
   * host hasn't described yet (just added) needs a reload.
   */
  function save(next: CanvasNode[]) {
    if (!data) return;
    data = { ...data, nodes: next };
    const path = data.path;
    const own = next.filter((n) => !n.in);
    const fresh = own.some((n) => !n.item);
    void app.run(async () => {
      await app.api.saveCanvas(path, own);
      if (fresh || own.length !== nodes.filter((n) => !n.raw.in).length) app.version++;
    });
  }

  /** Adds a node near `at`, where there is room. */
  function add(node: CanvasNode, at: Point) {
    if (!data) return;
    const spot = freeSpot(nodes, at, node.width, node.height);
    save([...data.nodes, { ...node, x: Math.round(spot.x), y: Math.round(spot.y) }]);
    selected = [node.id];
  }

  /** Adds a node centred on where it was dropped: a drop chooses its place. */
  function place(node: CanvasNode, at: Point) {
    if (!data) return;
    save([...data.nodes, { ...node, x: Math.round(at.x - node.width / 2), y: Math.round(at.y - node.height / 2) }]);
    selected = [node.id];
  }

  function newId(): string {
    return Math.random().toString(16).slice(2, 18);
  }

  /** The folder under `centre` that the nodes `ids` would go into, other than one of them. */
  function folderAt(ids: string[], centre: Point): { id: string; canvas: string } | null {
    for (const n of nodes) {
      const item = n.raw.item;
      if (!ids.includes(n.id) && item?.kind === "folder" && contains(n, centre)) return { id: n.id, canvas: item.canvas };
    }
    return null;
  }

  /** The nodes a drag of `id` moves: the selection when it is part of it. */
  function dragged(id: string): string[] {
    if (selected.includes(id)) return selected;
    return [id];
  }

  function dragging(id: string, x: number, y: number) {
    const node = nodes.find((n) => n.id === id);
    if (!node) return;
    const into = folderAt(dragged(id), { x: x + node.w / 2, y: y + node.h / 2 });
    if (into) dropping = { node: id, folder: into.id };
    else dropping = null;
  }

  function moved(moves: Move[], grabbed: string) {
    dropping = null;
    if (moves.length === 1) {
      movedOne(moves[0].id, moves[0].x, moves[0].y);
      return;
    }
    movedGroup(moves, grabbed);
  }

  /**
   * Several nodes let go: the one held decides where they go. Over a folder they all go into it,
   * over the board the issues among them return to it, and anywhere else they stay where they were
   * let go. An unfolded folder's nodes go back to its rows unless they go into another folder.
   */
  function movedGroup(moves: Move[], grabbed: string) {
    if (!data) return;
    const held = data.nodes.find((n) => n.id === grabbed);
    const lead = moves.find((m) => m.id === grabbed);
    if (!held || !lead) return;
    const centre = { x: lead.x + held.width / 2, y: lead.y + held.height / 2 };
    const ids = moves.map((m) => m.id);
    const into = folderAt(ids, centre);
    if (into) {
      void moveAllInto(ids, into.canvas);
      return;
    }
    const onBoard = boardNode !== undefined && contains(boardNode, centre);
    const at = new Map(moves.map((m) => [m.id, m]));
    const next = data.nodes.flatMap((n) => {
      const move = at.get(n.id);
      if (!move || n.in) return [n];
      if (onBoard && n.item?.kind === "issue") return [];
      return [{ ...n, x: move.x, y: move.y }];
    });
    save(next);
  }

  /** Moves the nodes `ids` onto the folder canvas `into`, one after the other, then reloads. */
  async function moveAllInto(ids: string[], into: string) {
    if (!data) return;
    const path = data.path;
    const moving = data.nodes.filter((n) => ids.includes(n.id));
    data = { ...data, nodes: data.nodes.filter((n) => !ids.includes(n.id)) };
    selected = [];
    for (const node of moving) {
      let from = path;
      let id = node.id;
      if (node.in) {
        from = node.in.canvas;
        id = node.in.id;
      }
      if (from !== into) await app.run(() => app.api.putInFolder(from, id, into));
    }
    app.version++;
  }

  function movedOne(id: string, x: number, y: number) {
    if (!data) return;
    const node = data.nodes.find((n) => n.id === id);
    if (!node) return;
    const centre = { x: x + node.width / 2, y: y + node.height / 2 };
    if (node.in) {
      movedFromFolder(node, node.in, x, y, centre);
      return;
    }
    // An issue dropped back on the board leaves the canvas and returns to its column.
    if (node.item?.kind === "issue" && boardNode && contains(boardNode, centre)) {
      save(data.nodes.filter((n) => n.id !== id));
      return;
    }
    const into = folderAt([id], centre);
    if (into) {
      void moveNode(id, data.path, id, into.canvas);
      return;
    }
    save(data.nodes.map((n) => (n.id === id ? { ...n, x, y } : n)));
  }

  /**
   * A node of the unfolded folder let go: onto another folder it goes there, among its folder's
   * rows it goes back to its place, and anywhere else onto this canvas where it was let go.
   */
  function movedFromFolder(node: CanvasNode, from: NonNullable<CanvasNode["in"]>, x: number, y: number, centre: Point) {
    if (!data) return;
    const into = folderAt([node.id], centre);
    if (into) {
      if (into.canvas !== from.canvas) void moveNode(node.id, from.canvas, from.id, into.canvas);
      return;
    }
    const area = bounds(nodes.filter((n) => n.raw.in?.folder === from.folder || n.id === from.folder));
    if (area && contains(area, centre)) return;
    void moveNode(node.id, from.canvas, from.id, data.path, [x, y]);
  }

  /**
   * Moves the node shown as `shown`, `id` on the canvas `from`, onto the canvas `into`. The host
   * moves its file too when it sits beside `from`, and the reload shows where it went.
   */
  async function moveNode(shown: string, from: string, id: string, into: string, at?: [number, number]) {
    if (!data) return;
    data = { ...data, nodes: data.nodes.filter((n) => n.id !== shown) };
    selected = [];
    await app.run(() => app.api.putInFolder(from, id, into, at));
    app.version++;
  }

  async function renameFolder(dir: string, name: string) {
    const next = name.trim();
    if (!next || next === dir.split("/").pop()) return;
    await app.run(() => app.api.renameFolder(dir, next));
    app.version++;
  }

  /**
   * How small a node can be resized, so its title stays readable: sticky notes, notes, issue
   * cards, web links and folders resize, the rest keep their size.
   */
  function minSize(n: Placed): { w: number; h: number } | null {
    // An unfolded folder's nodes are laid out in rows; their size is the folder canvas's.
    if (n.raw.in) return null;
    if (n.raw.type === "text") return { w: 120, h: 80 };
    if (n.raw.item?.kind === "note") return { w: 160, h: 90 };
    if (n.raw.item?.kind === "issue") return { w: 200, h: 96 };
    if (n.raw.item?.kind === "link") return { w: 200, h: 72 };
    if (n.raw.item?.kind === "folder") return { w: 180, h: 140 };
    return null;
  }

  function resizedNode(id: string, rect: { x: number; y: number; w: number; h: number }) {
    if (!data) return;
    save(data.nodes.map((n) => (n.id === id ? { ...n, x: rect.x, y: rect.y, width: rect.w, height: rect.h } : n)));
  }

  let menu = $state<ContextMenu>();
  let stopArrange: (() => void) | null = null;
  const ARRANGE_MS = 280;

  $effect(() => () => stopArrange?.());

  /** The menu for a right press on several nodes: ways to lay them out. */
  function context(ids: string[], x: number, y: number) {
    if (ids.length < 2) return;
    menu?.openAt(x, y, [
      { label: "Arrange in grid", icon: icons.arrangeGrid, onselect: () => arrange(ids, "grid") },
      { label: "Arrange in a row", icon: icons.arrangeRow, onselect: () => arrange(ids, "row") },
      { label: "Arrange in a column", icon: icons.arrangeColumn, onselect: () => arrange(ids, "column") },
    ]);
  }

  /**
   * Lays the nodes `ids` out as `arrangement` says, gliding them there, then saves. The board and
   * an unfolded folder's nodes stay where they are.
   */
  function arrange(ids: string[], arrangement: Arrangement) {
    if (!data) return;
    const own = data.nodes.filter((n) => ids.includes(n.id) && !n.in && n.item?.kind !== "board");
    if (own.length < 2) return;
    const starts = new Map(own.map((n) => [n.id, { x: n.x, y: n.y }]));
    const spots = arranged(own.map((n) => ({ id: n.id, x: n.x, y: n.y, w: n.width, h: n.height })), arrangement);
    const placedAt = (t: number) => (n: CanvasNode) => {
      const start = starts.get(n.id);
      const spot = spots.get(n.id);
      if (!start || !spot) return n;
      return { ...n, x: Math.round(start.x + (spot.x - start.x) * t), y: Math.round(start.y + (spot.y - start.y) * t) };
    };
    stopArrange?.();
    stopArrange = tween(
      0,
      1,
      ARRANGE_MS,
      (t) => {
        if (data) data = { ...data, nodes: data.nodes.map(placedAt(easeOut(t))) };
      },
      () => {
        if (data) save(data.nodes.map(placedAt(1)));
      },
    );
  }

  /** Glides so every node is in view, above the toolbar. */
  function fit() {
    if (nodes.length === 0) return;
    const left = Math.min(...nodes.map((n) => n.x));
    const top = Math.min(...nodes.map((n) => n.y));
    const right = Math.max(...nodes.map((n) => n.x + n.w));
    const bottom = Math.max(...nodes.map((n) => n.y + n.h));
    const pad = 40;
    const toolbar = 64;
    const zoom = Math.min(1.2, (size.width - 2 * pad) / (right - left), (size.height - 2 * pad - toolbar) / (bottom - top));
    glide(view, { x: left - pad / zoom, y: top - pad / zoom, zoom }, (next) => (view = next));
  }

  /** A press on a folder of this canvas unfolds it, and folds it again. */
  function pressed(id: string) {
    const node = data?.nodes.find((n) => n.id === id);
    if (node?.item?.kind !== "folder" || node.in) return;
    if (unfolded === id) fold();
    else void unfold(id);
  }

  function open(id: string) {
    selected = [id];
    const node = data?.nodes.find((n) => n.id === id);
    // A sticky note in a folder is written on the folder's canvas.
    if (node?.type === "text" && !node.in) {
      editing = id;
      return;
    }
    const item = data?.nodes.find((n) => n.id === id)?.item;
    if (!item) return;
    if (item.kind === "issue") app.select({ kind: "issue", node: item.card.node });
    else if (item.kind === "note") app.select({ kind: "note", path: item.path });
    else if (item.kind === "image" || item.kind === "file") void app.run(() => app.api.open(item.path));
    else if (item.kind === "link") void app.run(() => app.api.open(item.uri));
    else if (item.kind === "folder") app.open({ view: "project", project: folder, inside: item.path });
  }

  /** The selected node when it is the only one, whose bar shows above the canvas. */
  const selectedNode = $derived.by(() => {
    if (selected.length !== 1) return null;
    return data?.nodes.find((n) => n.id === selected[0]) ?? null;
  });

  /** The sticky note being written in place. */
  let editing = $state<string | null>(null);

  function newSticky(at: Point) {
    const id = newId();
    add({ id, type: "text", text: "", color: "3", x: at.x, y: at.y, width: 240, height: 170 }, at);
    editing = id;
  }

  function setText(id: string, text: string) {
    if (!data) return;
    save(data.nodes.map((n) => (n.id === id ? { ...n, text } : n)));
  }

  function setColor(id: string, color: string) {
    if (!data) return;
    save(data.nodes.map((n) => (n.id === id ? { ...n, color } : n)));
  }

  async function newNote(at: Point) {
    if (!data) return;
    await app.run(async () => {
      const note = await app.api.createNote(here, "Untitled");
      const id = newId();
      add({ id, type: "file", file: note.path, x: at.x, y: at.y, width: 280, height: 160 }, at);
      app.select({ kind: "note", path: note.path });
    });
  }

  function centre(): Point {
    return { x: view.x + (size.width / 2 - 140) / view.zoom, y: view.y + (size.height / 2 - 80) / view.zoom };
  }

  /** Adds the link on the clipboard to the middle of the view; other text is left alone. */
  async function paste() {
    const url = pastedUrl(await clipboard.read());
    if (!url) return;
    const node = await app.run(() => app.api.linkUrl(url));
    if (node) add(node, centre());
  }

  async function files(paths: string[], at: Point) {
    const linked = await app.run(() => app.api.linkFiles(paths));
    if (!linked || !data) return;
    if (linked.length === 1) {
      place(linked[0], at);
      return;
    }
    let next = [...data.nodes];
    for (const node of linked) {
      const spot = freeSpot(
        next.map((n) => ({ x: n.x, y: n.y, w: n.width, h: n.height })),
        { x: at.x + node.x, y: at.y + node.y },
        node.width,
        node.height,
      );
      next = [...next, { ...node, x: Math.round(spot.x), y: Math.round(spot.y) }];
    }
    save(next);
  }

  function isCard(d: unknown): d is { kind: "card"; card: Card } {
    return typeof d === "object" && d !== null && (d as { kind?: string }).kind === "card";
  }

  function dropped(d: unknown, at: Point) {
    if (isFileDrag(d)) {
      void files([d.path], at);
      return;
    }
    if (!isCard(d)) return;
    // A local issue is a file of the workspace, so it is placed as one.
    if (d.card.local) {
      place({ id: newId(), type: "file", file: d.card.node.slice("issue:".length), x: at.x, y: at.y, width: 280, height: 120 }, at);
      return;
    }
    if (!d.card.url) return;
    place({ id: newId(), type: "link", url: d.card.url, x: at.x, y: at.y, width: 280, height: 120 }, at);
  }

  function uriOf(id: string): string | null {
    const item = data?.nodes.find((n) => n.id === id)?.item;
    if (!item) return null;
    if (item.kind === "issue") return item.uri;
    return item.uri ?? null;
  }

  /** Relates each node of `link.from` that can be related to `link.to`, and says how many couldn't. */
  async function connect(type: string) {
    const link = connecting;
    connecting = null;
    if (!link) return;
    const to = uriOf(link.to);
    const from = link.from.map(uriOf).filter((uri) => uri !== null);
    if (!to || from.length === 0) {
      app.say("Only notes, files, links and issues can be connected.");
      return;
    }
    for (const uri of from) {
      await app.run(() => app.api.relate(uri, to, type));
    }
    const skipped = link.from.length - from.length;
    if (skipped === 1) app.say("1 node was left out: only notes, files, links and issues can be connected.");
    if (skipped > 1) app.say(`${skipped} nodes were left out: only notes, files, links and issues can be connected.`);
  }

  function cardOf(id: string): Card | null {
    const item = data?.nodes.find((n) => n.id === id)?.item;
    if (item?.kind === "issue") return item.card;
    return null;
  }

  /** The relationships the nodes being connected can all have, leaving out those that can't have any. */
  const connectOptions = $derived.by(() => {
    if (!connecting) return [];
    const from = connecting.from.filter((id) => uriOf(id) !== null);
    return relationOptions(from.map(cardOf), cardOf(connecting.to));
  });

  function removeSelected(): boolean {
    if (!data) return false;
    if (selectedArrow) {
      const arrow = data.arrows.find((a) => a.id === selectedArrow);
      selectedArrow = null;
      if (arrow) void app.run(() => app.api.unrelate(arrow as Arrow));
      return true;
    }
    // The board stays, and an unfolded folder's nodes live on the folder's canvas, removed there.
    const removed = data.nodes.filter((n) => selected.includes(n.id) && n.item?.kind !== "board" && !n.in);
    if (removed.length === 0) return false;
    selected = [];
    app.select(null);
    save(data.nodes.filter((n) => !removed.includes(n)));
    return true;
  }

  $effect(() => {
    projectKeys.remove = removeSelected;
    projectKeys.note = () => void newNote(centre());
    projectKeys.paste = paste;
    projectKeys.files = () => (filesOpen = !filesOpen);
    projectKeys.sticky = () => newSticky(centre());
    projectKeys.pen = () => toggleTool("pen");
    projectKeys.eraser = () => toggleTool("eraser");
    projectKeys.stopDrawing = () => {
      if (!tool) return false;
      tool = null;
      return true;
    };
    return () => {
      projectKeys.pen = null;
      projectKeys.eraser = null;
      projectKeys.stopDrawing = null;
      projectKeys.remove = null;
      projectKeys.note = null;
      projectKeys.paste = null;
      projectKeys.files = null;
      projectKeys.sticky = null;
    };
  });
</script>

<View style={{ flex: 1 }} onlayout={(l) => (size = l)}>
  {#if data}
    <Viewport
      {nodes}
      {arrows}
      bind:view
      {selected}
      {selectedArrow}
      linkable
      {minSize}
      onresized={resizedNode}
      onselect={(ids) => (selected = ids)}
      onbackground={fold}
      onopen={open}
      onselectarrow={(id) => (selectedArrow = id)}
      onmoved={moved}
      ondrag={dragging}
      onpress={pressed}
      onconnect={(from, to) => (connecting = { from, to })}
      oncontext={context}
      onfiles={(paths, at) => void files(paths, at)}
      accept={(d) => isCard(d) || isFileDrag(d)}
      ondropdata={dropped}
      {strokes}
      {tool}
      {pen}
      ondrew={drew}
      onerase={erased}
    >
      {#snippet node(n, zoom)}
        <!-- Held over a folder, a node lets the folder show through. -->
        <View style={{ flex: 1, opacity: dropping?.node === n.id ? 0.45 : (n.opacity ?? 1) }}>
          {#if n.raw.item?.kind === "board"}
            <BoardNode board={data!.board} project={data!.project} {zoom} {placed} />
          {:else if n.raw.type === "text"}
            <StickyNote
              text={n.raw.text ?? ""}
              color={n.raw.color}
              {zoom}
              selected={selected.includes(n.id)}
              editing={editing === n.id}
              onchange={(text) => setText(n.id, text)}
              ondone={() => (editing = null)}
            />
          {:else if n.raw.item}
            <ItemNode
              item={n.raw.item}
              {zoom}
              selected={selected.includes(n.id)}
              width={n.w}
              height={n.h}
              open={unfolded === n.id}
              target={dropping?.folder === n.id}
            />
          {/if}
        </View>
      {/snippet}
    </Viewport>

    <Toolbar {tools} bind:view {size} onfit={fit} />
    <!-- Its menu doesn't grow for a row's icon, which cuts the longest label short. -->
    <ContextMenu bind:this={menu} menuStyle={{ minWidth: 200 }} />

    {#if filesOpen}
      <Presence
        in={[fly, { x: -24, duration: 220 }]}
        out={[fade, { duration: 140 }]}
        style={{ position: "absolute", top: 12, left: 12, bottom: 12, direction: "row" }}
      >
        <Files project={data.project} onclose={() => (filesOpen = false)} />
      </Presence>
    {/if}

    {#if tool === "pen"}
      <!-- The pen's colours and widths, where a selected node's bar would be. -->
      <Presence
        in={[fly, { y: 10, duration: 180 }]}
        out={[fade, { duration: 120 }]}
        style={{ position: "absolute", top: 14, left: 0, right: 0, align: "center" }}
      >
        <View
          style={{ direction: "row", gap: 6, align: "center", padding: 6, radius: radii.card, background: c.surface, borderWidth: 1, borderColor: c.border, shadow }}
          onpress={() => {}}
        >
          {#each penColors as color (color)}
            <Button
              style={{ size: 22, radius: 11, background: color, borderWidth: 2, borderColor: pen.color === color ? c.text : c.surface, transition: 120 }}
              hover={{ borderColor: c.text }}
              onpress={() => (pen = { ...pen, color })}
            />
          {/each}
          <View style={{ width: 1, height: 18, background: c.border, marginX: 2 }} />
          {#each penWidths as width (width)}
            <Button
              style={{ size: 26, radius: 8, align: "center", justify: "center", background: pen.width === width ? c.hover : c.surface, transition: 120 }}
              hover={{ background: c.hover }}
              onpress={() => (pen = { ...pen, width })}
            >
              <View style={{ size: width + 2, radius: (width + 2) / 2, background: pen.color }} />
            </Button>
          {/each}
        </View>
      </Presence>
    {/if}

    {#if !tool && selectedNode?.type === "text" && !selectedNode.in && editing !== selectedNode.id}
      {@const sticky = selectedNode}
      <Presence
        in={[fly, { y: 10, duration: 180 }]}
        out={[fade, { duration: 120 }]}
        style={{ position: "absolute", top: 14, left: 0, right: 0, align: "center" }}
      >
        <View
          style={{ direction: "row", gap: 6, align: "center", padding: 6, radius: radii.card, background: c.surface, borderWidth: 1, borderColor: c.border, shadow }}
          onpress={() => {}}
        >
          {#each stickyColors as color (color)}
            {@const tint = stickyTint(color)}
            <Button
              style={{ size: 22, radius: 11, background: tint.background, borderWidth: 2, borderColor: (sticky.color ?? "3") === color ? c.text : tint.border, transition: 120 }}
              hover={{ borderColor: c.text }}
              onpress={() => setColor(sticky.id, color)}
            />
          {/each}
        </View>
      </Presence>
    {/if}

    {#if !tool && selectedNode?.item?.kind === "folder"}
      {@const folderItem = selectedNode.item}
      <Presence
        in={[fly, { y: 10, duration: 180 }]}
        out={[fade, { duration: 120 }]}
        style={{ position: "absolute", top: 14, left: 0, right: 0, align: "center" }}
      >
        {#key folderItem.path}
          <FolderName name={folderItem.title} onrename={(name) => void renameFolder(folderItem.path, name)} />
        {/key}
      </Presence>
    {/if}

    {#if selectedArrow}
      {@const arrow = data.arrows.find((a) => a.id === selectedArrow)}
      {#if arrow}
        <Presence
          in={[fly, { y: 10, duration: 180 }]}
          out={[fade, { duration: 120 }]}
          style={{ position: "absolute", top: 14, left: 0, right: 0, align: "center" }}
        >
          <View
            style={{ direction: "row", gap: 6, align: "center", padding: 5, paddingLeft: 14, radius: radii.card, background: c.surface, borderWidth: 1, borderColor: c.border, shadow }}
            onpress={() => {}}
          >
            <Text style={{ fontSize: 13, color: c.text }}>{arrow.label ?? arrow.type}</Text>
            <Text style={{ fontSize: 12, color: c.faint, paddingRight: 6 }}>
              {arrow.source === "github" ? "on GitHub" : arrow.source === "local" ? "in the issue files" : arrow.source === "derived" ? "from GitHub" : "in relations.jsonl"}
            </Text>
            <Button style={{ ...ghost, color: c.danger }} hover={{ background: c.dangerSoft }} onpress={() => removeSelected()}>
              <Text>Remove</Text>
            </Button>
          </View>
        </Presence>
      {/if}
    {/if}

    {#if connecting}
      <Presence
        in={[fade, { duration: 160 }]}
        out={[fade, { duration: 120 }]}
        style={{ position: "absolute", inset: 0, background: "#00000070", align: "center", justify: "center" }}
      >
        <View style={{ position: "absolute", inset: 0 }} onpress={() => (connecting = null)} />
        <View
          style={{ width: 340, padding: 6, gap: 2, radius: radii.panel, background: c.surface, borderWidth: 1, borderColor: c.border, shadow }}
          onpress={() => {}}
        >
          <Text style={{ ...eyebrow, padding: [10, 10, 6, 10] }}>Connect as</Text>
          {#each connectOptions as option (option.type)}
            <Button
              style={{ direction: "column", align: "start", gap: 2, paddingX: 10, paddingY: 8, radius: 10, transition: 100 }}
              hover={{ background: c.hover }}
              onpress={() => void connect(option.type)}
            >
              <Text style={{ fontSize: 14, color: c.text }}>{option.label}</Text>
              <Text style={{ fontSize: 12, color: c.faint }}>{option.where}</Text>
            </Button>
          {/each}
        </View>
      </Presence>
    {/if}
  {/if}
</View>
