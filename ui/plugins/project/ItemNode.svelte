<script lang="ts">
  // Everything on a project canvas that isn't the board: a note (a card), an image, a file (a PDF
  // or video with its poster), a web link, an issue placed off the board, a local board, a folder.
  // Lengths are in world units and the canvas layer's zoom scales them; `zoom` picks the level
  // of detail.
  import { Icon, Image, Text, View } from "@neoworks/svelte-gpui/ui";
  import type { Item } from "../../lib/api.ts";
  import IssueCard from "../../lib/IssueCard.svelte";
  import { plainText } from "../../lib/markdown/plain.ts";
  import { c, ring } from "../../lib/theme.ts";
  import FolderNode from "./FolderNode.svelte";
  import LocalBoard from "./LocalBoard.svelte";
  import { icons } from "../../lib/icons.ts";
  import { duration, pages } from "../../lib/media.ts";
  import { contain } from "../../lib/canvas/geometry.ts";
  import { fileIcon } from "../../lib/fileIcons.ts";
  const { fileText, play } = icons;

  // `width` and `height` are the node's, in world units. `open` is a folder unfolded beside it,
  // `target` one that a dragged node would go into, `opening` how far open it is for nodes falling in,
  // and `gone` how far each of its peeking cards has left it as its thing comes out.
  let {
    item,
    zoom,
    selected,
    width,
    height,
    open = false,
    target = false,
    opening = 0,
    gone = [],
  }: {
    item: Item;
    zoom: number;
    selected: boolean;
    width: number;
    height: number;
    open?: boolean;
    target?: boolean;
    opening?: number;
    gone?: number[];
  } = $props();

  const excerpt = $derived(item.kind === "note" ? plainText(item.excerpt, 240) : "");
  /** The height left for a note's excerpt, and how many of its lines fit there. `onresize`
   *  measures in window pixels, so the room is divided by `zoom` to compare it with the
   *  world-unit line height. */
  let excerptRoom = $state(0);
  const excerptLines = $derived(Math.floor(excerptRoom / zoom / 18));
  const frame = $derived({
    flex: 1,
    radius: 10,
    background: c.surface,
    borderWidth: 1,
    shadow: selected ? ring : null,
    borderColor: selected ? c.accent : c.border,
    overflow: "hidden" as const,
    transition: 140,
  });
</script>

