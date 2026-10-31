<script lang="ts">
  // All projects on one canvas, with the cross-repository blocks between them. Arrange the cards
  // by dragging; press one to open its project.
  import { Button, Text, View } from "@neoworks/svelte-gpui/ui";
  import type { CanvasNode, Overview, Project } from "../../lib/api.ts";
  import Viewport, { type Move, type ViewState } from "../../lib/canvas/Viewport.svelte";
  import { app } from "../../lib/state.svelte.ts";
  import { c, primary, primaryHover } from "../../lib/theme.ts";
  import { dialogs } from "./dialogs.svelte.ts";
  import ProjectCard from "./ProjectCard.svelte";

  let overview = $state<Overview | null>(null);
  let view = $state<ViewState>({ x: -40, y: -40, zoom: 1 });
  let selected = $state<string[]>([]);

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

  function moved(moves: Move[]) {
    if (!overview) return;
    const at = new Map(moves.map((m) => [m.id, m]));
    const next: CanvasNode[] = overview.nodes.map((n) => {
      const move = at.get(n.id);
      if (!move) return n;
      return { ...n, x: move.x, y: move.y };
    });
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
    <View style={{ flex: 1, align: "center", justify: "center", gap: 16 }}>
      <Text style={{ fontSize: 40, lineHeight: 44, color: c.text }}>Start a project</Text>
      <Text style={{ fontSize: 15, lineHeight: 23, color: c.muted, maxWidth: 440, textAlign: "center" }}>
        A board, notes and a canvas for one thing you work on. Keep its issues as files on your computer, or bring
        them from a GitHub repository.
      </Text>
      <Button style={{ ...primary, marginTop: 8, height: 38, paddingX: 18, radius: 19 }} hover={primaryHover} onpress={() => (dialogs.addProjects = true)}>
        <Text>New project</Text>
      </Button>
    </View>
  {:else}
    <Viewport
      {nodes}
      {arrows}
      bind:view
      {selected}
      onselect={(ids) => (selected = ids)}
      onopen={open}
      onpress={open}
      onmoved={moved}
    >
      {#snippet node(n, zoom)}
        <ProjectCard project={n.project} {zoom} selected={selected.includes(n.id)} />
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
