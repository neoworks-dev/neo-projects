<script lang="ts">
  // A project's canvas: its GitHub board as one region, and around it whatever you place: issues
  // dragged out of the board, notes, files, links, local boards. Arrows are relationships: between
  // two issues they can be blocks or sub-issues on GitHub; anything else is kept in the workspace.
  import { clipboard } from "@neoworks/svelte-gpui";
  import { Button, Presence, Text, View } from "@neoworks/svelte-gpui/ui";
  import { fade, fly } from "svelte/transition";
  import type { Arrow, CanvasNode, Card, ProjectView } from "../../lib/api.ts";
  import { contains, freeSpot, type Point } from "../../lib/canvas/geometry.ts";
  import { glide } from "../../lib/canvas/motion.ts";
  import { pastedUrl } from "../../lib/canvas/paste.ts";
  import Toolbar, { type Tool } from "../../lib/canvas/Toolbar.svelte";
  import Viewport, { type ViewState } from "../../lib/canvas/Viewport.svelte";
  import { app } from "../../lib/state.svelte.ts";
  import { c, eyebrow, ghost, radii, shadow, stickyTint } from "../../lib/theme.ts";
  import BoardNode from "./BoardNode.svelte";
  import Files, { isFileDrag } from "./Files.svelte";
  import ItemNode from "./ItemNode.svelte";
  import StickyNote, { stickyColors } from "./StickyNote.svelte";
  import { projectKeys } from "./keys.svelte.ts";
  import { icons } from "../../lib/icons.ts";
  const { board: kanban, files: treeView, note: notePencil } = icons;

  let { folder }: { folder: string } = $props();

  let data = $state<ProjectView | null>(null);
  let view = $state<ViewState>({ x: -40, y: -60, zoom: 0.9 });
  let selected = $state<string | null>(null);
  let selectedArrow = $state<string | null>(null);
  let connecting = $state<{ from: string; to: string } | null>(null);
  let size = $state({ width: 800, height: 600 });

  async function newBoard() {
    const project = data?.project;
    if (!project) return;
    const node = await app.run(() => app.api.createBoard(project.folder, "Notes board"));
    if (node) add(node, centre());
  }

  /** The file explorer at the left. */
  let filesOpen = $state(false);

  const tools: Tool[] = [
    { icon: treeView, label: "Files", key: "F", onpress: () => (filesOpen = !filesOpen) },
    { icon: notePencil, label: "Note", key: "N", onpress: () => void newNote(centre()) },
    { icon: icons.sticky, label: "Sticky note", key: "S", onpress: () => newSticky(centre()) },
    { icon: kanban, label: "Notes board", onpress: () => void newBoard() },
  ];

  $effect(() => {
    void app.version;
    const f = folder;
    void app.run(async () => {
      const next = await app.api.projectView(f);
      if (f === folder) data = next;
    });
  });

  interface Placed {
    id: string;
    x: number;
    y: number;
    w: number;
    h: number;
    raw: CanvasNode;
  }

  const nodes = $derived<Placed[]>(
    (data?.nodes ?? []).map((n) => ({ id: n.id, x: n.x, y: n.y, w: n.width, h: n.height, raw: n })),
  );
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

  /** Writes the canvas. A node the host hasn't described yet (just added) needs a reload. */
  function save(next: CanvasNode[]) {
    if (!data) return;
    data = { ...data, nodes: next };
    const path = data.path;
    const fresh = next.some((n) => !n.item);
    void app.run(async () => {
      await app.api.saveCanvas(path, next);
      if (fresh || next.length !== nodes.length) app.version++;
    });
  }

  /** Adds a node near `at`, where there is room. */
  function add(node: CanvasNode, at: Point) {
    if (!data) return;
    const spot = freeSpot(nodes, at, node.width, node.height);
    save([...data.nodes, { ...node, x: Math.round(spot.x), y: Math.round(spot.y) }]);
    selected = node.id;
  }

  /** Adds a node centred on where it was dropped: a drop chooses its place. */
  function place(node: CanvasNode, at: Point) {
    if (!data) return;
    save([...data.nodes, { ...node, x: Math.round(at.x - node.width / 2), y: Math.round(at.y - node.height / 2) }]);
    selected = node.id;
  }

  function newId(): string {
    return Math.random().toString(16).slice(2, 18);
  }

  function moved(id: string, x: number, y: number) {
    if (!data) return;
    const node = data.nodes.find((n) => n.id === id);
    if (!node) return;
    // An issue dropped back on the board leaves the canvas and returns to its column.
    if (node.item?.kind === "issue" && boardNode) {
      const centre = { x: x + node.width / 2, y: y + node.height / 2 };
      if (contains(boardNode, centre)) {
        save(data.nodes.filter((n) => n.id !== id));
        return;
      }
    }
    save(data.nodes.map((n) => (n.id === id ? { ...n, x, y } : n)));
  }

  /**
   * How small a node can be resized, so its title stays readable: sticky notes, notes, issue
   * cards and web links resize, the rest keep their size.
   */
  function minSize(n: Placed): { w: number; h: number } | null {
    if (n.raw.type === "text") return { w: 120, h: 80 };
    if (n.raw.item?.kind === "note") return { w: 160, h: 90 };
    if (n.raw.item?.kind === "issue") return { w: 200, h: 96 };
    if (n.raw.item?.kind === "link") return { w: 200, h: 72 };
    return null;
  }

  function resizedNode(id: string, rect: { x: number; y: number; w: number; h: number }) {
    if (!data) return;
    save(data.nodes.map((n) => (n.id === id ? { ...n, x: rect.x, y: rect.y, width: rect.w, height: rect.h } : n)));
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

  /** A press selects (and shows the handle for drawing an arrow); a double press opens. */
  function select(id: string | null) {
    selected = id;
  }

  function open(id: string) {
    selected = id;
    if (data?.nodes.find((n) => n.id === id)?.type === "text") {
      editing = id;
      return;
    }
    const item = data?.nodes.find((n) => n.id === id)?.item;
    if (!item) return;
    if (item.kind === "issue") app.select({ kind: "issue", node: item.card.node });
    else if (item.kind === "note") app.select({ kind: "note", path: item.path });
    else if (item.kind === "image" || item.kind === "file") void app.run(() => app.api.open(item.path));
    else if (item.kind === "link") void app.run(() => app.api.open(item.uri));
  }

  const selectedNode = $derived(data?.nodes.find((n) => n.id === selected) ?? null);

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
    const project = data.project;
    await app.run(async () => {
      const note = await app.api.createNote(project.folder, "Untitled");
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

  async function connect(type: string) {
    const link = connecting;
    connecting = null;
    if (!link) return;
    const from = uriOf(link.from);
    const to = uriOf(link.to);
    if (!from || !to) {
      app.say("Only notes, files, links and issues can be connected.");
      return;
    }
    await app.run(() => app.api.relate(from, to, type));
  }

  /** The relationships two nodes can have, and where each is kept. */
  const connectOptions = $derived.by(() => {
    if (!connecting || !data) return [];
    const cardOf = (id: string) => {
      const item = data!.nodes.find((n) => n.id === id)?.item;
      return item?.kind === "issue" ? item.card : null;
    };
    const [a, b] = [cardOf(connecting.from), cardOf(connecting.to)];
    const relates = { type: "relates", label: "Relates to", where: "Kept in relations.jsonl" };
    if (a && b && Boolean(a.local) === Boolean(b.local)) {
      const where = a.local ? "Kept in the issue files" : "On GitHub";
      return [
        { type: "blocks", label: "Blocks", where },
        { type: "parent-of", label: "Parent of, as a sub-issue", where },
        relates,
      ];
    }
    return [
      { type: "blocks", label: "Blocks", where: "Kept in relations.jsonl" },
      { type: "derived-from", label: "Derived from", where: "Kept in relations.jsonl" },
      relates,
    ];
  });

  function removeSelected(): boolean {
    if (!data) return false;
    if (selectedArrow) {
      const arrow = data.arrows.find((a) => a.id === selectedArrow);
      selectedArrow = null;
      if (arrow) void app.run(() => app.api.unrelate(arrow as Arrow));
      return true;
    }
    if (selected) {
      const node = data.nodes.find((n) => n.id === selected);
      if (!node || node.item?.kind === "board") return false;
      selected = null;
      app.select(null);
      save(data.nodes.filter((n) => n.id !== node.id));
      return true;
    }
    return false;
  }

  $effect(() => {
    projectKeys.remove = removeSelected;
    projectKeys.note = () => void newNote(centre());
    projectKeys.paste = paste;
    projectKeys.files = () => (filesOpen = !filesOpen);
    projectKeys.sticky = () => newSticky(centre());
    return () => {
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
      onselect={select}
      onopen={open}
      onselectarrow={(id) => (selectedArrow = id)}
      onmoved={moved}
      onconnect={(from, to) => (connecting = { from, to })}
      onfiles={(paths, at) => void files(paths, at)}
      accept={(d) => isCard(d) || isFileDrag(d)}
      ondropdata={dropped}
    >
      {#snippet node(n, zoom)}
        {#if n.raw.item?.kind === "board"}
          <BoardNode board={data!.board} project={data!.project} {zoom} {placed} />
        {:else if n.raw.type === "text"}
          <StickyNote
            text={n.raw.text ?? ""}
            color={n.raw.color}
            {zoom}
            selected={selected === n.id}
            editing={editing === n.id}
            onchange={(text) => setText(n.id, text)}
            ondone={() => (editing = null)}
          />
        {:else if n.raw.item}
          <ItemNode item={n.raw.item} {zoom} selected={selected === n.id} />
        {/if}
      {/snippet}
    </Viewport>

    <Toolbar {tools} bind:view {size} onfit={fit} />

    {#if filesOpen}
      <Presence
        in={[fly, { x: -24, duration: 220 }]}
        out={[fade, { duration: 140 }]}
        style={{ position: "absolute", top: 12, left: 12, bottom: 12, direction: "row" }}
      >
        <Files project={data.project} onclose={() => (filesOpen = false)} />
      </Presence>
    {/if}

    {#if selectedNode?.type === "text" && editing !== selectedNode.id}
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
