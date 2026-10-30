<script lang="ts">
  // A Markdown document, rendered. Links open in the browser unless `onlink` says otherwise.
  import { View } from "@neoworks/svelte-gpui/ui";
  import { app } from "../state.svelte.ts";
  import Blocks from "./Blocks.svelte";
  import { parse } from "./parse.ts";

  interface Props {
    source: string;
    /** The body's font size. */
    size?: number;
    /** The folder relative image paths are resolved against. */
    base?: string | null;
    onlink?: (href: string) => void;
    /** A press anywhere that isn't a link. */
    onpress?: () => void;
    /** Off to leave presses to what is around the text, such as a node being dragged. */
    interactive?: boolean;
  }

  let { source, size = 15, base = null, onlink, onpress, interactive = true }: Props = $props();

  const tokens = $derived(parse(source));
  const ctx = $derived({
    size,
    base,
    workspace: app.workspace,
    onlink: onlink ?? ((href: string) => void app.run(() => app.api.open(href))),
    onplain: () => onpress?.(),
    interactive,
  });
</script>

{#if interactive}
  <View style={{ gap: Math.round(size * 0.75) }} onpress={() => onpress?.()}>
    <Blocks {tokens} {ctx} />
  </View>
{:else}
  <View style={{ gap: Math.round(size * 0.75) }}>
    <Blocks {tokens} {ctx} />
  </View>
{/if}
