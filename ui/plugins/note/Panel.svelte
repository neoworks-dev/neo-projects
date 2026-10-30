<script lang="ts">
  import { app } from "../../lib/state.svelte.ts";
  import NoteSheet from "./NoteSheet.svelte";

  const path = $derived(app.selection?.kind === "note" ? app.selection.path : null);
</script>

{#if path}
  {#key path}
    <NoteSheet {path} />
  {/key}
{/if}
