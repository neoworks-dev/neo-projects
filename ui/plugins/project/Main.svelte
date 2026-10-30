<script lang="ts">
  // The project route's view, keyed by the project so switching projects starts fresh.
  import { app } from "../../lib/state.svelte.ts";
  import ProjectView from "./ProjectView.svelte";

  const folder = $derived(app.route.view === "project" ? app.route.project : null);
</script>

{#if folder}
  {#key folder}
    <ProjectView {folder} />
  {/key}
{/if}
