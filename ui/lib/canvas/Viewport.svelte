<script lang="ts" module>
  export interface VNode {
    id: string;
    x: number;
    y: number;
    w: number;
    h: number;
  }

  export interface VArrow {
    id: string;
    from: string;
    to: string;
    label?: string | null;
    color?: string;
  }

  /** What the viewport shows: `x`, `y` is the world point at its top left. */
  export interface ViewState {
    x: number;
    y: number;
    zoom: number;
  }

  /** Where a dragged node was let go. */
  export interface Move {
    id: string;
    x: number;
    y: number;
  }

  /** A line drawn on the background, through world points. */
  export interface VStroke {
    id: string;
    color: string;
    width: number;
    points: { x: number; y: number }[];
  }

  /** What a drag on the background does instead of selecting: draw a stroke, or erase strokes. */
  export type DrawTool = "pen" | "eraser";
</script>

<script lang="ts" generics="N extends VNode, A extends VArrow">
  // An infinite canvas: nodes placed in world coordinates, drawn at the view's zoom, with arrows
  // between them and strokes drawn on the background under them. With a drawing tool on, a drag
  // on the background draws a stroke or erases the strokes it crosses. Drag the background to select the nodes in a box (Shift adds to the selection),
  // drag it with the middle or right button or turn the wheel to pan, Ctrl+wheel to zoom. Press a
  // node to select it, Shift+press to add or take it out. Drag a node to move it with the rest of
  // the selection, or its sides and corners to resize it. Near a node, a dot slides out past the
  // edge nearest the pointer; drag it onto another node to connect them, from every selected node
  // when it is one of them. Files from other programs and in-app drags (a card from a board) can be
  // dropped anywhere.
  import {
    DropTarget,
    Presence,
    Text,
    View,
    type DragInfo,
    type FileDragEvent,
    type Layout,
    type PointerEvent,
    type WheelEvent,
  } from "@neoworks/svelte-gpui/ui";
  import type { Snippet } from "svelte";
  import { fly } from "svelte/transition";
  import { c } from "../theme.ts";
  import {
    arrowPath,
    contains,
    dotPoint,
    grabZone,
    handles,
    midpoint,
    near,
    nearestSide,
    resized,
    spanned,
    touches,
    type Handle,
    type Point,
    type Rect,
    type Side,
  } from "./geometry.ts";
  import GuideLine from "./GuideLine.svelte";
  import Ink from "./Ink.svelte";
  import { snap, type Guide } from "./snap.ts";
  import { thinned, touchesStroke } from "./strokes.ts";
  import { bounds } from "./unfold.ts";

  interface Props {
    nodes: N[];
    arrows: A[];
    view: ViewState;
    selected?: string[];
    selectedArrow?: string | null;
    /** Draws a node's content, at `zoom`; it is laid out in a box of the node's size times zoom. */
    node: Snippet<[N, number]>;
    /** Whether a node can be moved by dragging it. */
    movable?: (node: N) => boolean;
    /** Whether arrows can be drawn from the selected node. */
    linkable?: boolean;
    /** The smallest a node can be resized to, or null when it can't be resized. */
    minSize?: (node: N) => { w: number; h: number } | null;
    onresized?: (id: string, rect: Rect) => void;
    /** The nodes to select now, after a press, a Shift+press or a box on the background. */
    onselect?: (ids: string[]) => void;
    /** A press on the background that didn't drag a box. */
    onbackground?: () => void;
    /** A double press on a node. */
    onopen?: (id: string) => void;
    /** A node pressed and released without moving it or holding Shift. */
    onpress?: (id: string) => void;
    onselectarrow?: (id: string | null) => void;
    /** The dragged nodes were let go where `moves` says; `grabbed` is the one the pointer held. */
    onmoved?: (moves: Move[], grabbed: string) => void;
    /** The node held in a drag is now at `x`, `y`; `onmoved` follows when it is let go. */
    ondrag?: (id: string, x: number, y: number) => void;
    /** An arrow drawn from the nodes `from` onto the node `to`. */
    onconnect?: (from: string[], to: string) => void;
    /** A right press on a node, for the nodes `ids` it acts on, at a point in the window. */
    oncontext?: (ids: string[], windowX: number, windowY: number) => void;
    onfiles?: (paths: string[], at: Point) => void;
    /** Which in-app drags the canvas takes, and what to do with one dropped at a world point. */
    accept?: (data: unknown) => boolean;
    ondropdata?: (data: unknown, at: Point) => void;
    /** The lines drawn on the background, under everything else. */
    strokes?: VStroke[];
    /** Set while drawing: a drag on the background draws or erases instead of selecting. */
    tool?: DrawTool | null;
    /** The colour and width (world) the pen draws with. */
    pen?: { color: string; width: number };
    /** A stroke was drawn through these world points. */
    ondrew?: (points: Point[]) => void;
    /** The eraser was dragged across the strokes `ids`. */
    onerase?: (ids: string[]) => void;
  }

  let {
    nodes,
    arrows,
    view = $bindable(),
    selected = [],
    selectedArrow = null,
    node,
    movable = () => true,
    linkable = false,
    minSize = () => null,
    onresized,
    onselect,
    onbackground,
    onopen,
    onpress,
    onselectarrow,
    onmoved,
    ondrag,
    onconnect,
    oncontext,
    onfiles,
    accept = () => false,
    ondropdata,
    strokes = [],
    tool = null,
    pen = { color: "#ffffff", width: 3 },
    ondrew,
    onerase,
  }: Props = $props();

  let box = $state<Layout>({ x: 0, y: 0, width: 0, height: 0 });
  /**
   * The nodes being dragged, until they are dropped: the one held, where each started (world),
   * and how far they have come.
   */
  let drag = $state<{
    id: string;
    starts: Map<string, Point>;
    /** The box around all of them where they started. */
    box: Rect;
    dx: number;
    dy: number;
    from: Point;
    moved: boolean;
    shift: boolean;
  } | null>(null);
  /** The background being dragged: where the view and the pointer were when it started. */
  let pan: { view: ViewState; from: Point } | null = null;
  let panning = $state(false);
  /** The lines the dragged nodes are aligned on with others, in world coordinates. */
  let guides = $state<Guide[]>([]);
  /** How close an edge or centre comes to another's, in screen pixels, before it snaps to it. */
  const SNAP = 6;
  /**
   * A selection box being dragged on the background, from `from` to `to` (viewport coordinates),
   * and what was selected before it, which Shift keeps.
   */
  let lasso = $state<{ from: Point; to: Point; kept: string[] } | null>(null);
  /** An arrow being drawn: from which nodes, to where the pointer is (viewport coordinates). */
  let link = $state<{ from: string[]; to: Point } | null>(null);
  /** The stroke being drawn, through world points. */
  let drawing = $state<Point[] | null>(null);
  /** The strokes the eraser has gone across so far, hidden until it is let go. */
  let erasing = $state<string[] | null>(null);
  /** How close the eraser comes to a stroke, in screen pixels, to take it. */
  const ERASER = 8;
  let pointer: Point | null = null;

  /** How close the pointer comes to a node, in screen pixels, before its arrow dot shows. */
  const NEAR = 20;
  /** How far the dot slides out past the node's edge, in screen pixels. */
  const DOT_OUT = 16;
  /** The node the pointer is near and the side it is nearest, where the arrow dot is out. */
  let nearby = $state<{ id: string; side: Side } | null>(null);
  /** The pointer is on the dot's grab area, so a press there starts an arrow. */
  let dotHot = $state(false);

  function track(e: PointerEvent) {
    if (!linkable || link || tool) return;
    const at = toWorld(local(e));
    const found = near(shown, at, NEAR / zoom);
    if (!found) {
      if (nearby) nearby = null;
      dotHot = false;
      return;
    }
    const side = nearestSide(found, at);
    if (nearby?.id === found.id && nearby.side === side) return;
    // A dot taken away under the pointer never hears it leave.
    nearby = { id: found.id, side };
    dotHot = false;
  }

  const zoom = $derived(view.zoom);
  const toWorld = (p: Point): Point => ({ x: p.x / zoom + view.x, y: p.y / zoom + view.y });

  // Nodes sit in one layer at world position times zoom, and panning only moves that layer, so a
  // pan changes one style however many nodes there are.
  const shift = $derived<[number, number]>([-view.x * zoom, -view.y * zoom]);

  /** A node being resized by one of its handles: its box when it started, and as it is now. */
  let resize = $state<{ id: string; handle: Handle; start: Rect; from: Point; rect: Rect } | null>(null);

  function placed(n: N): N {
    const start = drag?.starts.get(n.id);
    if (drag && start) return { ...n, x: start.x + drag.dx, y: start.y + drag.dy };
    if (resize && resize.id === n.id) return { ...n, ...resize.rect };
    return n;
  }

  const shown = $derived(nodes.map(placed));
  const byId = $derived(new Map(shown.map((n) => [n.id, n])));

  // What is drawn: the view and a margin, in steps, so panning only adds or drops nodes now and
  // then instead of filtering them on every frame.
  const cell = 400;
  const cullKey = $derived.by(() => {
    if (box.width === 0) return "";
    const x0 = Math.floor((view.x - cell / zoom) / (cell / zoom));
    const y0 = Math.floor((view.y - cell / zoom) / (cell / zoom));
    const x1 = Math.ceil((view.x + (box.width + cell) / zoom) / (cell / zoom));
    const y1 = Math.ceil((view.y + (box.height + cell) / zoom) / (cell / zoom));
    return `${x0},${y0},${x1},${y1},${zoom}`;
  });
  const visible = $derived.by(() => {
    if (!cullKey) return shown;
    const [x0, y0, x1, y1] = cullKey.split(",").map(Number);
    const unit = cell / zoom;
    return shown.filter(
      (n) => n.x + n.w > x0 * unit && n.y + n.h > y0 * unit && n.x < x1 * unit && n.y < y1 * unit,
    );
  });

  // An arrow's path is kept while neither end moved, so a drag only redraws the arrows on the
  // dragged node: a new array would make every arrow's canvas draw again.
  let paths = new Map<string, { key: string; points: Point[]; mid: Point }>();

  /** Arrows in layer coordinates (world times zoom), with their midpoints for labels. */
  const lines = $derived.by(() => {
    const next = new Map<string, { key: string; points: Point[]; mid: Point }>();
    const out = arrows.flatMap((a) => {
      const from = byId.get(a.from);
      const to = byId.get(a.to);
      if (!from || !to) return [];
      const key = `${from.x},${from.y},${from.w},${from.h},${to.x},${to.y},${to.w},${to.h},${zoom}`;
      let path = paths.get(a.id);
      if (path?.key !== key) {
        const world = arrowPath(from, to);
        if (!world) return [];
        const points = world.map((p) => ({ x: p.x * zoom, y: p.y * zoom }));
        path = { key, points, mid: midpoint(points) };
      }
      next.set(a.id, path);
      return [{ arrow: a, points: path.points, mid: path.mid }];
    });
    paths = next;
    return out;
  });

  /** The arrows being drawn, one from each node they start at, in layer coordinates. */
  const linkLines = $derived.by(() => {
    if (!link) return [];
    const at = toWorld(link.to);
    return link.from.flatMap((id) => {
      const from = byId.get(id);
      if (!from) return [];
      const world = arrowPath(from, { x: at.x, y: at.y, w: 0, h: 0 });
      if (!world) return [];
      return [{ id, points: world.map((p) => ({ x: p.x * zoom, y: p.y * zoom })) }];
    });
  });

  /** The selection box, in viewport coordinates. */
  const lassoBox = $derived(lasso ? spanned(lasso.from, lasso.to) : null);

  const toLayer = (points: Point[]) => points.map((p) => ({ x: p.x * zoom, y: p.y * zoom }));

  /** The strokes on the background, without those the eraser is taking, in layer coordinates. */
  const inked = $derived(
    strokes.filter((s) => !erasing?.includes(s.id)).map((s) => ({ ...s, points: toLayer(s.points) })),
  );

  // Pointer moves come faster than frames (a 1000 Hz mouse sends sixteen a frame); only the last
  // one before each frame is applied.
  let pending: (() => void) | null = null;
  let frame = 0;
  function later(apply: () => void) {
    pending = apply;
    if (frame) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      const run = pending;
      pending = null;
      run?.();
    });
  }
  function flush() {
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
    const run = pending;
    pending = null;
    run?.();
  }

  function local(e: PointerEvent): Point {
    return { x: e.windowX - box.x, y: e.windowY - box.y };
  }

  /** Every point the pen has passed through, which `drawing` catches up with once a frame. */
  let penPoints: Point[] = [];

  function drawDown(e: PointerEvent) {
    nearby = null;
    const at = toWorld(local(e));
    if (tool === "pen") {
      penPoints = [at];
      drawing = penPoints;
      return;
    }
    erasing = [];
    erase(at);
  }

  function drawMove(e: PointerEvent) {
    const at = toWorld(local(e));
    if (erasing) {
      erase(at);
      return;
    }
    penPoints.push(at);
    later(() => (drawing = [...penPoints]));
  }

  function drawUp() {
    flush();
    if (drawing) {
      const points = thinned(penPoints, 1.5 / zoom);
      if (points.length > 1) ondrew?.(points);
    }
    if (erasing && erasing.length > 0) onerase?.(erasing);
    drawing = null;
    erasing = null;
    eraserAt = null;
    penPoints = [];
  }

  /** Where the eraser was last, in world coordinates. */
  let eraserAt: Point | null = null;

  /** Takes the strokes the eraser touches on its way from where it was to `at`. */
  function erase(at: Point) {
    const from = eraserAt ?? at;
    eraserAt = at;
    const taken = erasing ?? [];
    const reach = ERASER / zoom;
    const hit = strokes.filter((s) => !taken.includes(s.id) && touchesStroke(s.points, from, at, reach + s.width / 2));
    if (hit.length > 0) erasing = [...taken, ...hit.map((s) => s.id)];
  }

  function backgroundDown(e: PointerEvent) {
    onselectarrow?.(null);
    if (e.button === 0 && tool) {
      drawDown(e);
      return;
    }
    if (e.button === 0) {
      lasso = { from: local(e), to: local(e), kept: e.shift ? selected : [] };
      nearby = null;
      if (!e.shift) onselect?.([]);
      return;
    }
    pan = { view: { ...view }, from: { x: e.windowX, y: e.windowY } };
    panning = true;
  }

  /** Selects what was kept and every node the box touches. */
  function lassoSelect() {
    if (!lassoBox) return;
    const corner = toWorld(lassoBox);
    const area = { ...corner, w: lassoBox.w / zoom, h: lassoBox.h / zoom };
    const kept = lasso?.kept ?? [];
    const inside = shown.filter((n) => touches(n, area) && !kept.includes(n.id)).map((n) => n.id);
    onselect?.([...kept, ...inside]);
  }

  function backgroundMove(e: PointerEvent) {
    pointer = local(e);
    if (drawing || erasing) {
      drawMove(e);
      return;
    }
    if (lasso) {
      const to = local(e);
      later(() => {
        if (!lasso) return;
        lasso = { ...lasso, to };
        lassoSelect();
      });
      return;
    }
    track(e);
    const start = pan;
    if (!start) return;
    const { windowX, windowY } = e;
    later(() => {
      view = {
        ...view,
        x: start.view.x - (windowX - start.from.x) / zoom,
        y: start.view.y - (windowY - start.from.y) / zoom,
      };
    });
  }

  function backgroundUp() {
    if (drawing || erasing) {
      drawUp();
      return;
    }
    flush();
    pan = null;
    panning = false;
    if (lasso && lassoBox && lassoBox.w + lassoBox.h <= 3) onbackground?.();
    lasso = null;
  }

  function wheel(e: WheelEvent) {
    const { dx, dy } = e;
    if (e.ctrl || e.meta) {
      const at = pointer ?? { x: box.width / 2, y: box.height / 2 };
      later(() => {
        const before = toWorld(at);
        const next = Math.min(2.5, Math.max(0.15, zoom * Math.exp(-dy / 400)));
        view = { x: before.x - at.x / next, y: before.y - at.y / next, zoom: next };
      });
    } else {
      later(() => (view = { ...view, x: view.x + dx / zoom, y: view.y + dy / zoom }));
    }
  }

  /** A right press acts on the selection when the node is part of it, and else selects the node alone. */
  function nodeContext(n: N, e: PointerEvent) {
    let ids = [n.id];
    if (selected.includes(n.id)) ids = selected;
    else onselect?.(ids);
    oncontext?.(ids, e.windowX, e.windowY);
  }

  function nodeDown(n: N, e: PointerEvent) {
    if (e.button === 2) {
      nodeContext(n, e);
      return;
    }
    if (e.button !== 0) return;
    if (e.clicks === 2) {
      onopen?.(n.id);
      return;
    }
    onselectarrow?.(null);
    let group = [n.id];
    if (e.shift && selected.includes(n.id)) {
      onselect?.(selected.filter((id) => id !== n.id));
      return;
    }
    if (e.shift) {
      group = [...selected, n.id];
      onselect?.(group);
    } else if (selected.includes(n.id)) {
      group = selected;
    } else {
      onselect?.(group);
    }
    if (!movable(n)) return;
    const moving = shown.filter((m) => group.includes(m.id) && movable(m));
    const starts = new Map(moving.map((m): [string, Point] => [m.id, { x: m.x, y: m.y }]));
    const box = bounds(moving);
    if (!box) return;
    drag = { id: n.id, starts, box, dx: 0, dy: 0, from: { x: e.windowX, y: e.windowY }, moved: false, shift: e.shift };
  }

  function nodeMove(e: PointerEvent) {
    track(e);
    if (!drag) return;
    const { windowX, windowY, alt } = e;
    later(() => {
      if (!drag) return;
      let dx = (windowX - drag.from.x) / zoom;
      let dy = (windowY - drag.from.y) / zoom;
      const moved = drag.moved || Math.abs(dx) + Math.abs(dy) > 3 / zoom;
      // The nodes' edges and centres snap to those of the nodes in view; Alt places them freely.
      guides = [];
      if (moved && !alt) {
        const starts = drag.starts;
        const others = visible.filter((m) => !starts.has(m.id));
        const aligned = snap({ ...drag.box, x: drag.box.x + dx, y: drag.box.y + dy }, others, SNAP / zoom);
        dx += aligned.dx;
        dy += aligned.dy;
        guides = aligned.guides;
      }
      drag = { ...drag, dx: Math.round(dx), dy: Math.round(dy), moved };
      const start = drag.starts.get(drag.id);
      if (moved && start) ondrag?.(drag.id, start.x + drag.dx, start.y + drag.dy);
    });
  }

  function nodeUp() {
    flush();
    const done = drag;
    drag = null;
    guides = [];
    if (!done) return;
    if (done.moved) {
      const moves = [...done.starts].map(([id, start]) => ({ id, x: start.x + done.dx, y: start.y + done.dy }));
      onmoved?.(moves, done.id);
      return;
    }
    // Shift+press only adds to the selection; a plain press on one of several selects just it.
    if (done.shift) return;
    if (done.starts.size > 1) onselect?.([done.id]);
    onpress?.(done.id);
  }

  function resizeDown(n: N, handle: Handle, e: PointerEvent) {
    if (e.button !== 0) return;
    const start = { x: n.x, y: n.y, w: n.w, h: n.h };
    resize = { id: n.id, handle, start, from: { x: e.windowX, y: e.windowY }, rect: start };
  }

  function resizeMove(n: N, e: PointerEvent) {
    const min = minSize(n);
    if (!resize || !min) return;
    const { windowX, windowY, shift } = e;
    later(() => {
      if (!resize) return;
      const dx = (windowX - resize.from.x) / zoom;
      const dy = (windowY - resize.from.y) / zoom;
      resize = { ...resize, rect: resized(resize.start, resize.handle, dx, dy, min, shift) };
    });
  }

  function resizeUp() {
    flush();
    const done = resize;
    resize = null;
    if (!done) return;
    const { rect, start } = done;
    if (rect.x !== start.x || rect.y !== start.y || rect.w !== start.w || rect.h !== start.h) onresized?.(done.id, rect);
  }

  function linkDown(n: N, e: PointerEvent) {
    if (selected.includes(n.id)) link = { from: selected, to: local(e) };
    else link = { from: [n.id], to: local(e) };
  }

  function linkMove(e: PointerEvent) {
    const to = local(e);
    later(() => {
      if (link) link = { ...link, to };
    });
  }

  function linkUp() {
    flush();
    if (!link) return;
    const at = toWorld(link.to);
    const from = link.from;
    const target = [...shown].reverse().find((n) => !from.includes(n.id) && contains(n, at));
    if (target) onconnect?.(link.from, target.id);
    link = null;
  }

  function dropped(info: DragInfo<unknown>) {
    ondropdata?.(info.data, toWorld({ x: info.x, y: info.y }));
  }

  function files(e: FileDragEvent) {
    onfiles?.(e.paths, toWorld({ x: e.windowX - box.x, y: e.windowY - box.y }));
  }
