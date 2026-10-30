<script lang="ts">
  // The status bar's start: the workspace folder and, inside a project, its folder, as a path.
  import { Icon, Text } from "@neoworks/svelte-gpui/ui";
  import caret from "@phosphor-icons/core/regular/caret-right.svg";
  import folder from "@phosphor-icons/core/regular/folder.svg";
  import { app } from "../../lib/state.svelte.ts";
  import { c } from "../../lib/theme.ts";

  const parts = $derived.by(() => {
    const workspace = app.workspace?.split("/").filter(Boolean).pop();
    if (!workspace) return [];
    if (app.route.view === "project") return [workspace, ...app.route.project.split("/").filter(Boolean)];
    return [workspace];
  });
</script>

{#if parts.length > 0}
  <Icon svg={folder} size={12} color={c.faint} />
  {#each parts as part, i (i)}
    {#if i > 0}<Icon svg={caret} size={10} color={c.faint} />{/if}
    <Text style={{ fontSize: 12, color: i === parts.length - 1 ? c.muted : c.faint }}>{part}</Text>
  {/each}
{/if}
