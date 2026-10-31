<script lang="ts">
  // One line, on a vector canvas of its own just big enough to hold it. The host keeps its lines
  // and paints them as GPU paths each frame, so redrawing it as a node is dragged uploads nothing.
  import { Canvas } from "@neoworks/svelte-gpui/ui";
  import { arrowHead, type Point } from "./geometry.ts";

  interface Props {
    /** The points to draw through, in the coordinates of the layer it sits in. */
    points: Point[];
    color: string;
    /** In the layer's units, like `head`. */
    width?: number;
    /** How long the arrow head's sides are; none when 0. */
    head?: number;
  }

  let { points, color, width = 1.75, head = 0 }: Props = $props();

  const pad = $derived(head + width);
  const left = $derived(Math.min(...points.map((p) => p.x)) - pad);
  const top = $derived(Math.min(...points.map((p) => p.y)) - pad);
  const w = $derived(Math.max(...points.map((p) => p.x)) - left + pad);
  const h = $derived(Math.max(...points.map((p) => p.y)) - top + pad);

  let ink = $state<Canvas>();

  $effect(() => {
    if (!ink || points.length < 2) return;
    const local = points.map((p) => ({ x: p.x - left, y: p.y - top }));
    ink.clear();
    ink.begin(local[0].x, local[0].y, { color, width });
    for (const p of local.slice(1)) ink.lineTo(p.x, p.y);
    ink.end();
    if (!head) return;
    // The head points along the last stretch of the path, which on a curve is nearly square to
    // the side it arrives at.
    const tip = local[local.length - 1];
    const before = local[Math.max(0, local.length - 3)];
    const [l, r] = arrowHead(before, tip, head);
    ink.line(tip.x, tip.y, l.x, l.y, { color, width });
    ink.line(tip.x, tip.y, r.x, r.y, { color, width });
  });
</script>

<Canvas bind:this={ink} vector style={{ position: "absolute", left, top, width: w, height: h }} />