</script>

<View
  style={{ flex: 1, overflow: "hidden" }}
  onlayout={(l) => (box = l)}
  ondrop={files}
  onhover={(inside) => {
    if (!inside) nearby = null;
  }}
>
  <DropTarget {accept} ondrop={dropped} style={{ position: "absolute", inset: 0 }}>
    <View
      style={{ position: "absolute", inset: 0, background: c.bg, cursor: panning ? "grabbing" : tool ? "crosshair" : "default" }}
      ondown={backgroundDown}
      onmove={backgroundMove}
      onup={backgroundUp}
      onwheel={wheel}
    />
  </DropTarget>
  <View style={{ position: "absolute", left: 0, top: 0, width: 0, height: 0, translate: shift }}>
    {#each inked as stroke (stroke.id)}
      <Ink points={stroke.points} color={stroke.color} width={stroke.width * zoom} head={false} />
    {/each}
    {#if drawing && drawing.length > 1}
      <Ink points={toLayer(drawing)} color={pen.color} width={pen.width * zoom} head={false} />
    {/if}
    {#each lines as { arrow, points } (arrow.id)}
      {@const chosen = arrow.id === selectedArrow}
      <Ink {points} color={chosen ? c.accent : (arrow.color ?? c.borderStrong)} width={chosen ? 2.5 : 1.75} />
    {/each}
    {#each linkLines as line (line.id)}
      <Ink points={line.points} color={c.accent} width={2} head={false} />
    {/each}
    {#each lines as { arrow, mid } (arrow.id)}
      {@const chosen = arrow.id === selectedArrow}
      <View
        style={{
          position: "absolute",
          left: mid.x - (arrow.label ? arrow.label.length * 3.1 + 7 : 5),
          top: mid.y - (arrow.label ? 10 : 5),
          direction: "row",
        }}
        onpress={() => onselectarrow?.(chosen ? null : arrow.id)}
      >
        {#if arrow.label}
          <View
            style={{
              paddingX: 6,
              height: 20,
              justify: "center",
              radius: 10,
              background: chosen ? c.accent : c.raised,
              borderWidth: 1,
              borderColor: chosen ? c.accent : c.border,
              transition: 120,
            }}
          >
            <Text style={{ fontSize: 11, color: chosen ? c.onAccent : c.muted, nowrap: true }}>{arrow.label}</Text>
          </View>
        {:else}
          <View
            style={{ width: 10, height: 10, radius: 5, background: chosen ? c.accent : c.raised, borderWidth: 1, borderColor: chosen ? c.accent : c.borderStrong }}
            hover={{ background: c.accent }}
          />
        {/if}
      </View>
    {/each}
    {#each visible as n (n.id)}
      {#if linkable && nearby?.id === n.id && !drag && !resize}
        <!-- The dot for drawing an arrow, on the side nearest the pointer, drawn before the node
             so it slides out from under that edge. Keyed by side: moving to another side slides
             this one back in and the next one out. Its grab area is larger than the dot, and the
             dot fills while the pointer is on it. -->
        {#each [nearby.side] as side (side)}
          {@const dot = dotPoint(n, side, zoom, DOT_OUT)}
          <Presence
            in={[fly, { ...dot.back, duration: 160 }]}
            out={[fly, { ...dot.back, duration: 120 }]}
            style={{ position: "absolute", left: dot.at.x - 11, top: dot.at.y - 11 }}
          >
            <View
              style={{ size: 22, align: "center", justify: "center", cursor: "crosshair" }}
              onhover={(inside) => (dotHot = inside)}
              ondown={(e) => linkDown(n, e)}
              onmove={linkMove}
              onup={linkUp}
            >
              <View
                style={{
                  size: 11,
                  radius: 6,
                  background: dotHot || link ? c.accent : c.surface,
                  borderWidth: 2,
                  borderColor: c.accent,
                  transition: 120,
                }}
              />
            </View>
          </Presence>
        {/each}
      {/if}
      <View
        style={{ position: "absolute", left: n.x * zoom, top: n.y * zoom, width: n.w * zoom, height: n.h * zoom }}
        ondown={(e) => nodeDown(n, e)}
        onmove={nodeMove}
        onup={nodeUp}
      >
        {@render node(n, zoom)}
      </View>
      {#if minSize(n)}
        <!-- Invisible strips on the sides and squares on the corners: dragging there resizes. -->
        {#each handles as handle (handle)}
          {@const zone = grabZone(n, handle, zoom)}
          <View
            style={{
              position: "absolute",
              left: zone.x,
              top: zone.y,
              width: zone.w,
              height: zone.h,
              cursor: handle.length === 2 ? "default" : handle === "e" || handle === "w" ? "col-resize" : "row-resize",
            }}
            ondown={(e) => resizeDown(n, handle, e)}
            onmove={(e) => resizeMove(n, e)}
            onup={resizeUp}
          />
        {/each}
      {/if}
    {/each}
    <!-- Keyed by axis, so a guide that moves to another line eases there instead of starting over. -->
    {#each guides as guide (guide.axis)}
      <GuideLine {guide} {zoom} />
    {/each}
  </View>
  {#if lassoBox}
    <View
      style={{
        position: "absolute",
        left: lassoBox.x,
        top: lassoBox.y,
        width: lassoBox.w,
        height: lassoBox.h,
        background: c.accentSoft,
        borderWidth: 1,
        borderColor: c.accent,
      }}
    />
  {/if}
</View>
