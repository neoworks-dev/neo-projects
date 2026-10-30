<script lang="ts">
  // One arrow, on a canvas of its own just big enough to hold it. The host draws a canvas on the
  // CPU and uploads it whole when it changes, so one small canvas per arrow keeps a drag cheap
  // where one window-sized canvas for all of them was redrawn and uploaded on every pointer move.
  import { Canvas } from "@neoworks/svelte-gpui/ui";
  import { arrowHead, type Point } from "./geometry.ts";

  interface Props {
    /** The ends, in the coordinates of the layer it sits in. */
    from: Point;
    to: Point;
    color: string;
    width?: number;
    head?: boolean;
  }

  let { from, to, color, width = 1.75, head = true }: Props = $props();

  const pad = 12;
  // The canvas grows in steps, so a drag redraws it in place instead of resizing it each frame:
  // a canvas keeps its pixels when resized, so what was drawn before the resize would be clipped.
  const step = 96;
  const left = $derived(Math.min(from.x, to.x) - pad);
  const top = $derived(Math.min(from.y, to.y) - pad);
  const w = $derived(Math.ceil((Math.abs(to.x - from.x) + 2 * pad) / step) * step);
  const h = $derived(Math.ceil((Math.abs(to.y - from.y) + 2 * pad) / step) * step);

  let ink = $state<Canvas>();

  $effect(() => {
    if (!ink) return;
    const p = { x: from.x - left, y: from.y - top };
    const q = { x: to.x - left, y: to.y - top };
    const pen = { color, width };
    ink.clear();
    ink.line(p.x, p.y, q.x, q.y, pen);
    if (head) {
      const [l, r] = arrowHead(p, q, 9);
      ink.line(q.x, q.y, l.x, l.y, pen);
      ink.line(q.x, q.y, r.x, r.y, pen);
    }
  });
</script>

{#key `${w}x${h}`}
  <Canvas bind:this={ink} style={{ position: "absolute", left, top, width: w, height: h }} />
{/key}
