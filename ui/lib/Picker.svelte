<script lang="ts" module>
  export interface PickerOption {
    id: string;
    label: string;
    /** A dot in this colour before the label. */
    color?: string;
    /** Text after the label, such as a count or a key. */
    hint?: string;
  }
</script>

<script lang="ts">
  // A property's value as a quiet button that opens a list to pick from, anchored to it: filter as
  // you type, a dot in each option's colour, a check on what is chosen. One choice closes it;
  // with `multiple`, each press toggles and it stays open.
  import { Button, Icon, Input, Popover, Scroll, Text, View } from "@neoworks/svelte-gpui/ui";
  import check from "@phosphor-icons/core/bold/check-bold.svg";
  import type { Snippet } from "svelte";
  import { c, radii, shadow } from "./theme.ts";

  interface Props {
    options: PickerOption[];
    chosen: string[];
    multiple?: boolean;
    /** Shown as the first option when one may pick nothing. */
    noneLabel?: string;
    placeholder?: string;
    onpick: (id: string | null) => void;
    /** What the button shows. */
    children: Snippet;
  }

  let { options, chosen, multiple = false, noneLabel, placeholder = "Filter…", onpick, children: shownValue }: Props = $props();

  let open = $state(false);
  let query = $state("");
  let filter = $state<Input>();

  const shown = $derived(
    query.trim() ? options.filter((o) => o.label.toLowerCase().includes(query.trim().toLowerCase())) : options,
  );

  $effect(() => {
    if (open) filter?.focus();
    else query = "";
  });

  function pick(id: string | null, close: () => void) {
    onpick(id);
    if (!multiple) close();
  }
</script>

{#snippet row(id: string | null, label: string, color: string | undefined, hint: string | undefined, close: () => void)}
  {@const on = id === null ? chosen.length === 0 : chosen.includes(id)}
  <Button
    style={{ direction: "row", align: "center", gap: 9, paddingX: 9, height: 30, radius: 7, transition: 100 }}
    hover={{ background: c.hover }}
    onpress={() => pick(id, close)}
  >
    {#if color}
      <View style={{ width: 8, height: 8, radius: 4, background: color, shrink: 0 }} />
    {/if}
    <Text style={{ flex: 1, fontSize: 13, color: id === null ? c.muted : c.text, truncate: true }}>{label}</Text>
    {#if hint}<Text style={{ fontSize: 11, color: c.faint }}>{hint}</Text>{/if}
    <View style={{ width: 14, align: "center" }}>
      {#if on}<Icon svg={check} size={12} color={c.accentText} />{/if}
    </View>
  </Button>
{/snippet}

<Popover
  bind:open
  side="bottom"
  align="start"
  focusPanel={false}
  style={{ width: 260, padding: 5, gap: 4, radius: radii.card, background: c.surface, borderWidth: 1, borderColor: c.border, shadow }}
>
  {#snippet trigger({ toggle })}
    <Button
      style={{ direction: "row", align: "center", gap: 6, minHeight: 28, paddingX: 8, radius: 7, transition: 120 }}
      hover={{ background: c.hover }}
      onpress={toggle}
    >
      {@render shownValue()}
    </Button>
  {/snippet}
  {#snippet children({ close })}
    {#if options.length > 7}
      <Input
        bind:this={filter}
        bind:value={query}
        {placeholder}
        style={{ paddingX: 9, height: 30, fontSize: 13, color: c.text, radius: 7, background: c.raised }}
        onsubmit={() => {
          if (shown.length > 0) pick(shown[0].id, () => close("select"));
        }}
      />
    {/if}
    <Scroll style={{ maxHeight: 300 }}>
      <View style={{ gap: 1 }}>
        {#if noneLabel && !query}
          {@render row(null, noneLabel, undefined, undefined, () => close("select"))}
        {/if}
        {#each shown as option (option.id)}
          {@render row(option.id, option.label, option.color, option.hint, () => close("select"))}
        {/each}
        {#if shown.length === 0}
          <Text style={{ fontSize: 12, color: c.faint, padding: 9 }}>Nothing matches</Text>
        {/if}
      </View>
    </Scroll>
  {/snippet}
</Popover>
