<script lang="ts">
  // A project's canvas: its GitHub board as one region, and around it whatever you place: issues
  // dragged out of the board, notes, files, links, local boards. Arrows are relationships: between
  // two issues they can be blocks or sub-issues on GitHub; anything else is kept in the workspace.
  import { Button, Input, Popover, Text, View } from "@neoworks/svelte-gpui/ui";
  import type { Arrow, CanvasNode, Card, ProjectView } from "../../lib/api.ts";
  import { contains, freeSpot, type Point } from "../../lib/canvas/geometry.ts";
  import Viewport, { type ViewState } from "../../lib/canvas/Viewport.svelte";
  import { app } from "../../lib/state.svelte.ts";
  import { button, buttonHover, c, field, fieldFocus, ghost, ghostHover } from "../../lib/theme.ts";
  import BoardNode from "./BoardNode.svelte";
  import ItemNode from "./ItemNode.svelte";
  import { projectKeys } from "./keys.svelte.ts";

  let { folder }: { folder: string } = $props();

  let data = $state<ProjectView | null>(null);
  let view = $state<ViewState>({ x: -40, y: -60, zoom: 0.9 });
  let selected = $state<string | null>(null);
  let selectedArrow = $state<string | null>(null);
  let connecting = $state<{ from: string; to: string } | null>(null);
  let linkUrl = $state("");
  let linkOpen = $state(false);

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

  function save(next: CanvasNode[]) {
    if (!data) return;
    data = { ...data, nodes: next };
    const path = data.path;
    void app.run(() => app.api.saveCanvas(path, next));
  }

  function add(node: CanvasNode, at: Point) {
    if (!data) return;
    const spot = freeSpot(nodes, at, node.width, node.height);
    save([...data.nodes, { ...node, x: Math.round(spot.x), y: Math.round(spot.y) }]);
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

  function select(id: string | null) {
    selected = id;
    const item = data?.nodes.find((n) => n.id === id)?.item;
    if (item?.kind === "issue") app.select({ kind: "issue", node: item.card.node });
    else if (item?.kind === "note") app.select({ kind: "note", path: item.path });
  }

  function open(id: string) {
    const item = data?.nodes.find((n) => n.id === id)?.item;
    if (!item) return;
    if (item.kind === "image" || item.kind === "file") void app.run(() => app.api.open(item.path));
    else if (item.kind === "link") void app.run(() => app.api.open(item.uri));
    else select(id);
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
    return { x: view.x + 400 / view.zoom, y: view.y + 300 / view.zoom };
  }

  async function files(paths: string[], at: Point) {
    const linked = await app.run(() => app.api.linkFiles(paths));
    if (!linked || !data) return;
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
    if (!isCard(d) || !d.card.url) return;
    add({ id: newId(), type: "link", url: d.card.url, x: at.x, y: at.y, width: 280, height: 120 }, at);
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

  const bothIssues = $derived(
    connecting !== null &&
      data?.nodes.find((n) => n.id === connecting!.from)?.item?.kind === "issue" &&
      data?.nodes.find((n) => n.id === connecting!.to)?.item?.kind === "issue",
  );

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
    return () => (projectKeys.remove = null);
  });
</script>

