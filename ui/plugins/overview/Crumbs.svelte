<script lang="ts">
  // Where you are, at the top bar's start, as on neoworks.dev: the mark and name, a divider, then
  // Projects and the open project with where it lives (its repository, or local), and the folders
  // open within it. Each step back is a button.
  import { Button, Icon, Text, View } from "@neoworks/svelte-gpui/ui";
  import type { Project } from "../../lib/api.ts";
  import { folderSteps } from "../../lib/folders.ts";
  import { app } from "../../lib/state.svelte.ts";
  import { c } from "../../lib/theme.ts";
  import { icons } from "../../lib/icons.ts";
  const { next: caret, folder, github, mark } = icons;

  const folderName = $derived(app.route.view === "project" ? app.route.project : null);
  let project = $state<Project | null>(null);

  const folders = $derived(app.route.view === "project" ? folderSteps(app.route.project, app.route.inside) : []);

  $effect(() => {
    void app.version;
    const wanted = folderName;
    if (!wanted) {
      project = null;
      return;
    }
    void app.api.projectView(wanted).then((view) => {
      if (wanted === folderName) project = view.project;
    });
  });
</script>

{#snippet step(label: string, current: boolean, onpress: () => void)}
  <Button
    style={{ paddingX: 8, height: 28, radius: 8, color: current ? c.text : c.muted, transition: 120 }}
    hover={{ background: c.hover, color: c.text }}
    {onpress}
  >
    <Text style={{ fontSize: 14 }}>{label}</Text>
  </Button>
{/snippet}

<View style={{ direction: "row", align: "center", gap: 8, paddingLeft: 6, paddingRight: 4 }}>
  <Icon svg={mark} size={18} color={c.text} />
  <Text style={{ fontSize: 14, fontWeight: "semibold" }}>NeoWorks</Text>
  <View style={{ width: 1, height: 16, background: c.borderStrong, marginX: 4 }} />
</View>
{@render step("Projects", !folderName, () => app.open({ view: "overview" }))}
{#if folderName}
  {@const name = project?.name ?? folderName.split("/").pop() ?? ""}
  <Icon svg={caret} size={12} color={c.faint} />
  {#if folders.length > 0}
    {@render step(name, false, () => app.open({ view: "project", project: folderName }))}
  {:else}
    <Text style={{ fontSize: 14, paddingX: 6 }}>{name}</Text>
  {/if}
  {#if project}
    <View
      style={{
        direction: "row",
        align: "center",
        gap: 5,
        paddingX: 8,
        height: 22,
        radius: 11,
        borderWidth: 1,
        borderColor: c.border,
      }}
    >
      <Icon svg={project.repo ? github : folder} size={12} color={c.faint} />
      <Text style={{ fontSize: 12, color: c.muted }}>{project.repo ?? "Local"}</Text>
    </View>
  {/if}
  {#each folders as inner, i (inner.path)}
    <Icon svg={caret} size={12} color={c.faint} />
    {#if i === folders.length - 1}
      <Text style={{ fontSize: 14, paddingX: 6 }}>{inner.name}</Text>
    {:else}
      {@render step(inner.name, false, () => app.open({ view: "project", project: folderName, inside: inner.path }))}
    {/if}
  {/each}
{/if}
