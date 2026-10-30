<script lang="ts">
  // The tool rail: the overview, then one tile per project to jump straight to it, and at the
  // bottom a button for a new project. The open one is marked by a bar at the rail's edge.
  import { Button, Icon, Text, View } from "@neoworks/svelte-gpui/ui";
  import { Tooltip } from "@neoworks/svelte-gpui-ui";
  import plus from "@phosphor-icons/core/regular/plus.svg";
  import squares from "@phosphor-icons/core/regular/squares-four.svg";
  import type { Project } from "../../lib/api.ts";
  import { app } from "../../lib/state.svelte.ts";
  import { c, projectColor } from "../../lib/theme.ts";
  import { dialogs } from "./dialogs.svelte.ts";

  let projects = $state<Project[]>([]);

  $effect(() => {
    void app.version;
    if (!app.workspace) return;
    void app.api.overview().then((overview) => {
      projects = overview.nodes.flatMap((n) => (n.item?.kind === "project" ? [n.item.project] : []));
    });
  });

  const open = $derived(app.route.view === "project" ? app.route.project : null);

  function initials(name: string): string {
    const words = name.split(/[-_ .]+/).filter(Boolean);
    if (words.length > 1) return (words[0][0] + words[1][0]).toUpperCase();
    return name.slice(0, 2).toUpperCase();
  }
</script>

{#snippet marker(on: boolean)}
  <View
    style={{
      position: "absolute",
      left: -8,
      top: 9,
      width: 3,
      height: 18,
      radius: 2,
      background: c.text,
      opacity: on ? 1 : 0,
      transition: 160,
    }}
  />
{/snippet}

{#if app.workspace}
  <Tooltip text="All projects" placement="right">
    <View>
      {@render marker(app.route.view === "overview")}
      <Button
        style={{
          size: 36,
          radius: 10,
          align: "center",
          justify: "center",
          color: app.route.view === "overview" ? c.text : c.muted,
          background: app.route.view === "overview" ? c.hover : "transparent",
          transition: 140,
        }}
        hover={{ background: c.hover, color: c.text }}
        onpress={() => app.open({ view: "overview" })}
      >
        <Icon svg={squares} size={19} />
      </Button>
    </View>
  </Tooltip>
  <View style={{ width: 22, height: 1, background: c.border, marginY: 2 }} />
  {#each projects as project (project.folder)}
    {@const on = open === project.folder}
    {@const tint = projectColor(project.name)}
    <Tooltip text={project.repo ?? `${project.name} · local`} placement="right">
      <View>
        {@render marker(on)}
        <Button
          style={{
            size: 36,
            radius: on ? 12 : 10,
            align: "center",
            justify: "center",
            background: tint.soft,
            borderWidth: 1,
            borderColor: on ? tint.solid : tint.line,
            transition: 160,
          }}
          hover={{ borderColor: tint.solid }}
          onpress={() => app.open({ view: "project", project: project.folder })}
        >
          <Text style={{ fontSize: 12, fontWeight: "semibold", color: tint.text }}>{initials(project.name)}</Text>
        </Button>
      </View>
    </Tooltip>
  {/each}
  <Tooltip text="New project" placement="right">
    <Button
      style={{ size: 36, radius: 10, align: "center", justify: "center", color: c.faint, transition: 140 }}
      hover={{ background: c.hover, color: c.text }}
      onpress={() => (dialogs.addProjects = true)}
    >
      <Icon svg={plus} size={17} />
    </Button>
  </Tooltip>
{/if}
