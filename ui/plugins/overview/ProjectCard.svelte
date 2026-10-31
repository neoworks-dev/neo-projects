<script lang="ts">
  // A project on the overview: its name and repository, how many issues are open and blocked,
  // what is in progress, and a bar of the Status split.
  import { Text, View } from "@neoworks/svelte-gpui/ui";
  import type { Project } from "../../lib/api.ts";
  import { c, projectColor, projectInitials, ring, tone } from "../../lib/theme.ts";

  let { project, zoom, selected }: { project: Project; zoom: number; selected: boolean } = $props();

  const summary = $derived(project.summary);
  const tint = $derived(projectColor(project.name));
  const total = $derived(summary ? summary.split.reduce((sum, part) => sum + part.count, 0) : 0);
</script>

<View
  style={{
    flex: 1,
    padding: 14,
    gap: 10,
    radius: 12,
    background: c.surface,
    borderWidth: 1,
    shadow: selected ? ring : null,
    borderColor: selected ? c.accent : c.border,
    overflow: "hidden",
    transition: 140,
  }}
  hover={{ borderColor: selected ? c.accent : c.borderStrong }}
>
  <View style={{ direction: "row", align: "center", gap: 10 }}>
    <View
      style={{
        size: 34,
        radius: 9,
        align: "center",
        justify: "center",
        background: tint.soft,
        borderWidth: 1,
        borderColor: tint.line,
        shrink: 0,
      }}
    >
      <Text style={{ fontSize: 12, fontWeight: "semibold", color: tint.text }}>{projectInitials(project.name)}</Text>
    </View>
    <View style={{ gap: 2, flex: 1, minWidth: 0 }}>
      <Text style={{ fontSize: zoom < 0.5 ? 30 : 17, color: c.text, truncate: true }}>
        {project.name}
      </Text>
      {#if zoom >= 0.5}
        <Text style={{ fontSize: 11, color: c.faint, truncate: true }}>{project.repo ?? "Local project"}</Text>
      {/if}
    </View>
  </View>
  {#if summary && zoom >= 0.5}
    <View style={{ direction: "row", gap: 14 }}>
      <View>
        <Text style={{ fontSize: 18, fontWeight: "semibold", color: c.text }}>{summary.open}</Text>
        <Text style={{ fontSize: 10, color: c.muted }}>open</Text>
      </View>
      <View>
        <Text style={{ fontSize: 18, fontWeight: "semibold", color: summary.blocked > 0 ? c.danger : c.text }}>
          {summary.blocked}
        </Text>
        <Text style={{ fontSize: 10, color: c.muted }}>blocked</Text>
      </View>
      <View>
        <Text style={{ fontSize: 18, fontWeight: "semibold", color: c.muted }}>{summary.closed}</Text>
        <Text style={{ fontSize: 10, color: c.muted }}>closed</Text>
      </View>
    </View>
    <View style={{ direction: "row", height: 6, radius: 3, overflow: "hidden", background: c.raised }}>
      {#each summary.split.filter((part) => part.count > 0) as part (part.name)}
        <View style={{ flex: part.count / Math.max(total, 1), background: tone(part.color) }} />
      {/each}
    </View>
    {#if summary.inProgress.length > 0}
      <View style={{ gap: 4 }}>
        <Text style={{ fontSize: 10, color: c.faint, uppercase: true }}>In progress</Text>
        {#each summary.inProgress as issue (issue.node)}
          <Text style={{ fontSize: 12, color: c.text, truncate: true }}>
            <Text style={{ color: c.faint }}>#{issue.number}</Text>
            {issue.title}
          </Text>
        {/each}
      </View>
    {:else if !summary.hasStatus}
      <Text style={{ fontSize: 11, color: c.faint }}>No Status field: Open/Closed board</Text>
    {/if}
  {:else if !summary && zoom >= 0.5}
    <Text style={{ fontSize: 12, color: c.faint }}>{project.repo ? "Waiting for the first sync…" : "Local project"}</Text>
  {/if}
</View>
