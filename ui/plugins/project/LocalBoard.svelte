<script lang="ts">
  // A local board: notes grouped by a front matter field (`status`). Dragging a note to another
  // column writes the field into the note; "+" makes a new note in that column.
  import { Icon, Button, Draggable, DropTarget, Input, Scroll, Text, View } from "@neoworks/svelte-gpui/ui";
  import type { Item } from "../../lib/api.ts";
  import { icons } from "../../lib/icons.ts";
  import { app } from "../../lib/state.svelte.ts";
  import { c, ring } from "../../lib/theme.ts";

  type LocalBoardItem = Extract<Item, { kind: "localboard" }>;
  let { item, zoom, selected }: { item: LocalBoardItem; zoom: number; selected: boolean } = $props();

  let adding = $state<string | null>(null);
  let title = $state("");
  const folder = $derived(item.path.split("/").slice(0, -1).join("/"));

  interface NoteDrag {
    kind: "note";
    board: string;
    path: string;
  }

  function isNote(data: unknown): data is NoteDrag {
    return typeof data === "object" && data !== null && (data as { kind?: string }).kind === "note";
  }

  async function create(value: string | null) {
    const text = title.trim();
    adding = null;
    title = "";
    if (!text) return;
    await app.run(async () => {
      const note = await app.api.createNote(folder, text, item.groupBy, value ?? undefined);
      app.select({ kind: "note", path: note.path });
      app.version++;
    });
  }
</script>

<View
  style={{
    flex: 1,
    radius: 12,
    background: c.surface,
    borderWidth: 1,
    shadow: selected ? ring : null,
    borderColor: selected ? c.accent : c.border,
    overflow: "hidden",
  }}
>
  <View style={{ direction: "row", align: "center", gap: 8, height: 38, paddingX: 14, cursor: "grab" }}>
    <Text style={{ fontSize: 13, fontWeight: "semibold" }}>{item.title}</Text>
    <Text style={{ fontSize: 11, color: c.faint }}>notes by {item.groupBy}</Text>
  </View>
  <View style={{ flex: 1, direction: "row", gap: 8, padding: [0, 8, 8, 8], minHeight: 0 }}>
    {#each item.columns as column (column.value ?? "other")}
      <DropTarget
        accept={(data) => isNote(data) && data.board === item.path && !column.notes.some((n) => n.path === data.path)}
        ondrop={({ data }) => {
          if (isNote(data)) void app.run(() => app.api.boardMove(item.path, data.path, column.value));
        }}
        style={{ flex: 1, minWidth: 0, radius: 8, background: c.bg, padding: 6, gap: 6 }}
        over={{ background: c.accentSoft }}
      >
        <View style={{ direction: "row", align: "center", gap: 6, paddingX: 4, height: 24 }}>
          <Text style={{ fontSize: 12, fontWeight: "semibold", nowrap: true }}>{column.label}</Text>
          <Text style={{ fontSize: 11, color: c.faint }}>{column.notes.length}</Text>
          <View style={{ flex: 1 }} />
          <Button
            style={{ size: 24, radius: 6, align: "center", justify: "center", color: c.muted, transition: 120 }}
            hover={{ background: c.hover, color: c.text }}
            onpress={() => (adding = column.value ?? "")}
          >
            <Icon svg={icons.add} size={14} />
          </Button>
        </View>
        {#if adding === (column.value ?? "")}
          <Input
            bind:value={title}
            placeholder="Note title, Enter to create"
            style={{ paddingX: 8, paddingY: 6, radius: 6, background: c.raised, borderWidth: 1, borderColor: c.accent, fontSize: 13, color: c.text }}
            onsubmit={() => void create(column.value)}
            oncommit={() => {
              if (!title.trim()) adding = null;
            }}
          />
        {/if}
        <Scroll style={{ flex: 1 }}>
          <View style={{ gap: 6 }}>
            {#each column.notes as note (note.path)}
              <Draggable data={{ kind: "note", board: item.path, path: note.path }}>
                <View
                  style={{ padding: 8, gap: 4, radius: 6, background: c.raised, borderWidth: 1, borderColor: c.border }}
                  hover={{ borderColor: c.borderStrong }}
                  onpress={() => app.select({ kind: "note", path: note.path })}
                >
                  <Text style={{ fontSize: 13, maxLines: 2 }}>{note.title}</Text>
                  {#if note.excerpt && zoom >= 0.6}
                    <Text style={{ fontSize: 11, color: c.muted, maxLines: 2 }}>{note.excerpt}</Text>
                  {/if}
                </View>
              </Draggable>
            {/each}
          </View>
        </Scroll>
      </DropTarget>
    {/each}
  </View>
</View>
