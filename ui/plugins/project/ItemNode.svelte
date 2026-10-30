<script lang="ts">
  // Everything on a project canvas that isn't the board: a note (a card), an image, a file, a web
  // link, an issue placed off the board, a local board. Drawn at `zoom`.
  import { Icon, Image, Text, View } from "@neoworks/svelte-gpui/ui";
  import type { Item } from "../../lib/api.ts";
  import IssueCard from "../../lib/IssueCard.svelte";
  import { plainText } from "../../lib/markdown/plain.ts";
  import { c, ring } from "../../lib/theme.ts";
  import LocalBoard from "./LocalBoard.svelte";
  import { icons } from "../../lib/icons.ts";
  const { fileText } = icons;

  let { item, zoom, selected }: { item: Item; zoom: number; selected: boolean } = $props();

  const s = (n: number) => n * zoom;
  const excerpt = $derived(item.kind === "note" ? plainText(item.excerpt, 240) : "");
  const frame = $derived({
    flex: 1,
    radius: s(10),
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
  <View style={{ ...frame, padding: s(14), gap: s(8) }} hover={{ borderColor: selected ? c.accent : c.borderStrong }}>
    {#if zoom >= 0.45}
      <View style={{ direction: "row", align: "center", gap: s(6) }}>
        <Icon svg={fileText} size={s(13)} color={c.faint} />
        <Text style={{ fontSize: s(11), color: c.faint }}>Note</Text>
      </View>
    {/if}
    <Text style={{ fontSize: s(zoom < 0.45 ? 26 : 16), lineHeight: s(zoom < 0.45 ? 30 : 21), color: c.text, maxLines: 2 }}>{item.title}</Text>
    {#if zoom >= 0.45}
      {#if excerpt}
        <Text style={{ fontSize: s(12.5), color: c.muted, lineHeight: s(18) }}>{excerpt}</Text>
      {:else}
        <Text style={{ fontSize: s(12.5), color: c.faint, lineHeight: s(18) }}>Empty. Double-click to write.</Text>
      {/if}
    {/if}
  </View>
{:else if item.kind === "image"}
  <View style={{ ...frame, background: c.bg }}>
    <Image src={item.path} fit="contain" style={{ flex: 1 }} />
    {#if zoom >= 0.5}
      <Text style={{ fontSize: s(11), color: c.muted, padding: [s(4), s(8)], truncate: true }}>{item.title}</Text>
    {/if}
  </View>
{:else if item.kind === "file"}
  <View style={{ ...frame, direction: "row", align: "center", gap: s(10), padding: s(12) }}>
    <View
      style={{ width: s(34), height: s(40), radius: s(4), background: c.raised, align: "center", justify: "center" }}
    >
      <Text style={{ fontSize: s(9), color: c.muted, uppercase: true }}>{item.title.split(".").pop()}</Text>
    </View>
    <View style={{ flex: 1, minWidth: 0 }}>
      <Text style={{ fontSize: s(13), truncate: true }}>{item.title}</Text>
      <Text style={{ fontSize: s(10), color: c.faint, truncate: true }}>{item.path}</Text>
    </View>
  </View>
{:else if item.kind === "link"}
  <View style={{ ...frame, padding: s(12), gap: s(4), justify: "center" }}>
    <Text style={{ fontSize: s(10), color: c.accent, uppercase: true }}>{item.host}</Text>
    <Text style={{ fontSize: s(13), maxLines: 2 }}>{item.title}</Text>
  </View>
{:else if item.kind === "localboard"}
  <LocalBoard {item} {zoom} {selected} />
{:else if item.kind === "text"}
  <View style={{ ...frame, padding: s(12), background: c.raised }}>
    <Text style={{ fontSize: s(13), lineHeight: s(18) }}>{item.text ?? ""}</Text>
  </View>
{:else if item.kind === "group"}
  <View style={{ flex: 1, radius: s(14), borderWidth: 1, borderColor: c.borderStrong, padding: s(10) }}>
    <Text style={{ fontSize: s(12), color: c.muted }}>{item.title ?? ""}</Text>
  </View>
{:else}
  <View style={{ ...frame, padding: s(12), borderColor: c.danger, justify: "center" }}>
    <Text style={{ fontSize: s(12), color: c.muted }}>{item.kind === "missing" ? item.title : "?"}</Text>
    {#if item.uri}<Text style={{ fontSize: s(10), color: c.faint, truncate: true }}>{item.uri}</Text>{/if}
  </View>
{/if}
