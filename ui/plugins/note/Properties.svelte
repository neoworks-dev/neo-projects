<script lang="ts">
  // A note's front matter as rows of properties, as Obsidian shows them: the name, and an editor by
  // the value's type. Each change is written at once through `note_prop`, which edits only that
  // key's lines, and answers with the note as it is now. The `id` is what links point at, so it
  // shows read-only.
  import { Button, Icon, Input, Text, View } from "@neoworks/svelte-gpui/ui";
  import { Checkbox, Tooltip } from "@neoworks/svelte-gpui-ui";
  import type { Note, NoteProp } from "../../lib/api.ts";
  import { icons } from "../../lib/icons.ts";
  import { newKey, numberFrom, propertyKind, type PropertyKind } from "../../lib/properties.ts";
  import { app } from "../../lib/state.svelte.ts";
  import { c } from "../../lib/theme.ts";
  const { add, close, mark } = icons;

  let { note, onchange }: { note: Note; onchange: (note: Note) => void } = $props();

  /** What is typed into each list's "Add…" field, by key, so a submit can empty it. */
  let drafts = $state<Record<string, string>>({});

  const kindIcons: Record<PropertyKind, string> = {
    text: icons.text,
    number: icons.number,
    checkbox: icons.check,
    date: icons.date,
    list: icons.labels,
    other: icons.field,
  };

  async function setValue(key: string, value: unknown) {
    const next = await app.run(() => app.api.setNoteProp(note.path, key, value));
    if (next) onchange(next);
  }

  async function rename(from: string, typed: string) {
    const to = typed.trim();
    if (!to || to === from) return;
    const next = await app.run(() => app.api.renameNoteProp(note.path, from, to));
    if (next) onchange(next);
  }

  function addProperty() {
    void setValue(newKey(note.props.map((prop) => prop.key)), "");
  }

  function commitText(prop: NoteProp, typed: string) {
    if (typed === (prop.value ?? "")) return;
    void setValue(prop.key, typed);
  }

  function commitNumber(prop: NoteProp, typed: string) {
    const parsed = numberFrom(typed);
    if (parsed === null) {
      app.say(`"${typed}" isn't a number.`);
      return;
    }
    if (parsed !== prop.value) void setValue(prop.key, parsed);
  }

  function addItem(prop: NoteProp, typed: string) {
    const item = typed.trim();
    if (!item) return;
    drafts[prop.key] = "";
    void setValue(prop.key, [...(prop.value as unknown[]), item]);
  }

  function removeItem(prop: NoteProp, index: number) {
    const items = (prop.value as unknown[]).filter((_, at) => at !== index);
    void setValue(prop.key, items);
  }

  const fieldStyle = {
    flex: 1,
    fontSize: 13,
    color: c.text,
    paddingX: 8,
    height: 28,
    radius: 7,
    background: "transparent",
    borderWidth: 1,
    borderColor: "transparent",
  } as const;
  const fieldHover = { background: c.hover };
  const fieldFocus = { background: c.bg, borderColor: c.accent };
</script>

<View style={{ gap: 2 }}>
  {#each note.props as prop (prop.key)}
    {@const kind = propertyKind(prop.value)}
    <View style={{ direction: "row", align: "start", gap: 4 }}>
      <View style={{ direction: "row", align: "center", gap: 2, width: 160, shrink: 0, height: 28 }}>
        <View style={{ width: 22, align: "center", shrink: 0 }}>
          <Icon svg={prop.key === "id" ? mark : kindIcons[kind]} size={14} color={c.faint} />
        </View>
        {#if prop.key === "id"}
          <Text style={{ fontSize: 13, color: c.muted, paddingX: 8 }}>id</Text>
        {:else}
          <Input
            value={prop.key}
            style={{ ...fieldStyle, color: c.muted }}
            hover={fieldHover}
            focus={fieldFocus}
            oncommit={(typed) => void rename(prop.key, typed)}
          />
        {/if}
      </View>

      <View style={{ flex: 1, minWidth: 0, direction: "row", align: "center", minHeight: 28 }}>
        {#if prop.key === "id"}
          <Text style={{ fontSize: 12, fontFamily: "mono", color: c.faint, paddingX: 8, truncate: true }}>{String(prop.value)}</Text>
        {:else if kind === "checkbox"}
          <!-- The checkbox's press target is 40px; it overflows the 28px row rather than growing it. -->
          <View style={{ height: 28, justify: "center", marginLeft: -2 }}>
            <Checkbox size="sm" checked={prop.value === true} onchange={(checked) => void setValue(prop.key, checked)} />
          </View>
        {:else if kind === "list"}
          {@const items = prop.value as unknown[]}
          <View style={{ flex: 1, direction: "row", wrap: true, gap: 4, align: "center", paddingX: 4, paddingY: 3 }}>
            {#each items as item, index (index)}
              <View style={{ direction: "row", align: "center", gap: 2, paddingLeft: 8, paddingRight: 2, height: 22, radius: 11, borderWidth: 1, borderColor: c.border }}>
                <Text style={{ fontSize: 12, color: c.text }}>{String(item)}</Text>
                <Button
                  style={{ size: 18, radius: 9, align: "center", justify: "center", color: c.faint }}
                  hover={{ background: c.hover, color: c.text }}
                  onpress={() => removeItem(prop, index)}
                >
                  <Icon svg={close} size={10} />
                </Button>
              </View>
            {/each}
            <Input
              value={drafts[prop.key] ?? ""}
              placeholder="Add…"
              style={{ ...fieldStyle, flex: undefined, width: 96, shrink: 0, height: 22, fontSize: 12 }}
              oninput={(typed) => (drafts[prop.key] = typed)}
              hover={fieldHover}
              focus={fieldFocus}
              onsubmit={(typed) => addItem(prop, typed)}
            />
          </View>
        {:else if kind === "other"}
          <Text style={{ fontSize: 12, fontFamily: "mono", color: c.faint, paddingX: 8, paddingY: 6 }}>{JSON.stringify(prop.value)}</Text>
        {:else}
          <Input
            value={prop.value === null ? "" : String(prop.value)}
            placeholder={kind === "date" ? "YYYY-MM-DD" : "Empty"}
            style={fieldStyle}
            hover={fieldHover}
            focus={fieldFocus}
            oncommit={(typed) => {
              if (kind === "number") commitNumber(prop, typed);
              else commitText(prop, typed);
            }}
          />
        {/if}
      </View>

      {#if prop.key !== "id"}
        <Tooltip text="Remove" placement="left">
          <Button
            style={{ size: 28, radius: 7, align: "center", justify: "center", color: c.faint, transition: 120 }}
            hover={{ background: c.hover, color: c.text }}
            onpress={() => void setValue(prop.key, null)}
          >
            <Icon svg={close} size={12} />
          </Button>
        </Tooltip>
      {:else}
        <View style={{ width: 28, shrink: 0 }} />
      {/if}
    </View>
  {/each}
  <Button
    style={{ direction: "row", align: "center", gap: 8, paddingX: 8, height: 28, radius: 7, color: c.faint, alignSelf: "start", transition: 120 }}
    hover={{ background: c.hover, color: c.text }}
    onpress={addProperty}
  >
    <Icon svg={add} size={14} />
    <Text style={{ fontSize: 13 }}>Add property</Text>
  </Button>
</View>
