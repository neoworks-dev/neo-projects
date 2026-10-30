<script lang="ts" module>
  // Highlighted blocks, by language and code, so a re-render doesn't ask the host again.
  const cache = new Map<string, { text: string; kind: string }[]>();
</script>

<script lang="ts">
  // A code block, highlighted by the host. Until the runs arrive, and for languages it doesn't
  // know, the code shows plain.
  import { Text } from "@neoworks/svelte-gpui/ui";
  import { app } from "../state.svelte.ts";
  import { c, codeColors } from "../theme.ts";

  let { code, lang, size }: { code: string; lang: string; size: number } = $props();

  let runs = $state<{ text: string; kind: string }[] | null>(null);

  $effect(() => {
    const key = `${lang}:${code}`;
    const known = cache.get(key);
    if (known) {
      runs = known;
      return;
    }
    runs = null;
    if (!lang) return;
    void app.api.highlight(code, lang).then((found) => {
      cache.set(key, found);
      if (key === `${lang}:${code}`) runs = found;
    });
  });
</script>

<!-- One line on purpose: a `pre` paragraph would keep the template's line breaks and indents. -->
<Text pre style={{ fontFamily: "mono", fontSize: size * 0.85, lineHeight: size * 1.4, color: c.text }}>{#if runs}{#each runs as run, i (i)}<Text style={{ color: codeColors[run.kind] ?? c.text, italic: run.kind === "comment" }}>{run.text}</Text>{/each}{:else}{code}{/if}</Text>
