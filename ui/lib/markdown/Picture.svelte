<script lang="ts">
  // An image of a Markdown document. Its source may be in more than one place (see
  // `imageSources`), so it tries each in turn until one loads, and shows its alt text when none does.
  import { Image, Text } from "@neoworks/svelte-gpui/ui";
  import type { Tokens } from "marked";
  import { c, radii } from "../theme.ts";
  import type { MarkdownContext } from "./Blocks.svelte";
  import { imageSources } from "./parse.ts";

  let { image, ctx }: { image: Tokens.Image; ctx: MarkdownContext } = $props();

  const sources = $derived(imageSources(image.href, ctx.base, ctx.workspace));
  let attempt = $state(0);

  $effect(() => {
    void sources;
    attempt = 0;
  });
</script>

{#if attempt < sources.length}
  <Image
    src={sources[attempt]}
    fit="contain"
    style={{ maxWidth: "full", maxHeight: 420, radius: radii.control, alignSelf: "start" }}
    onerror={() => attempt++}
  />
{:else}
  <Text style={{ fontSize: ctx.size * 0.9, color: c.faint }}>[{image.text || image.href}]</Text>
{/if}