{#if item.kind === "issue"}
  <View style={{ flex: 1 }}>
    <IssueCard card={item.card} {zoom} {selected} fill showRepo />
  </View>
{:else if item.kind === "note"}
  <View style={{ ...frame, padding: 14, gap: 8 }} hover={{ borderColor: selected ? c.accent : c.borderStrong }}>
    {#if zoom >= 0.45}
      <View style={{ direction: "row", align: "center", gap: 6 }}>
        <Icon svg={fileText} size={13} color={c.faint} />
        <Text style={{ fontSize: 11, color: c.faint }}>Note</Text>
      </View>
    {/if}
    <!-- The card's height is the node's: the title keeps its lines, and the excerpt takes what
         room is left in whole lines, so nothing is cut through the middle. -->
    <Text style={{ fontSize: zoom < 0.45 ? 26 : 16, lineHeight: zoom < 0.45 ? 30 : 21, color: c.text, maxLines: 2, shrink: 0 }}>
      {item.title}
    </Text>
    {#if zoom >= 0.45}
      {#if excerpt}
        <View style={{ flex: 1, minHeight: 0 }} onresize={(l) => (excerptRoom = l.height)}>
          {#if excerptLines > 0}
            <Text style={{ fontSize: 12.5, color: c.muted, lineHeight: 18, maxLines: excerptLines }}>{excerpt}</Text>
          {/if}
        </View>
      {:else}
        <Text style={{ fontSize: 12.5, color: c.faint, lineHeight: 18 }}>Empty. Double-click to write.</Text>
      {/if}
    {/if}
  </View>
{:else if item.kind === "image"}
  {@const fitted = contain(width, height, item.aspect ?? 4 / 3)}
  <!-- The image itself, at its own proportions inside the node, as a poster is. Its border only
       shows when selected: around a PNG's transparent parts it would outline an empty box. -->
  <View style={{ flex: 1, align: "center", justify: "center" }}>
    <View
      style={{
        width: fitted.w,
        height: fitted.h,
        radius: 8,
        borderWidth: 1,
        borderColor: selected ? c.accent : "transparent",
        shadow: selected ? ring : null,
        transition: 140,
      }}
    >
      <Image src={item.src} fit="fill" style={{ position: "absolute", inset: 0, radius: 8 - 1 }} />
    </View>
  </View>
{:else if item.kind === "file" && item.media?.poster}
  {@const media = item.media}
  {@const pdf = item.mime === "application/pdf"}
  {@const fitted = contain(width, height, media.posterAspect ?? (pdf ? 0.707 : 16 / 9))}
  {@const corner = pdf ? 3 : 8}
  {@const badge = pdf && media.pages !== null ? pages(media.pages) : media.duration !== null ? duration(media.duration) : null}
  <!-- The page or the frame itself, at its own proportions inside the node, and nothing around
       it. GPUI rounds an image as drawn, so the image fills a box of exactly its shape. -->
  <View style={{ flex: 1, align: "center", justify: "center" }}>
    <View
      style={{
        width: fitted.w,
        height: fitted.h,
        radius: corner,
        borderWidth: 1,
        borderColor: selected ? c.accent : c.border,
        shadow: selected ? ring : null,
        align: "center",
        justify: "center",
        transition: 140,
      }}
    >
      <Image src={item.media.poster} fit="fill" style={{ position: "absolute", inset: 0, radius: corner - 1 }} />
      {#if !pdf}
        <View style={{ size: 40, radius: 20, background: "#000000a0", align: "center", justify: "center" }}>
          <Icon svg={play} size={18} color="#fff" />
        </View>
      {/if}
      {#if badge && zoom >= 0.45}
        <Text
          style={{ position: "absolute", right: 8, bottom: 8, fontSize: 10.5, color: "#fff", background: "#000000a0", radius: 4, paddingX: 5, paddingY: 2 }}
        >
          {badge}
        </Text>
      {/if}
    </View>
  </View>
{:else if item.kind === "file"}
  <View style={{ ...frame, direction: "row", align: "center", gap: 10, padding: 12 }}>
    <Image src={fileIcon(item.title)} style={{ size: 30, shrink: 0 }} />
    <View style={{ flex: 1, minWidth: 0 }}>
      <Text style={{ fontSize: 13, truncate: true }}>{item.title}</Text>
      <Text style={{ fontSize: 10, color: c.faint, truncate: true }}>{item.path}</Text>
    </View>
  </View>
{:else if item.kind === "link"}
  <View style={frame}>
    {#if item.preview?.image}
      <!-- The card's overflow clips square, so the image rounds its own top corners. GPUI rounds
           the image as drawn, not its box, so the box takes the image's proportions: a cover
           image scaled past its box would have its rounded corners cut off. 1.91 is Open Graph's
           recommended shape, for an image whose size couldn't be read. -->
      <Image
        src={item.preview.image}
        fit="cover"
        style={{
          width: "full",
          aspectRatio: item.preview.imageAspect ?? 1.91,
          shrink: 0,
          radius: [10 - 1, 10 - 1, 0, 0],
        }}
      />
    {/if}
    <View style={{ padding: 12, gap: 4, justify: item.preview?.image ? "start" : "center", flex: 1 }}>
      <View style={{ direction: "row", align: "center", gap: 6 }}>
        {#if item.preview?.favicon}
          <Image src={item.preview.favicon} fit="contain" style={{ width: 12, height: 12 }} />
        {/if}
        <Text style={{ fontSize: 10, color: c.accent, uppercase: true, truncate: true }}>
          {item.preview?.site ?? item.host}
        </Text>
      </View>
      <!-- Titles and descriptions often hold a URL or a repository path longer than the card. -->
      <Text style={{ fontSize: 13, maxLines: 2, overflowWrap: "anywhere" }}>{item.preview?.title ?? item.title}</Text>
      {#if item.preview?.description && zoom >= 0.45}
        <Text style={{ fontSize: 11.5, color: c.muted, lineHeight: 16, maxLines: 2, overflowWrap: "anywhere" }}>
          {item.preview.description}
        </Text>
      {/if}
    </View>
  </View>
{:else if item.kind === "folder"}
  <FolderNode {item} {zoom} {height} {selected} {open} {target} {opening} {gone} />
{:else if item.kind === "localboard"}
  <LocalBoard {item} {zoom} {selected} />
{:else if item.kind === "text"}
  <View style={{ ...frame, padding: 12, background: c.raised }}>
    <Text style={{ fontSize: 13, lineHeight: 18 }}>{item.text ?? ""}</Text>
  </View>
{:else if item.kind === "group"}
  <View style={{ flex: 1, radius: 14, borderWidth: 1, borderColor: c.borderStrong, padding: 10 }}>
    <Text style={{ fontSize: 12, color: c.muted }}>{item.title ?? ""}</Text>
  </View>
{:else}
  <View style={{ ...frame, padding: 12, borderColor: c.danger, justify: "center" }}>
    <Text style={{ fontSize: 12, color: c.muted }}>{item.kind === "missing" ? item.title : "?"}</Text>
    {#if item.uri}<Text style={{ fontSize: 10, color: c.faint, truncate: true }}>{item.uri}</Text>{/if}
  </View>
{/if}