<View style={{ flex: 1 }}>
  {#if data}
    <Viewport
      {nodes}
      {arrows}
      bind:view
      {selected}
      {selectedArrow}
      linkable
      onselect={select}
      onopen={open}
      onselectarrow={(id) => (selectedArrow = id)}
      onmoved={moved}
      onconnect={(from, to) => (connecting = { from, to })}
      onbackground={(at) => void newNote(at)}
      onfiles={(paths, at) => void files(paths, at)}
      accept={isCard}
      ondropdata={dropped}
    >
      {#snippet node(n, zoom)}
        {#if n.raw.item?.kind === "board"}
          <BoardNode board={data!.board} project={data!.project} {zoom} {placed} />
        {:else if n.raw.item}
          <ItemNode item={n.raw.item} {zoom} selected={selected === n.id} />
        {/if}
      {/snippet}
    </Viewport>

    <View style={{ position: "absolute", top: 12, left: 12, direction: "row", gap: 6 }}>
      <Button style={button} hover={buttonHover} onpress={() => void newNote(centre())}>
        <Text>+ Note</Text>
      </Button>
      <Popover bind:open={linkOpen} side="bottom" align="start">
        {#snippet trigger({ toggle })}
          <Button style={button} hover={buttonHover} onpress={toggle}><Text>+ Link</Text></Button>
        {/snippet}
        {#snippet children({ close })}
          <View style={{ width: 340, padding: 10, gap: 8, radius: 8, background: c.surface, borderWidth: 1, borderColor: c.border }}>
            <Input
              bind:value={linkUrl}
              placeholder="https://…"
              style={field}
              focus={fieldFocus}
              onsubmit={async () => {
                const url = linkUrl.trim();
                close();
                linkUrl = "";
                if (!url) return;
                const node = await app.run(() => app.api.linkUrl(url));
                if (node) add(node, centre());
              }}
            />
            <Text style={{ fontSize: 11, color: c.faint }}>Enter to add. Files: drop them from your file manager.</Text>
          </View>
        {/snippet}
      </Popover>
      <Button
        style={button}
        hover={buttonHover}
        onpress={async () => {
          const project = data?.project;
          if (!project) return;
          const node = await app.run(() => app.api.createBoard(project.folder, "Notes board"));
          if (node) add(node, centre());
        }}
      >
        <Text>+ Notes board</Text>
      </Button>
      <Button style={ghost} hover={ghostHover} onpress={() => (view = { x: -40, y: -60, zoom: 0.9 })}>
        <Text>Reset view</Text>
      </Button>
    </View>

    {#if selectedArrow}
      {@const arrow = data.arrows.find((a) => a.id === selectedArrow)}
      {#if arrow}
        <View
          style={{ position: "absolute", bottom: 16, left: 0, right: 0, align: "center" }}
        >
          <View style={{ direction: "row", gap: 8, align: "center", padding: 8, radius: 8, background: c.surface, borderWidth: 1, borderColor: c.border }}>
            <Text style={{ fontSize: 12, color: c.muted, paddingX: 4 }}>
              {arrow.label ?? arrow.type}{arrow.source === "github" ? " · on GitHub" : ""}
            </Text>
            <Button style={button} hover={buttonHover} onpress={() => removeSelected()}>
              <Text style={{ color: c.danger }}>Remove relationship</Text>
            </Button>
          </View>
        </View>
      {/if}
    {/if}

    {#if connecting}
      <View style={{ position: "absolute", inset: 0, background: "#00000066", align: "center", justify: "center" }}>
        <View style={{ width: 360, padding: 16, gap: 10, radius: 12, background: c.surface, borderWidth: 1, borderColor: c.border }}>
          <Text style={{ fontSize: 14, fontWeight: "semibold" }}>Connect as</Text>
          {#if bothIssues}
            <Button style={button} hover={buttonHover} onpress={() => void connect("blocks")}>
              <Text>Blocks (on GitHub)</Text>
            </Button>
            <Button style={button} hover={buttonHover} onpress={() => void connect("parent-of")}>
              <Text>Parent of, as a sub-issue (on GitHub)</Text>
            </Button>
          {:else}
            <Button style={button} hover={buttonHover} onpress={() => void connect("blocks")}>
              <Text>Blocks</Text>
            </Button>
            <Button style={button} hover={buttonHover} onpress={() => void connect("derived-from")}>
              <Text>Derived from</Text>
            </Button>
          {/if}
          <Button style={button} hover={buttonHover} onpress={() => void connect("relates")}>
            <Text>Relates to</Text>
          </Button>
          <Button style={ghost} hover={ghostHover} onpress={() => (connecting = null)}>
            <Text>Cancel</Text>
          </Button>
        </View>
      </View>
    {/if}
  {/if}
</View>
