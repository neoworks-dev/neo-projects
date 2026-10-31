<script lang="ts" module>
  /**
   * The sticky colours, by JSON Canvas's preset numbers ("1" red to "6" purple), from the
   * design system's landscape hues so they sit quietly on the dark canvas.
   */
  export const stickyColors = ["3", "4", "5", "1"] as const;
</script>

<script lang="ts">
  // A sticky note: a text node of the canvas, only a body, rendered as Markdown and written in
  // place. A double press (or typing right after it is made) turns it into a field; Escape or
  // pressing elsewhere turns it back. The text lives in the canvas file, as JSON Canvas keeps it.
  import { Input, Text, View } from "@neoworks/svelte-gpui/ui";
  import Markdown from "../../lib/markdown/Markdown.svelte";
  import { c, ring, stickyTint } from "../../lib/theme.ts";

  interface Props {
    text: string;
    color: string | undefined;
    zoom: number;
    selected: boolean;
    editing: boolean;
    /** The text after an edit, once the field lets go of it. */
    onchange: (text: string) => void;
    ondone: () => void;
  }

  let { text, color, zoom, selected, editing, onchange, ondone }: Props = $props();

  const tint = $derived(stickyTint(color));
  let draft = $state("");
  let field = $state<Input>();

  $effect(() => {
    if (!editing) return;
    draft = text;
    field?.focus();
  });

  function finish() {
    if (draft !== text) onchange(draft);
    ondone();
  }
</script>

<View
  style={{
    flex: 1,
    padding: 14,
    radius: 6,
    background: tint.background,
    borderWidth: 1,
    borderColor: selected ? c.accent : tint.border,
    shadow: selected ? ring : [{ x: 0, y: 6, blur: 16, spread: -6, color: "rgba(0, 0, 0, 0.5)" }],
    overflow: "hidden",
    transition: 140,
  }}
>
  {#if editing}
    <Input
      bind:this={field}
      bind:value={draft}
      multiline
      rows={3}
      placeholder="Write… Markdown works"
      style={{ flex: 1, fontFamily: "mono", fontSize: 13, lineHeight: 20, color: tint.text, background: "transparent", padding: 0 }}
      oncommit={finish}
      onblur={finish}
      onkey={(e) => {
        if (e.key !== "Escape") return;
        e.preventDefault();
        e.stopPropagation();
        field?.blur();
      }}
    />
  {:else if text.trim()}
    <Markdown source={text} size={14} interactive={false} />
  {:else}
    <Text style={{ fontSize: 13, color: tint.text, opacity: 0.55 }}>Double-click to write</Text>
  {/if}
</View>
