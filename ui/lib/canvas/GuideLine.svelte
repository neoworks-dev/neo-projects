<script lang="ts">
  // One alignment guide while dragging: a faint hairline across the gap between two aligned nodes,
  // in world units in the zoomed layer and one screen pixel thick. It grows out from its middle
  // when it appears and eases to its new length when it moves; `transition` doesn't ease sizes,
  // so a Tween does.
  import { View } from "@neoworks/svelte-gpui/ui";
  import { cubicOut } from "svelte/easing";
  import { Tween } from "svelte/motion";
  import { c } from "../theme.ts";
  import type { Guide } from "./snap.ts";

  let { guide, zoom }: { guide: Guide; zoom: number } = $props();

  const length = new Tween(0, { duration: 160, easing: cubicOut });
  $effect(() => {
    length.target = guide.to - guide.from;
  });
  const start = $derived((guide.from + guide.to) / 2 - length.current / 2);
</script>

{#if guide.axis === "x"}
  <View style={{ position: "absolute", left: guide.at, top: start, width: 1 / zoom, height: length.current, background: c.accent, opacity: 0.4 }} />
{:else}
  <View style={{ position: "absolute", left: start, top: guide.at, width: length.current, height: 1 / zoom, background: c.accent, opacity: 0.4 }} />
{/if}
