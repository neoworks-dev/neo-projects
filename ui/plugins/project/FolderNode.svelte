<script lang="ts">
  // A folder on the canvas, drawn as one: a back with its tab, the first few things inside
  // peeking out over the top, and a front with its name and how much it holds. Lengths are in
  // world units and the canvas layer's zoom scales them; `zoom` picks the level of detail.
  import { Icon, Image, Text, View } from "@neoworks/svelte-gpui/ui";
  import type { Item, Peek } from "../../lib/api.ts";
  import { fileIcon } from "../../lib/fileIcons.ts";
  import { icons } from "../../lib/icons.ts";
  import { c, ring, stateTransition, stickyTint } from "../../lib/theme.ts";

  // `height` is the node's, in world units. `open` while its contents are unfolded beside it,
  // `target` while a dragged node would go in, and `opening`, 0 to 1, while nodes fall in: the
  // front lowers and what peeks out rises.
  let {
    item,
    zoom,
    height,
    selected,
    open,
    target,
    opening = 0,
  }: {
    item: Extract<Item, { kind: "folder" }>;
    zoom: number;
    height: number;
    selected: boolean;
    open: boolean;
    target: boolean;
    opening?: number;
  } = $props();

  /** Below this height (world units) nothing has room to peek out, and the folder is its front. */
  const PEEK_HEIGHT = 120;

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
  <!-- Each card sits a little lower and further right than the one behind it. -->
  <View
    style={{
      position: "absolute",
      top: 16 + index * 11 - 8 * opening,
      left: 14 + index * 12,
      right: 14 + (item.peek.length - 1 - index) * 12,
      // Down behind the front, however tall the folder is.
      bottom: 14,
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
        top: 10,
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
        top: 0,
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
  <!-- The front, over the lower half of what peeks out, or the whole folder when nothing does. -->
  <View
    style={{
      position: "absolute",
      left: 0,
      right: 0,
      bottom: 0,
      top: peeking ? undefined : 0,
      height: peeking ? `${46 - 12 * opening}%` : undefined,
      radius: 12,
      background: target ? c.hover : c.surface,
      borderWidth: 1,
      borderColor: line,
      shadow: selected && !peeking ? ring : null,
      justify: peeking ? "end" : "center",
      gap: 2,
      padding: 12,
      transition: stateTransition,
    }}
  >
    <View style={{ direction: "row", align: "center", gap: 7 }}>
      <Image src={fileIcon(item.title, true, open || opening > 0.3)} style={{ size: 16, shrink: 0 }} />
      <Text style={{ flex: 1, fontSize: 13.5, truncate: true }}>{item.title}</Text>
    </View>
    {#if zoom >= 0.45}
      <Text style={{ fontSize: 11, color: c.faint }}>{count}</Text>
    {/if}
  </View>
</View>
