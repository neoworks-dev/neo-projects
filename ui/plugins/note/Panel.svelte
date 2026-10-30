<script lang="ts">
  import { app } from "../../lib/state.svelte.ts";
  import NotePanel from "./NotePanel.svelte";

  const path = $derived(app.selection?.kind === "note" ? app.selection.path : null);
</script>

{#if path}
  {#key path}
    <NotePanel {path} />
  {/key}
{/if}
