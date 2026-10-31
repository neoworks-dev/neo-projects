<script lang="ts">
  // The tool rail: the overview, then one tile per project to jump straight to it, and at the
  // bottom a button for a new project. The open one is marked by a bar at the rail's edge.
  import { Button, Icon, Text, View } from "@neoworks/svelte-gpui/ui";
  import { Tooltip } from "@neoworks/svelte-gpui-ui";
  import type { Project } from "../../lib/api.ts";
  import { app } from "../../lib/state.svelte.ts";
  import { c, projectColor, projectInitials } from "../../lib/theme.ts";
  import { dialogs } from "./dialogs.svelte.ts";
  import { icons } from "../../lib/icons.ts";
  const { add: plus, overview: squares } = icons;

  let projects = $state<Project[]>([]);

  $effect(() => {
    void app.version;
    if (!app.workspace) return;
    void app.api.overview().then((overview) => {
      projects = overview.nodes.flatMap((n) => (n.item?.kind === "project" ? [n.item.project] : []));
    });
  });

  const open = $derived(app.route.view === "project" ? app.route.project : null);

  // A tooltip's wrapper aligns itself to the start; on the rail every button sits in the middle.
  const centred = { alignSelf: "center" } as const;
</script>

{#snippet marker(on: boolean)}
  <View
    style={{
      position: "absolute",
      // At the rail's edge: the rail is 46 wide and a button 36, centred.
      left: -5,
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
  <Tooltip text="All projects" placement="right" anchorStyle={centred}>
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
    <Tooltip text={project.repo ?? `${project.name} · local`} placement="right" anchorStyle={centred}>
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
          <Text style={{ fontSize: 12, fontWeight: "semibold", color: tint.text }}>{projectInitials(project.name)}</Text>
        </Button>
      </View>
    </Tooltip>
  {/each}
  <Tooltip text="New project" placement="right" anchorStyle={centred}>
    <Button
      style={{ size: 36, radius: 10, align: "center", justify: "center", color: c.faint, transition: 140 }}
      hover={{ background: c.hover, color: c.text }}
      onpress={() => (dialogs.addProjects = true)}
    >
      <Icon svg={plus} size={17} />
    </Button>
  </Tooltip>
{/if}
