<script lang="ts">
  // Where you are, at the top bar's start: Projects, then the open project.
  import { Button, Text } from "@neoworks/svelte-gpui/ui";
  import { app } from "../../lib/state.svelte.ts";
  import { c, ghost, ghostHover } from "../../lib/theme.ts";
  import { dialogs } from "./dialogs.svelte.ts";

  const project = $derived(app.route.view === "project" ? app.route.project : null);
</script>

<Button style={{ ...ghost, color: project ? c.muted : c.text, fontSize: 13 }} hover={ghostHover} onpress={() => app.open({ view: "overview" })}>
  <Text style={{ fontWeight: "semibold" }}>Projects</Text>
</Button>
{#if project}
  <Text style={{ color: c.faint }}>/</Text>
  <Text style={{ fontSize: 13, fontWeight: "semibold", paddingX: 6 }}>{project.split("/").pop()}</Text>
{:else}
  <Button style={ghost} hover={ghostHover} onpress={() => (dialogs.addProjects = true)}>
    <Text>+ Add projects</Text>
  </Button>
{/if}
