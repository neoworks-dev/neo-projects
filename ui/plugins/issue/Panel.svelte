<script lang="ts">
  // The panel slides in from the right when an issue is first selected; moving to another issue
  // swaps its content in place.
  import { Presence } from "@neoworks/svelte-gpui/ui";
  import { cubicOut } from "svelte/easing";
  import { fly } from "svelte/transition";
  import { app } from "../../lib/state.svelte.ts";
  import IssuePanel from "./IssuePanel.svelte";

  const node = $derived(app.selection?.kind === "issue" ? app.selection.node : null);
</script>

{#if node}
  <Presence in={[fly, { x: 48, duration: 260, easing: cubicOut }]} style={{ direction: "row" }}>
    {#key node}
      <IssuePanel {node} />
    {/key}
  </Presence>
{/if}
