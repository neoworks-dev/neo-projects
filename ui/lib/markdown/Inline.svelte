<script lang="ts">
  // Inline Markdown as runs of one paragraph: emphasis, code, links and line breaks are nested
  // `Text`s, so the paragraph wraps as one, and a formula sits in its lines. Runs can't be
  // pressed one by one; the paragraph around them decides what a press does (see `Markdown.svelte`).
  import { Text } from "@neoworks/svelte-gpui/ui";
  import type { Token } from "marked";
  import { c } from "../theme.ts";
  import Formula from "./Formula.svelte";
  import Inline from "./Inline.svelte";
  import type { MathToken } from "./parse.ts";

  /** `size` is the paragraph's font size, which a formula is scaled to. */
  let { tokens, size }: { tokens: Token[]; size: number } = $props();
</script>

{#each tokens as token, i (i)}
  {#if token.type === "strong"}
    <Text style={{ fontWeight: "semibold" }}><Inline tokens={token.tokens ?? []} {size} /></Text>
  {:else if token.type === "em"}
    <Text style={{ italic: true }}><Inline tokens={token.tokens ?? []} {size} /></Text>
  {:else if token.type === "del"}
    <Text style={{ strikethrough: true, color: c.faint }}><Inline tokens={token.tokens ?? []} {size} /></Text>
  {:else if token.type === "codespan"}
    <Text style={{ fontFamily: "mono", background: c.raised, color: c.accentText }}>{` ${decode(token.text)} `}</Text>
  {:else if token.type === "link"}
    <Text style={{ color: c.accentText, underline: true }}><Inline tokens={token.tokens ?? []} {size} /></Text>
  {:else if token.type === "math"}
    {@const math = token as unknown as MathToken}
    <Formula tex={math.text} display={math.display} {size} />
  {:else if token.type === "image"}
    <!-- The block draws the picture after the text. -->
  {:else if token.type === "br"}
    <Text>{"\n"}</Text>
  {:else if token.type === "text" && "tokens" in token && token.tokens}
    <Inline tokens={token.tokens} {size} />
  {:else if token.type === "html"}
    <Text style={{ color: c.faint }}>{token.text}</Text>
  {:else if "text" in token}
    <Text>{decode(String(token.text))}</Text>
  {/if}
{/each}

<script lang="ts" module>
  // marked escapes `&`, `<`, `>` and quotes in text for HTML; the window draws text as it is.
  const entities: Record<string, string> = { "&amp;": "&", "&lt;": "<", "&gt;": ">", "&quot;": '"', "&#39;": "'" };

  function decode(text: string): string {
    return text.replace(/&(amp|lt|gt|quot|#39);/g, (m) => entities[m]);
  }
</script>
