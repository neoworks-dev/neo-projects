<script lang="ts">
  // Content that keeps its screen size at a point in the zoomed canvas layer: the inner view's
  // zoom undoes the layer's. Children are placed in screen pixels from the point.
  import { View } from "@neoworks/svelte-gpui/ui";
  import type { Snippet } from "svelte";
  import type { Point } from "./geometry.ts";

  let { at, zoom, children }: { at: Point; zoom: number; children: Snippet } = $props();
</script>

<View style={{ position: "absolute", left: at.x, top: at.y }}>
  <View style={{ position: "absolute", left: 0, top: 0, zoom: 1 / zoom }}>
    {@render children()}
  </View>
</View>
