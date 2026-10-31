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
</script>

<script lang="ts" generics="N extends VNode, A extends VArrow">
  // An infinite canvas: nodes placed in world coordinates, drawn at the view's zoom, with arrows
  // between them. Drag the background to pan, the wheel to pan, Ctrl+wheel to zoom. Drag a node to
  // move it, or its sides and corners to resize it. Near a node, a dot slides out past the edge
  // nearest the pointer; drag it onto another node to connect them. Files
  // from other programs and in-app drags (a card from a board) can be dropped anywhere.
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
    type Handle,
    type Point,
    type Rect,
    type Side,
  } from "./geometry.ts";
  import Ink from "./Ink.svelte";

  interface Props {
    nodes: N[];
    arrows: A[];
    view: ViewState;
    selected?: string | null;
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
    onselect?: (id: string | null) => void;
    /** A double press on a node. */
    onopen?: (id: string) => void;
    /** A node pressed and released without moving it. */
    onpress?: (id: string) => void;
    onselectarrow?: (id: string | null) => void;
    onmoved?: (id: string, x: number, y: number) => void;
    onconnect?: (from: string, to: string) => void;
    onfiles?: (paths: string[], at: Point) => void;
    /** Which in-app drags the canvas takes, and what to do with one dropped at a world point. */
    accept?: (data: unknown) => boolean;
    ondropdata?: (data: unknown, at: Point) => void;
  }

  let {
    nodes,
    arrows,
    view = $bindable(),
    selected = null,
    selectedArrow = null,
    node,
    movable = () => true,
    linkable = false,
    minSize = () => null,
    onresized,
    onselect,
    onopen,
    onpress,
    onselectarrow,
    onmoved,
    onconnect,
    onfiles,
    accept = () => false,
    ondropdata,
  }: Props = $props();

  let box = $state<Layout>({ x: 0, y: 0, width: 0, height: 0 });
  /** The node being dragged and where it is now (world), until it is dropped. */
  let drag = $state<{ id: string; x: number; y: number; start: Point; from: Point; moved: boolean } | null>(null);
  /** The background being dragged: where the view and the pointer were when it started. */
  let pan: { view: ViewState; from: Point } | null = null;
  let panning = $state(false);
  /** An arrow being drawn: from which node, to where the pointer is (viewport coordinates). */
  let link = $state<{ from: string; to: Point } | null>(null);
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
    if (!linkable || link) return;
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
    if (drag && drag.id === n.id) return { ...n, x: drag.x, y: drag.y };
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

  /** The arrow being drawn, in layer coordinates. */
  const linkLine = $derived.by(() => {
    if (!link) return null;
    const from = byId.get(link.from);
    if (!from) return null;
    const at = toWorld(link.to);
    const world = arrowPath(from, { x: at.x, y: at.y, w: 0, h: 0 });
    if (!world) return null;
    return world.map((p) => ({ x: p.x * zoom, y: p.y * zoom }));
  });

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

  function backgroundDown(e: PointerEvent) {
    pan = { view: { ...view }, from: { x: e.windowX, y: e.windowY } };
    panning = true;
    onselect?.(null);
    onselectarrow?.(null);
  }

  function backgroundMove(e: PointerEvent) {
    pointer = local(e);
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
    flush();
    pan = null;
    panning = false;
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

  function nodeDown(n: N, e: PointerEvent) {
    if (e.button !== 0) return;
    if (e.clicks === 2) {
      onopen?.(n.id);
      return;
    }
    onselect?.(n.id);
    onselectarrow?.(null);
    if (!movable(n)) return;
    drag = { id: n.id, x: n.x, y: n.y, start: { x: n.x, y: n.y }, from: { x: e.windowX, y: e.windowY }, moved: false };
  }

  function nodeMove(e: PointerEvent) {
    track(e);
    if (!drag) return;
    const { windowX, windowY } = e;
    later(() => {
      if (!drag) return;
      const dx = (windowX - drag.from.x) / zoom;
      const dy = (windowY - drag.from.y) / zoom;
      const moved = drag.moved || Math.abs(dx) + Math.abs(dy) > 3 / zoom;
      drag = { ...drag, x: Math.round(drag.start.x + dx), y: Math.round(drag.start.y + dy), moved };
    });
  }

  function nodeUp() {
    flush();
    if (drag?.moved) onmoved?.(drag.id, drag.x, drag.y);
    else if (drag) onpress?.(drag.id);
    drag = null;
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
    link = { from: n.id, to: local(e) };
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
    const target = [...shown].reverse().find((n) => n.id !== link?.from && contains(n, at));
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
      style={{ position: "absolute", inset: 0, background: c.bg, cursor: panning ? "grabbing" : "default" }}
      ondown={backgroundDown}
      onmove={backgroundMove}
      onup={backgroundUp}
      onwheel={wheel}
    />
  </DropTarget>
  <View style={{ position: "absolute", left: 0, top: 0, width: 0, height: 0, translate: shift }}>
    {#each lines as { arrow, points } (arrow.id)}
      {@const chosen = arrow.id === selectedArrow}
      <Ink {points} color={chosen ? c.accent : (arrow.color ?? c.borderStrong)} width={chosen ? 2.5 : 1.75} />
    {/each}
    {#if linkLine}
      <Ink points={linkLine} color={c.accent} width={2} head={false} />
    {/if}
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
  </View>
</View>
