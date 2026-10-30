<script lang="ts">
  // One entry of a slot, drawn in the context of the plugin that registered it.
  import { untrack, type Component } from "svelte";
  import { provideCtx } from "./context.ts";
  import type { Occupant } from "./registry.ts";

  let { occupant, props }: { occupant: Occupant; props?: Record<string, unknown> } = $props();

  // Fixed for the component: another entry is another `occupant`, drawn anew.
  const Drawn = untrack(() => occupant.component as Component<Record<string, unknown>>);
  provideCtx(untrack(() => occupant.ctx as Parameters<typeof provideCtx>[0]));
</script>

<Drawn {...props} />
