<script lang="ts">
  // The project route's view, keyed by the project and the folder open in it, so switching
  // either starts fresh.
  import { app } from "../../lib/state.svelte.ts";
  import ProjectView from "./ProjectView.svelte";

  const folder = $derived(app.route.view === "project" ? app.route.project : null);
  const inside = $derived(app.route.view === "project" ? app.route.inside : undefined);
</script>

{#if folder}
  {#key `${folder}\n${inside ?? ""}`}
    <ProjectView {folder} {inside} />
  {/key}
{/if}
