<script lang="ts">
  // A project on the overview: its name and repository, how many issues are open and blocked,
  // what is in progress, and a bar of the Status split.
  import { Text, View } from "@neoworks/svelte-gpui/ui";
  import type { Project } from "../../lib/api.ts";
  import { c, tone } from "../../lib/theme.ts";

  let { project, zoom, selected }: { project: Project; zoom: number; selected: boolean } = $props();

  const s = (n: number) => n * zoom;
  const summary = $derived(project.summary);
  const total = $derived(summary ? summary.split.reduce((sum, part) => sum + part.count, 0) : 0);
</script>

<View
  style={{
    flex: 1,
    padding: s(14),
    gap: s(10),
    radius: s(12),
    background: c.surface,
    borderWidth: selected ? 2 : 1,
    borderColor: selected ? c.accent : c.border,
    overflow: "hidden",
  }}
  hover={{ borderColor: selected ? c.accent : c.borderStrong }}
>
  <View style={{ gap: s(2) }}>
    <Text style={{ fontSize: s(zoom < 0.5 ? 30 : 16), fontWeight: "semibold", color: c.text, truncate: true }}>
      {project.name}
    </Text>
    {#if zoom >= 0.5}
      <Text style={{ fontSize: s(11), color: c.faint, truncate: true }}>{project.repo ?? "no repository"}</Text>
    {/if}
  </View>
  {#if summary && zoom >= 0.5}
    <View style={{ direction: "row", gap: s(14) }}>
      <View>
        <Text style={{ fontSize: s(18), fontWeight: "semibold", color: c.text }}>{summary.open}</Text>
        <Text style={{ fontSize: s(10), color: c.muted }}>open</Text>
      </View>
      <View>
        <Text style={{ fontSize: s(18), fontWeight: "semibold", color: summary.blocked > 0 ? c.danger : c.text }}>
          {summary.blocked}
        </Text>
        <Text style={{ fontSize: s(10), color: c.muted }}>blocked</Text>
      </View>
      <View>
        <Text style={{ fontSize: s(18), fontWeight: "semibold", color: c.muted }}>{summary.closed}</Text>
        <Text style={{ fontSize: s(10), color: c.muted }}>closed</Text>
      </View>
    </View>
    <View style={{ direction: "row", height: s(6), radius: s(3), overflow: "hidden", background: c.raised }}>
      {#each summary.split.filter((part) => part.count > 0) as part (part.name)}
        <View style={{ flex: part.count / Math.max(total, 1), background: tone(part.color) }} />
      {/each}
    </View>
    {#if summary.inProgress.length > 0}
      <View style={{ gap: s(4) }}>
        <Text style={{ fontSize: s(10), color: c.faint, uppercase: true }}>In progress</Text>
        {#each summary.inProgress as issue (issue.node)}
          <Text style={{ fontSize: s(12), color: c.text, truncate: true }}>
            <Text style={{ color: c.faint }}>#{issue.number}</Text>
            {issue.title}
          </Text>
        {/each}
      </View>
    {:else if !summary.hasStatus}
      <Text style={{ fontSize: s(11), color: c.faint }}>No Status field: Open/Closed board</Text>
    {/if}
  {:else if !summary && zoom >= 0.5}
    <Text style={{ fontSize: s(12), color: c.faint }}>{project.repo ? "Waiting for the first sync…" : "Local project"}</Text>
  {/if}
</View>
