<script lang="ts">
  // An issue as a card: number, title, labels, Priority, who's on it, and whether it is blocked,
  // waiting to sync or in conflict. Drawn at `zoom` on a canvas, at 1 in lists.
  import { Icon, Text, View } from "@neoworks/svelte-gpui/ui";
  import { icons, stateIcon } from "./icons.ts";
  import type { Card } from "./api.ts";
  import Avatar from "./Avatar.svelte";
  import { plainText } from "./markdown/plain.ts";
  import { c, labelColor, ring, tones } from "./theme.ts";

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
  const stateMark = $derived(stateIcon(card));
  const excerpt = $derived(fill ? plainText(card.excerpt ?? "", 320) : "");
  /** The height left for the excerpt in a filled card, and how many of its lines fit there. */
  let excerptRoom = $state(0);
  const excerptLines = $derived(Math.min(4, Math.floor(excerptRoom / s(17.5))));
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
    borderWidth: 1,
    shadow: selected ? ring : null,
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
    <View style={{ direction: "row", align: "center", gap: s(6), shrink: 0 }}>
      <Icon svg={stateMark.icon} size={s(14)} color={stateMark.color} />
      <Text style={{ fontSize: s(11), color: c.muted, nowrap: true }}>
        {showRepo ? card.repoName : ""}{card.number ? `#${card.number}` : "new"}
      </Text>
      <View style={{ flex: 1 }} />
      {#if card.blockedBy > 0}
        <View style={{ direction: "row", align: "center", gap: s(3) }}>
          <Icon svg={icons.blocked} size={s(12)} color={c.danger} />
          <Text style={{ fontSize: s(11), color: c.danger, nowrap: true }}>{card.blockedBy}</Text>
        </View>
      {/if}
      {#if card.conflict}
        <Text style={{ fontSize: s(11), color: c.danger }}>conflict</Text>
      {:else if card.pending}
        <View style={{ width: s(6), height: s(6), radius: s(3), background: c.warn }} />
      {/if}
    </View>
    <Text style={{ fontSize: s(13), color: c.text, lineHeight: s(18), maxLines: fill ? 2 : 3, shrink: 0 }}>{card.title}</Text>
    {#if fill}
      <!-- The excerpt takes what room is left, in whole lines, so nothing is cut through the middle. -->
      <View style={{ flex: 1, minHeight: 0 }} onresize={(l) => (excerptRoom = l.height)}>
        {#if excerpt && excerptLines > 0}
          <Text style={{ fontSize: s(12), color: c.muted, lineHeight: s(17.5), maxLines: excerptLines }}>{excerpt}</Text>
        {/if}
      </View>
    {/if}
    {#if card.labels.length > 0 || priority || card.assignees.length > 0}
      <View style={{ direction: "row", gap: s(4), align: "center", shrink: 0 }}>
        <View style={{ flex: 1, minWidth: 0, direction: "row", wrap: true, gap: s(4), align: "center" }}>
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
        </View>
        {#if card.assignees.length > 0}
          <View style={{ direction: "row" }}>
            {#each card.assignees.slice(0, 3) as login, i (login)}
              <View style={{ marginLeft: i > 0 ? -s(6) : 0 }}>
                <Avatar {login} size={s(20)} github={!card.local} outline={c.raised} />
              </View>
            {/each}
          </View>
        {/if}
      </View>
    {/if}
  {/if}
</View>
