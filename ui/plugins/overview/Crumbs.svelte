<script lang="ts">
  // Where you are, at the top bar's start, as on neoworks.dev: the mark and name, a divider, then
  // Projects and the open project with where it lives (its repository, or local).
  import { Button, Icon, Text, View } from "@neoworks/svelte-gpui/ui";
  import type { Project } from "../../lib/api.ts";
  import { app } from "../../lib/state.svelte.ts";
  import { c } from "../../lib/theme.ts";
  import { icons } from "../../lib/icons.ts";
  const { next: caret, folder, github, mark } = icons;

  const folderName = $derived(app.route.view === "project" ? app.route.project : null);
  let project = $state<Project | null>(null);

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

<View style={{ direction: "row", align: "center", gap: 8, paddingLeft: 6, paddingRight: 4 }}>
  <Icon svg={mark} size={18} color={c.text} />
  <Text style={{ fontSize: 14, fontWeight: "semibold" }}>NeoWorks</Text>
  <View style={{ width: 1, height: 16, background: c.borderStrong, marginX: 4 }} />
</View>
<Button
  style={{ paddingX: 8, height: 28, radius: 8, color: folderName ? c.muted : c.text, transition: 120 }}
  hover={{ background: c.hover, color: c.text }}
  onpress={() => app.open({ view: "overview" })}
>
  <Text style={{ fontSize: 14 }}>Projects</Text>
</Button>
{#if folderName}
  <Icon svg={caret} size={12} color={c.faint} />
  <Text style={{ fontSize: 14, paddingX: 6 }}>{project?.name ?? folderName.split("/").pop()}</Text>
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
{/if}
