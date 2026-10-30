<script lang="ts">
  // An issue as a card: number, title, labels, Priority, who's on it, and whether it is blocked,
  // waiting to sync or in conflict. Drawn at `zoom` on a canvas, at 1 in lists.
  import { Text, View } from "@neoworks/svelte-gpui/ui";
  import type { Card } from "./api.ts";
  import { c, labelColor, tones } from "./theme.ts";

  interface Props {
    card: Card;
    zoom?: number;
    selected?: boolean;
    /** Fill the box it is given (a canvas node) instead of sizing to its content (a list). */
    fill?: boolean;
    showRepo?: boolean;
  }

  let { card, zoom = 1, selected = false, fill = false, showRepo = false }: Props = $props();

  const s = (n: number) => n * zoom;
  const closed = $derived(card.state === "CLOSED");
  const priority = $derived(card.fields.Priority as string | undefined);
  const priorityColor: Record<string, string> = {
    Urgent: tones.PINK,
    High: tones.RED,
    Medium: tones.YELLOW,
    Low: tones.GREEN,
  };
</script>

<View
  style={{
    flex: fill ? 1 : undefined,
    padding: [s(8), s(10)],
    gap: s(6),
    radius: s(8),
    background: c.raised,
    borderWidth: selected ? 2 : 1,
    borderColor: selected ? c.accent : card.conflict ? c.danger : c.border,
    overflow: "hidden",
    opacity: closed ? 0.6 : 1,
    transition: 140,
  }}
  hover={{ borderColor: selected ? c.accent : c.borderStrong }}
>
  {#if zoom < 0.45}
    <Text style={{ fontSize: s(26), color: c.text, fontWeight: "medium", maxLines: 3 }}>{card.title}</Text>
  {:else}
    <View style={{ direction: "row", align: "center", gap: s(6) }}>
      <View
        style={{ width: s(8), height: s(8), radius: s(4), background: closed ? tones.PURPLE : tones.GREEN, shrink: 0 }}
      />
      <Text style={{ fontSize: s(11), color: c.muted, nowrap: true }}>
        {showRepo ? card.repoName : ""}{card.number ? `#${card.number}` : "new"}
      </Text>
      <View style={{ flex: 1 }} />
      {#if card.blockedBy > 0}
        <Text style={{ fontSize: s(11), color: c.danger, nowrap: true }}>⛔ {card.blockedBy}</Text>
      {/if}
      {#if card.conflict}
        <Text style={{ fontSize: s(11), color: c.danger }}>conflict</Text>
      {:else if card.pending}
        <View style={{ width: s(6), height: s(6), radius: s(3), background: c.warn }} />
      {/if}
    </View>
    <Text style={{ fontSize: s(13), color: c.text, lineHeight: s(18), maxLines: 3 }}>{card.title}</Text>
    {#if card.labels.length > 0 || priority || card.assignees.length > 0}
      <View style={{ direction: "row", wrap: true, gap: s(4), align: "center" }}>
        {#if priority}
          <View
            style={{
              direction: "row",
              align: "center",
              gap: s(4),
              paddingX: s(5),
              height: s(18),
              radius: s(4),
              borderWidth: 1,
              borderColor: c.border,
            }}
          >
            <View style={{ width: s(6), height: s(6), radius: s(3), background: priorityColor[priority] ?? tones.GRAY }} />
            <Text style={{ fontSize: s(10), color: c.muted }}>{priority}</Text>
          </View>
        {/if}
        {#each card.labels.slice(0, 3) as label (label.name)}
          <View
            style={{
              direction: "row",
              align: "center",
              gap: s(4),
              paddingX: s(5),
              height: s(18),
              radius: s(9),
              borderWidth: 1,
              borderColor: c.border,
            }}
          >
            <View style={{ width: s(6), height: s(6), radius: s(3), background: labelColor(label.color) }} />
            <Text style={{ fontSize: s(10), color: c.muted, nowrap: true }}>{label.name}</Text>
          </View>
        {/each}
        {#if card.assignees.length > 0}
          <View style={{ flex: 1 }} />
          <Text style={{ fontSize: s(10), color: c.faint }}>@{card.assignees[0]}</Text>
        {/if}
      </View>
    {/if}
  {/if}
</View>
