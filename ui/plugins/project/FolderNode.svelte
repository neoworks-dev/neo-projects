<script lang="ts">
  // A folder on the canvas, drawn as one: a back with its tab, the first few things inside
  // peeking out over the top, and a front with its name and how much it holds. Lengths are in
  // world units and the canvas layer's zoom scales them; `zoom` picks the level of detail.
  import { Icon, Image, Text, View } from "@neoworks/svelte-gpui/ui";
  import type { Item, Peek } from "../../lib/api.ts";
  import { fileIcon } from "../../lib/fileIcons.ts";
  import { PEEK_BOTTOM, PEEK_INSET, PEEK_STEP } from "../../lib/canvas/unfold.ts";
  import { icons } from "../../lib/icons.ts";
  import { c, ring, stateTransition, stickyTint } from "../../lib/theme.ts";

  // `height` is the node's, in world units. `open` while its contents are unfolded beside it,
  // `target` while a dragged node would go in, and `opening`, 0 to 1, while nodes fall in: the
  // front lowers and what peeks out rises. `gone` says how far each peeking card has left, 0 to 1,
  // as its thing comes out of the folder onto the canvas.
  let {
    item,
    zoom,
    height,
    selected,
    open,
    target,
    opening = 0,
    gone = [],
  }: {
    item: Extract<Item, { kind: "folder" }>;
    zoom: number;
    height: number;
    selected: boolean;
    open: boolean;
    target: boolean;
    opening?: number;
    gone?: number[];
  } = $props();

  /** Below this height (world units) nothing has room to peek out, and the folder is its front. */
  const PEEK_HEIGHT = 120;
  /** Where the back starts, leaving room above it for what sticks out. */
  const BACK_TOP = 26;

  const peeking = $derived(height >= PEEK_HEIGHT);
  const line = $derived(selected || target ? c.accent : open ? c.borderStrong : c.border);
  const count = $derived(item.count === 1 ? "1 item" : `${item.count} items`);

  /** The icon of a thing without a picture. */
  function iconOf(peek: Peek): { src?: string; svg?: string } {
    if (peek.kind === "folder") return { src: fileIcon(peek.title ?? "", true) };
    if (peek.kind === "file") return { src: fileIcon(peek.title ?? "") };
    if (peek.kind === "note") return { svg: icons.note };
    if (peek.kind === "issue") return { svg: icons.stateTodo };
    if (peek.kind === "link") return { svg: icons.link };
    return { svg: icons.fileText };
  }
</script>

{#snippet card(peek: Peek, index: number)}
  {@const sticky = peek.kind === "text" ? stickyTint(undefined) : null}
  <!-- Each card sticks out above the back, a little lower than the one behind it, where
       `peekCard` in unfold.ts says, so its thing can come out of it. -->
  <View
    style={{
      position: "absolute",
      top: index * PEEK_STEP - 8 * opening,
      left: PEEK_INSET,
      right: PEEK_INSET,
      // Down behind the front, however tall the folder is.
      bottom: PEEK_BOTTOM,
      opacity: 1 - (gone[index] ?? 0),
      radius: 8,
      background: sticky ? sticky.background : c.surface,
      borderWidth: 1,
      borderColor: sticky ? sticky.border : c.border,
      overflow: "hidden",
      shadow: [{ x: 0, y: -2, blur: 8, spread: 0, color: "rgba(0, 0, 0, 0.35)" }],
    }}
  >
    {#if peek.image}
      <Image src={peek.image} fit="cover" style={{ flex: 1, radius: 8 - 1 }} />
    {:else}
      {@const icon = iconOf(peek)}
      <View style={{ direction: "row", gap: 6, padding: 8 }}>
        {#if !sticky}
          {#if icon.src}
            <Image src={icon.src} style={{ size: 13, shrink: 0 }} />
          {:else if icon.svg}
            <Icon svg={icon.svg} size={13} color={c.faint} />
          {/if}
        {/if}
        <Text style={{ flex: 1, fontSize: 10.5, lineHeight: 14, color: sticky ? sticky.text : c.muted, maxLines: 2, overflowWrap: "anywhere" }}>
          {peek.title ?? ""}
        </Text>
      </View>
    {/if}
  </View>
{/snippet}

<View style={{ flex: 1 }}>
  {#if peeking}
    <!-- The back, then its tab over the back's top edge, open at the bottom so the two are one. -->
    <View
      style={{
        position: "absolute",
        top: BACK_TOP,
        left: 0,
        right: 0,
        bottom: 0,
        radius: [0, 12, 12, 12],
        background: c.raised,
        borderWidth: 1,
        borderColor: line,
        shadow: selected ? ring : null,
        transition: stateTransition,
      }}
    />
    <View
      style={{
        position: "absolute",
        top: BACK_TOP - 10,
        left: 0,
        width: 84,
        height: 10 + 1,
        radius: [10, 10, 0, 0],
        background: c.raised,
        borderWidth: [1, 1, 0, 1],
        borderColor: line,
        transition: stateTransition,
      }}
    />
    {#each item.peek as peek, index (index)}
      {@render card(peek, index)}
    {/each}
  {/if}
  <!-- The front, over all but the top of what peeks out, or the whole folder when nothing does:
       how much it holds over its name, and its icon in the corner. -->
  <View
    style={{
      position: "absolute",
      left: 0,
      right: 0,
      bottom: 0,
      top: peeking ? undefined : 0,
      height: peeking ? `${64 - 14 * opening}%` : undefined,
      radius: 12,
      background: target ? c.hover : c.surface,
      borderWidth: 1,
      borderColor: line,
      shadow: selected && !peeking ? ring : null,
      direction: "row",
      align: "end",
      gap: 10,
      padding: 14,
      transition: stateTransition,
    }}
  >
    <View style={{ flex: 1, gap: 2 }}>
      {#if zoom >= 0.45}
        <Text style={{ fontSize: 11, color: c.faint }}>{count}</Text>
      {/if}
      <Text style={{ fontSize: 15, fontWeight: "semibold", lineHeight: 19, maxLines: 2, overflowWrap: "anywhere" }}>
        {item.title}
      </Text>
    </View>
    <View
      style={{
        size: 34,
        shrink: 0,
        radius: 9,
        align: "center",
        justify: "center",
        background: c.raised,
        borderWidth: 1,
        borderColor: c.border,
      }}
    >
      <Image src={fileIcon(item.title, true, open || opening > 0.3)} style={{ size: 18 }} />
    </View>
  </View>
</View>
