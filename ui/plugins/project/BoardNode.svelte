<script lang="ts">
  // The project's GitHub board as a region of its canvas: one column per Status option (or Open
  // and Closed). Drag a card to another column to set its status, or out onto the canvas to give
  // it a place of its own. "+" in a column header makes a new issue there.
  import {
    Button,
    Draggable,
    DropTarget,
    Input,
    Scroll,
    Text,
    View,
  } from "@neoworks/svelte-gpui/ui";
  import type { Board, Card, Project } from "../../lib/api.ts";
  import IssueCard from "../../lib/IssueCard.svelte";
  import { app } from "../../lib/state.svelte.ts";
  import { c, tone } from "../../lib/theme.ts";

  interface Props {
    board: Board;
    project: Project;
    zoom: number;
    /** Issues placed on the canvas, per column key, which the column only counts. */
    placed: Record<string, number>;
  }

  let { board, project, zoom, placed }: Props = $props();

  const s = (n: number) => n * zoom;
  let adding = $state<string | null>(null);
  let title = $state("");

  function isCard(data: unknown): data is { kind: "card"; card: Card } {
    return typeof data === "object" && data !== null && (data as { kind?: string }).kind === "card";
  }

  async function create(column: string) {
    const text = title.trim();
    adding = null;
    title = "";
    if (!text) return;
    if (project.local) {
      await app.run(() => app.api.createLocalIssue(project.folder, text, column));
      return;
    }
    if (!project.repoId) return;
    await app.run(() => app.api.createIssue(project.repoId!, text, column === "open" ? null : column));
  }

  const selectedIssue = $derived(app.selection?.kind === "issue" ? app.selection.node : null);
</script>

<View
  style={{
    flex: 1,
    radius: s(12),
    background: c.surface,
    borderWidth: 1,
    borderColor: c.border,
    overflow: "hidden",
  }}
>
  <View style={{ direction: "row", align: "center", gap: s(8), height: s(40), paddingX: s(14), cursor: "grab" }}>
    <Text style={{ fontSize: s(13), fontWeight: "semibold" }}>{project.local ? "Issues" : "Board"}</Text>
    <Text style={{ fontSize: s(12), color: c.faint }}>{project.repo ?? `${project.folder}/Issues`}</Text>
    {#if !board.statusField && board.columns.length > 0}
      <Text style={{ fontSize: s(11), color: c.faint }}>· no Status field, so Open and Closed</Text>
    {/if}
  </View>
  {#if board.columns.length === 0}
    <View style={{ flex: 1, align: "center", justify: "center" }}>
      <Text style={{ fontSize: s(13), color: c.muted }}>Waiting for the first sync…</Text>
    </View>
  {/if}
  <View style={{ flex: 1, direction: "row", gap: s(10), padding: [0, s(10), s(10), s(10)], minHeight: 0 }}>
    {#each board.columns as column (column.key)}
      <DropTarget
        accept={(data) => isCard(data) && !column.cards.some((card) => card.node === data.card.node)}
        ondrop={({ data }) => {
          if (isCard(data)) void app.run(() => app.api.moveCard(data.card.node, column.key));
        }}
        style={{ flex: 1, minWidth: 0, radius: s(10), background: c.bg, padding: s(6), gap: s(6) }}
        over={{ background: c.accentSoft }}
      >
        <View style={{ direction: "row", align: "center", gap: s(6), paddingX: s(4), height: s(26) }}>
          <View style={{ width: s(8), height: s(8), radius: s(4), background: tone(column.color) }} />
          <Text style={{ fontSize: s(12), fontWeight: "semibold", nowrap: true }}>{column.name}</Text>
          <Text style={{ fontSize: s(11), color: c.faint }}>{column.cards.length}</Text>
          {#if placed[column.key]}
            <Text style={{ fontSize: s(10), color: c.faint }}>+{placed[column.key]} on canvas</Text>
          {/if}
          <View style={{ flex: 1 }} />
          {#if project.repoId || project.local}
            <Button
              style={{ paddingX: s(5), radius: s(4), color: c.muted, fontSize: s(14) }}
              hover={{ background: c.hover, color: c.text }}
              onpress={() => {
                adding = column.key;
                title = "";
              }}
            >
              <Text>+</Text>
            </Button>
          {/if}
        </View>
        {#if adding === column.key}
          <Input
            bind:value={title}
            placeholder="Issue title, Enter to create"
            style={{
              paddingX: s(8),
              paddingY: s(6),
              radius: s(6),
              background: c.raised,
              borderWidth: 1,
              borderColor: c.accent,
              fontSize: s(13),
              color: c.text,
            }}
            onsubmit={() => void create(column.key)}
            oncommit={() => {
              if (!title.trim()) adding = null;
            }}
          />
        {/if}
        <Scroll style={{ flex: 1 }}>
          <View style={{ gap: s(6) }}>
            {#each column.cards as card (card.node)}
              <Draggable data={{ kind: "card", card }}>
                <View onpress={() => app.select({ kind: "issue", node: card.node })}>
                  <IssueCard {card} {zoom} selected={selectedIssue === card.node} />
                </View>
              </Draggable>
            {/each}
          </View>
        </Scroll>
      </DropTarget>
    {/each}
  </View>
</View>
