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
  // move it; drag the handle on the selected node's edge onto another node to connect them. Files
  // from other programs and in-app drags (a card from a board) can be dropped anywhere.
  import {
    Canvas,
    DropTarget,
    Text,
    View,
    type DragInfo,
    type FileDragEvent,
    type Layout,
    type PointerEvent,
    type WheelEvent,
  } from "@neoworks/svelte-gpui/ui";
  import type { Snippet } from "svelte";
  import { c } from "../theme.ts";
  import { arrowEnds, arrowHead, contains, type Point } from "./geometry.ts";

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
    onselect?: (id: string | null) => void;
    /** A double press on a node. */
    onopen?: (id: string) => void;
    onselectarrow?: (id: string | null) => void;
    onmoved?: (id: string, x: number, y: number) => void;
    onconnect?: (from: string, to: string) => void;
    /** A double press on empty canvas, at that world point. */
    onbackground?: (at: Point) => void;
    onfiles?: (paths: string[], at: Point) => void;
    /** Which in-app drags the canvas takes, and what to do with one dropped at a world point. */
    accept?: (data: unknown) => boolean;
    ondropdata?: (data: unknown, at: Point) => void;
    /** Drawn above the arrows and under the nodes: what lives in world space but isn't a node. */
    overlay?: Snippet<[ViewState]>;
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
    onselect,
    onopen,
    onselectarrow,
    onmoved,
    onconnect,
    onbackground,
    onfiles,
    accept = () => false,
    ondropdata,
    overlay,
  }: Props = $props();

  let box = $state<Layout>({ x: 0, y: 0, width: 0, height: 0 });
  let ink = $state<Canvas>();
  /** The node being dragged and where it is now (world), until it is dropped. */
  let drag = $state<{ id: string; x: number; y: number; start: Point; from: Point; moved: boolean } | null>(null);
  /** The background being dragged: where the view and the pointer were when it started. */
  let pan = $state<{ view: ViewState; from: Point } | null>(null);
  /** An arrow being drawn: from which node, to where the pointer is (viewport coordinates). */
  let link = $state<{ from: string; to: Point } | null>(null);
  let pointer: Point | null = null;

  const zoom = $derived(view.zoom);
  const toScreen = (p: Point): Point => ({ x: (p.x - view.x) * zoom, y: (p.y - view.y) * zoom });
  const toWorld = (p: Point): Point => ({ x: p.x / zoom + view.x, y: p.y / zoom + view.y });

  function placed(n: N): N {
    return drag && drag.id === n.id ? { ...n, x: drag.x, y: drag.y } : n;
  }

  const shown = $derived(nodes.map(placed));
  const byId = $derived(new Map(shown.map((n) => [n.id, n])));

  /** Screen rects of the nodes that are at least partly in view. */
  const visible = $derived(
    shown.filter((n) => {
      const p = toScreen(n);
      return p.x + n.w * zoom > -40 && p.y + n.h * zoom > -40 && p.x < box.width + 40 && p.y < box.height + 40;
    }),
  );

  const midpoints = $derived(
    arrows.flatMap((a) => {
      const from = byId.get(a.from);
      const to = byId.get(a.to);
      if (!from || !to) return [];
      const ends = arrowEnds(from, to);
      if (!ends) return [];
      const mid = toScreen({ x: (ends[0].x + ends[1].x) / 2, y: (ends[0].y + ends[1].y) / 2 });
      return [{ arrow: a, mid }];
    }),
  );

  // Arrows are strokes on one Canvas under the nodes, drawn again whenever anything moves.
  $effect(() => {
    if (!ink || box.width === 0) return;
    ink.clear();
    for (const a of arrows) {
      const from = byId.get(a.from);
      const to = byId.get(a.to);
      if (!from || !to) continue;
      const ends = arrowEnds(from, to);
      if (!ends) continue;
      const [p, q] = [toScreen(ends[0]), toScreen(ends[1])];
      const chosen = a.id === selectedArrow;
      const pen = { color: chosen ? c.accent : (a.color ?? c.borderStrong), width: chosen ? 2.5 : 1.75 };
      ink.line(p.x, p.y, q.x, q.y, pen);
      const [l, r] = arrowHead(p, q, 9);
      ink.line(q.x, q.y, l.x, l.y, pen);
      ink.line(q.x, q.y, r.x, r.y, pen);
    }
    if (link) {
      const from = byId.get(link.from);
      if (from) {
        const start = toScreen({ x: from.x + from.w, y: from.y + from.h / 2 });
        ink.line(start.x, start.y, link.to.x, link.to.y, { color: c.accent, width: 2 });
      }
    }
  });

  function local(e: PointerEvent): Point {
    return { x: e.windowX - box.x, y: e.windowY - box.y };
  }

  function backgroundDown(e: PointerEvent) {
    if (e.clicks === 2) {
      onbackground?.(toWorld(local(e)));
      return;
    }
    pan = { view: { ...view }, from: { x: e.windowX, y: e.windowY } };
    onselect?.(null);
    onselectarrow?.(null);
  }

  function backgroundMove(e: PointerEvent) {
    pointer = local(e);
    if (!pan) return;
    view = {
      ...view,
      x: pan.view.x - (e.windowX - pan.from.x) / zoom,
      y: pan.view.y - (e.windowY - pan.from.y) / zoom,
    };
  }

  function wheel(e: WheelEvent) {
    if (e.ctrl || e.meta) {
      const at = pointer ?? { x: box.width / 2, y: box.height / 2 };
      const before = toWorld(at);
      const next = Math.min(2.5, Math.max(0.15, zoom * Math.exp(-e.dy / 400)));
      view = { x: before.x - at.x / next, y: before.y - at.y / next, zoom: next };
    } else {
      view = { ...view, x: view.x + e.dx / zoom, y: view.y + e.dy / zoom };
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
    if (!drag) return;
    const dx = (e.windowX - drag.from.x) / zoom;
    const dy = (e.windowY - drag.from.y) / zoom;
    const moved = drag.moved || Math.abs(dx) + Math.abs(dy) > 3 / zoom;
    drag = { ...drag, x: Math.round(drag.start.x + dx), y: Math.round(drag.start.y + dy), moved };
  }

  function nodeUp() {
    if (drag?.moved) onmoved?.(drag.id, drag.x, drag.y);
    drag = null;
  }

  function linkDown(n: N, e: PointerEvent) {
    link = { from: n.id, to: local(e) };
  }

  function linkMove(e: PointerEvent) {
    if (link) link = { ...link, to: local(e) };
  }

  function linkUp() {
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

<View style={{ flex: 1, overflow: "hidden" }} onlayout={(l) => (box = l)} ondrop={files}>
  <DropTarget {accept} ondrop={dropped} style={{ position: "absolute", inset: 0 }}>
    <Canvas
      bind:this={ink}
      style={{ position: "absolute", inset: 0, background: c.bg, cursor: pan ? "grabbing" : "default" }}
      ondown={backgroundDown}
      onmove={backgroundMove}
      onup={() => (pan = null)}
      onwheel={wheel}
    />
  </DropTarget>
  {#each midpoints as { arrow, mid } (arrow.id)}
    {@const chosen = arrow.id === selectedArrow}
    <View
      style={{
        position: "absolute",
        left: mid.x - (arrow.label ? 0 : 5),
        top: mid.y - (arrow.label ? 10 : 5),
        translate: arrow.label ? [-50, 0] : [0, 0],
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
          }}
        >
          <Text style={{ fontSize: 11, color: chosen ? "#fff" : c.muted, nowrap: true }}>{arrow.label}</Text>
        </View>
      {:else}
        <View
          style={{ width: 10, height: 10, radius: 5, background: chosen ? c.accent : c.raised, borderWidth: 1, borderColor: chosen ? c.accent : c.borderStrong }}
          hover={{ background: c.accent }}
        />
      {/if}
    </View>
  {/each}
  {#if overlay}
    {@render overlay(view)}
  {/if}
  {#each visible as n (n.id)}
    {@const p = toScreen(n)}
    <View
      style={{ position: "absolute", left: p.x, top: p.y, width: n.w * zoom, height: n.h * zoom }}
      ondown={(e) => nodeDown(n, e)}
      onmove={nodeMove}
      onup={nodeUp}
    >
      {@render node(n, zoom)}
    </View>
    {#if linkable && selected === n.id}
      <View
        style={{
          position: "absolute",
          left: p.x + n.w * zoom - 7,
          top: p.y + (n.h * zoom) / 2 - 7,
          width: 14,
          height: 14,
          radius: 7,
          background: c.accent,
          borderWidth: 2,
          borderColor: c.bg,
          cursor: "crosshair",
        }}
        hover={{ background: "#9aa6ff" }}
        ondown={(e) => linkDown(n, e)}
        onmove={linkMove}
        onup={linkUp}
      />
    {/if}
  {/each}
</View>
