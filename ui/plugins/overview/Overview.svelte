<script lang="ts">
  // All projects on one canvas, with the cross-repository blocks between them. Arrange the cards
  // by dragging; double-press one to open its project.
  import { Button, Text, View } from "@neoworks/svelte-gpui/ui";
  import type { CanvasNode, Overview, Project } from "../../lib/api.ts";
  import Viewport, { type ViewState } from "../../lib/canvas/Viewport.svelte";
  import { app } from "../../lib/state.svelte.ts";
  import { button, buttonHover, c } from "../../lib/theme.ts";
  import { dialogs } from "./dialogs.svelte.ts";
  import ProjectCard from "./ProjectCard.svelte";

  let overview = $state<Overview | null>(null);
  let view = $state<ViewState>({ x: -40, y: -40, zoom: 1 });
  let selected = $state<string | null>(null);

  $effect(() => {
    void app.version;
    void app.workspace;
    void app.run(async () => {
      overview = await app.api.overview();
    });
  });

  interface Placed {
    id: string;
    x: number;
    y: number;
    w: number;
    h: number;
    project: Project;
  }

  const nodes = $derived<Placed[]>(
    (overview?.nodes ?? []).flatMap((n) =>
      n.item?.kind === "project" ? [{ id: n.id, x: n.x, y: n.y, w: n.width, h: n.height, project: n.item.project }] : [],
    ),
  );

  const arrows = $derived(
    (overview?.arrows ?? []).map((a) => ({
      ...a,
      label: a.count ? `${a.count} blocking` : a.label,
      color: c.danger,
    })),
  );

  function moved(id: string, x: number, y: number) {
    if (!overview) return;
    const next: CanvasNode[] = overview.nodes.map((n) => (n.id === id ? { ...n, x, y } : n));
    overview = { ...overview, nodes: next };
    void app.run(() => app.api.saveCanvas(overview!.path, next));
  }

  function open(id: string | null) {
    const placed = nodes.find((n) => n.id === id);
    if (placed) app.open({ view: "project", project: placed.project.folder });
  }
</script>

<View style={{ flex: 1 }}>
  {#if overview && nodes.length === 0}
    <View style={{ flex: 1, align: "center", justify: "center", gap: 14 }}>
      <Text style={{ fontSize: 20, fontWeight: "semibold" }}>No projects yet</Text>
      <Text style={{ fontSize: 13, color: c.muted, maxWidth: 420, textAlign: "center" }}>
        A project is one GitHub repository, with its board, notes and files. Pick the repositories you work on.
      </Text>
      <Button style={button} hover={buttonHover} onpress={() => (dialogs.addProjects = true)}>
        <Text>Add projects from GitHub</Text>
      </Button>
    </View>
  {:else}
    <Viewport
      {nodes}
      {arrows}
      bind:view
      {selected}
      onselect={(id) => (selected = id)}
      onopen={open}
      onpress={open}
      onmoved={moved}
    >
      {#snippet node(n, zoom)}
        <ProjectCard project={n.project} {zoom} selected={selected === n.id} />
      {/snippet}
    </Viewport>
  {/if}
  {#if overview && overview.errors.length > 0}
    <View
      style={{
        position: "absolute",
        left: 12,
        bottom: 12,
        padding: 10,
        gap: 4,
        radius: 8,
        background: c.surface,
        borderWidth: 1,
        borderColor: c.danger,
        maxWidth: 520,
      }}
    >
      {#each overview.errors as error (error)}
        <Text style={{ fontSize: 12, color: c.text }}>{error}</Text>
      {/each}
    </View>
  {/if}
</View>
