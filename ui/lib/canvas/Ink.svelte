<script lang="ts">
  // One arrow, on a canvas of its own just big enough to hold it. The host draws a canvas on the
  // CPU and uploads it whole when it changes, so one small canvas per arrow keeps a drag cheap
  // where one window-sized canvas for all of them was redrawn and uploaded on every pointer move.
  import { Canvas } from "@neoworks/svelte-gpui/ui";
  import { arrowHead, type Point } from "./geometry.ts";

  interface Props {
    /** The points to draw through, in the coordinates of the layer it sits in. */
    points: Point[];
    color: string;
    width?: number;
    head?: boolean;
  }

  let { points, color, width = 1.75, head = true }: Props = $props();

  const pad = 12;
  // The canvas grows in steps, so a drag redraws it in place instead of resizing it each frame:
  // a canvas keeps its pixels when resized, so what was drawn before the resize would be clipped.
  const step = 96;
  const left = $derived(Math.min(...points.map((p) => p.x)) - pad);
  const top = $derived(Math.min(...points.map((p) => p.y)) - pad);
  const w = $derived(Math.ceil((Math.max(...points.map((p) => p.x)) - left + pad) / step) * step);
  const h = $derived(Math.ceil((Math.max(...points.map((p) => p.y)) - top + pad) / step) * step);

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
    const [l, r] = arrowHead(before, tip, 9);
    ink.line(tip.x, tip.y, l.x, l.y, { color, width });
    ink.line(tip.x, tip.y, r.x, r.y, { color, width });
  });
</script>

{#key `${w}x${h}`}
  <Canvas bind:this={ink} style={{ position: "absolute", left, top, width: w, height: h }} />
{/key}
