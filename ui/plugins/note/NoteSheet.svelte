<script lang="ts">
  // A note, as a sheet that rises over the canvas: the canvas stays in sight above it, dimmed, and
  // "Full screen" lifts the sheet to cover it. A wide, quiet page for writing: the title (the
  // file's name) and the Markdown body, saved as you type. The file is plain Markdown; "Open"
  // hands it to your editor.
  import { Button, Icon, Input, Presence, Scroll, Text, View } from "@neoworks/svelte-gpui/ui";
  import { Tooltip } from "@neoworks/svelte-gpui-ui";
  import arrowsIn from "@phosphor-icons/core/regular/arrows-in-simple.svg";
  import arrowsOut from "@phosphor-icons/core/regular/arrows-out-simple.svg";
  import external from "@phosphor-icons/core/regular/arrow-square-out.svg";
  import close from "@phosphor-icons/core/regular/x.svg";
  import { cubicOut } from "svelte/easing";
  import { fade, fly } from "svelte/transition";
  import type { Note } from "../../lib/api.ts";
  import { app } from "../../lib/state.svelte.ts";
  import { c, radii, shadow } from "../../lib/theme.ts";

  let { path }: { path: string } = $props();

  let note = $state<Note | null>(null);
  let title = $state("");
  let body = $state("");
  let saved = $state<"saved" | "saving" | "editing">("saved");
  let full = $state(false);
  let dirty = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let editor = $state<Input>();

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

  // A fresh note starts with the cursor in its body.
  $effect(() => {
    if (editor && note && !body) editor.focus();
  });

  function typed() {
    dirty = true;
    saved = "editing";
    clearTimeout(timer);
    timer = setTimeout(() => void save(), 600);
  }

  async function save() {
    clearTimeout(timer);
    if (!note || !dirty) return;
    const text = body;
    saved = "saving";
    await app.run(() => app.api.writeNote(note!.path, text));
    dirty = false;
    saved = "saved";
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

  const words = $derived(body.trim() ? body.trim().split(/\s+/).length : 0);
  const status = $derived(saved === "saved" ? "Saved" : saved === "saving" ? "Saving…" : "Editing");

  const iconButton = { size: 32, radius: 8, align: "center", justify: "center", color: c.muted, transition: 120 } as const;
  const iconHover = { background: c.hover, color: c.text };
</script>

<Presence
  in={[fade, { duration: 200 }]}
  out={[fade, { duration: 180 }]}
  style={{ position: "absolute", inset: 0, background: "#00000080" }}
>
  <View style={{ position: "absolute", inset: 0 }} onpress={() => app.select(null)} />
</Presence>
<Presence
  in={[fly, { y: 520, duration: 340, easing: cubicOut }]}
  out={[fly, { y: 420, duration: 220, opacity: 0 }]}
  style={{
    position: "absolute",
    left: full ? 0 : 28,
    right: full ? 0 : 28,
    top: full ? 0 : 56,
    bottom: 0,
  }}
>
  <View
    style={{
      flex: 1,
      radius: full ? 0 : [radii.panel + 4, radii.panel + 4, 0, 0],
      background: c.surface,
      borderWidth: full ? 0 : 1,
      borderBottomWidth: 0,
      borderColor: c.border,
      shadow,
      overflow: "hidden",
    }}
    onpress={() => {}}
  >
    <View style={{ align: "center", paddingTop: 8 }}>
      <View style={{ width: 36, height: 4, radius: 2, background: c.borderStrong }} />
    </View>
    <View style={{ direction: "row", align: "center", gap: 4, paddingX: 16, height: 44, shrink: 0 }}>
      <Text style={{ fontSize: 12, color: c.faint, flex: 1, truncate: true }}>{note?.path ?? path}</Text>
      <View style={{ direction: "row", align: "center", gap: 6, paddingX: 8 }}>
        <View
          style={{
            width: 6,
            height: 6,
            radius: 3,
            background: saved === "saved" ? c.ok : c.warn,
            transition: 200,
          }}
        />
        <Text style={{ fontSize: 12, color: c.faint }}>{status}</Text>
      </View>
      <Tooltip text="Open in your editor" placement="bottom">
        <Button style={iconButton} hover={iconHover} onpress={() => note && void app.api.open(note.path)}>
          <Icon svg={external} size={16} />
        </Button>
      </Tooltip>
      <Tooltip text={full ? "Back to the canvas" : "Full screen"} placement="bottom">
        <Button style={iconButton} hover={iconHover} onpress={() => (full = !full)}>
          <Icon svg={full ? arrowsIn : arrowsOut} size={16} />
        </Button>
      </Tooltip>
      <Tooltip text="Close  Esc" placement="bottom">
        <Button style={iconButton} hover={iconHover} onpress={() => app.select(null)}>
          <Icon svg={close} size={16} />
        </Button>
      </Tooltip>
    </View>
    <Scroll style={{ flex: 1 }}>
      <View style={{ align: "center", paddingX: 32 }}>
        <View style={{ width: "full", maxWidth: 720, paddingTop: 28, paddingBottom: 120, gap: 18 }}>
          <Input
            bind:value={title}
            placeholder="Untitled"
            style={{
              fontSize: 34,
              lineHeight: 40,
              color: c.text,
              background: "transparent",
              padding: 0,
            }}
            onsubmit={() => {
              void rename();
              editor?.focus();
            }}
            oncommit={() => void rename()}
          />
          <Text style={{ fontSize: 12, color: c.faint }}>{words} {words === 1 ? "word" : "words"} · Markdown</Text>
          <View style={{ height: 1, background: c.border }} />
          <Input
            bind:this={editor}
            bind:value={body}
            multiline
            rows={18}
            placeholder="Start writing…"
            style={{ fontSize: 16, lineHeight: 27, color: c.text, background: "transparent", padding: 0 }}
            oninput={typed}
            oncommit={() => void save()}
          />
        </View>
      </View>
    </Scroll>
  </View>
</Presence>
