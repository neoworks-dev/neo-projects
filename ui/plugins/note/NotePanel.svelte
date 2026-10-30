<script lang="ts">
  // A note, at the right: its title (the file's name) and its Markdown body, saved as you type.
  // The file is plain Markdown; "Open" hands it to your editor.
  import { Button, Input, Scroll, Text, View } from "@neoworks/svelte-gpui/ui";
  import type { Note } from "../../lib/api.ts";
  import { app } from "../../lib/state.svelte.ts";
  import { c, field, fieldFocus, ghost, ghostHover } from "../../lib/theme.ts";

  let { path }: { path: string } = $props();

  let note = $state<Note | null>(null);
  let title = $state("");
  let body = $state("");
  let dirty = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    void app.version;
    const p = path;
    void app.run(async () => {
      const next = await app.api.note(p);
      if (p !== path || dirty) return;
      note = next;
      title = next.title;
      body = next.body;
    });
  });

  function typed() {
    dirty = true;
    clearTimeout(timer);
    timer = setTimeout(() => void save(), 600);
  }

  async function save() {
    clearTimeout(timer);
    if (!note || !dirty) return;
    const text = body;
    await app.run(() => app.api.writeNote(note!.path, text));
    dirty = false;
  }

  async function rename() {
    const next = title.trim();
    if (!note || !next || next === note.title) return;
    const renamed = await app.run(() => app.api.renameNote(note!.path, next));
    if (renamed) {
      note = renamed;
      app.select({ kind: "note", path: renamed.path });
    }
  }

  $effect(() => () => void save());
</script>

<View style={{ width: 460, borderLeftWidth: 1, borderColor: c.border, background: c.surface }}>
  <View style={{ direction: "row", align: "center", gap: 8, paddingX: 14, height: 44, shrink: 0 }}>
    <Text style={{ fontSize: 12, color: c.muted, flex: 1, truncate: true }}>{note?.path ?? path}</Text>
    <Button style={ghost} hover={ghostHover} onpress={() => note && void app.api.open(note.path)}>
      <Text>Open ↗</Text>
    </Button>
    <Button style={ghost} hover={ghostHover} onpress={() => app.select(null)}>
      <Text>✕</Text>
    </Button>
  </View>
  <Scroll style={{ flex: 1 }}>
    <View style={{ padding: [4, 14, 20, 14], gap: 10 }}>
      <Input
        bind:value={title}
        style={{ ...field, fontSize: 18, fontWeight: "semibold", background: "transparent", borderColor: "transparent", paddingX: 2 }}
        focus={{ borderColor: c.accent, background: c.bg }}
        onsubmit={() => void rename()}
        oncommit={() => void rename()}
      />
      <Input
        bind:value={body}
        multiline
        rows={24}
        placeholder="Write in Markdown…"
        style={{ ...field, lineHeight: 20, fontSize: 13 }}
        focus={fieldFocus}
        oninput={typed}
        oncommit={() => void save()}
      />
    </View>
  </Scroll>
</View>
