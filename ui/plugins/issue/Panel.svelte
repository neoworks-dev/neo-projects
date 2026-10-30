<script lang="ts">
  import { app } from "../../lib/state.svelte.ts";
  import IssuePanel from "./IssuePanel.svelte";

  const node = $derived(app.selection?.kind === "issue" ? app.selection.node : null);
</script>

{#if node}
  {#key node}
    <IssuePanel {node} />
  {/key}
{/if}
