<script lang="ts" module>
  /** What every block of one document shares: its type size, where its images live, what a link does. */
  export interface MarkdownContext {
    /** The body's font size; headings and code scale from it. */
    size: number;
    /** The folder relative image paths are resolved against, or null. */
    base: string | null;
    onlink: (href: string) => void;
    /** A press on the text that isn't a link: the place to start editing. */
    onplain: () => void;
    /** Off where the text sits on something that is dragged, such as a sticky note. */
    interactive: boolean;
  }
</script>

<script lang="ts">
  // Markdown blocks drawn with the window's primitives: headings, paragraphs, lists and task
  // lists, quotes, code, tables, rules and images. Lists and quotes hold blocks of their own,
  // so this draws itself for them.
  import { Icon, Image, Scroll, Text, View } from "@neoworks/svelte-gpui/ui";
  import type { Token, Tokens } from "marked";
  import { c, radii } from "../theme.ts";
  import Blocks from "./Blocks.svelte";
  import Code from "./Code.svelte";
  import Inline from "./Inline.svelte";
  import { links, onlyImages, resolveSrc } from "./parse.ts";
  import { icons } from "../icons.ts";
  const { taskDone: checkSquare, taskOpen: square } = icons;

  let { tokens, ctx, tight = false }: { tokens: Token[]; ctx: MarkdownContext; tight?: boolean } = $props();

  const headingScale = [0, 1.75, 1.4, 1.2, 1.05, 1, 0.9];
  const line = $derived(Math.round(ctx.size * 1.6));

  /** A paragraph with one link opens it when pressed; any other press is on plain text. */
  function press(inline: Token[]) {
    const found = links(inline);
    if (found.length === 1) ctx.onlink(found[0]);
    else ctx.onplain();
  }
</script>

{#each tokens as token, i (i)}
  {#if token.type === "heading"}
    {@const h = token as Tokens.Heading}
    <Text
      style={{
        fontSize: ctx.size * headingScale[h.depth],
        lineHeight: ctx.size * headingScale[h.depth] * 1.3,
        fontWeight: h.depth <= 2 ? "normal" : "semibold",
        color: c.text,
        marginTop: i === 0 ? 0 : ctx.size * (h.depth <= 2 ? 0.9 : 0.5),
      }}
    >
      <Inline tokens={h.tokens} />
    </Text>
    {#if h.depth === 1}<View style={{ height: 1, background: c.border, marginTop: 2 }} />{/if}
  {:else if token.type === "paragraph"}
    {@const p = token as Tokens.Paragraph}
    {@const images = onlyImages(p.tokens)}
    {#if images}
      <View style={{ direction: "row", wrap: true, gap: 8 }}>
        {#each images as image, j (j)}
          <Image
            src={resolveSrc(image.href, ctx.base)}
            fit="contain"
            style={{ maxWidth: "full", maxHeight: 420, radius: radii.control, alignSelf: "start" }}
          />
        {/each}
      </View>
    {:else}
      {#if ctx.interactive}
        <View onpress={() => press(p.tokens)}>
          <Text style={{ fontSize: ctx.size, lineHeight: line, color: c.text }}><Inline tokens={p.tokens} /></Text>
        </View>
      {:else}
        <Text style={{ fontSize: ctx.size, lineHeight: line, color: c.text }}><Inline tokens={p.tokens} /></Text>
      {/if}
    {/if}
  {:else if token.type === "text"}
    {@const t = token as Tokens.Text}
    <Text style={{ fontSize: ctx.size, lineHeight: line, color: c.text }}><Inline tokens={t.tokens ?? [t]} /></Text>
  {:else if token.type === "list"}
    {@const list = token as Tokens.List}
    <View style={{ gap: tight || !list.loose ? 2 : ctx.size * 0.5 }}>
      {#each list.items as item, j (j)}
        <View style={{ direction: "row", gap: 8, align: "start" }}>
          <View style={{ width: ctx.size * 1.2, height: line, align: "end", justify: "center", shrink: 0 }}>
            {#if item.task}
              <Icon svg={item.checked ? checkSquare : square} size={ctx.size * 1.05} color={item.checked ? c.accentText : c.faint} />
            {:else if list.ordered}
              <Text style={{ fontSize: ctx.size * 0.92, color: c.faint }}>{Number(list.start || 1) + j}.</Text>
            {:else}
              <View style={{ size: 5, radius: 3, background: c.faint }} />
            {/if}
          </View>
          <View style={{ flex: 1, minWidth: 0, gap: 4, opacity: item.task && item.checked ? 0.6 : 1 }}>
            <Blocks tokens={item.tokens.filter((t) => t.type !== "checkbox")} {ctx} tight />
          </View>
        </View>
      {/each}
    </View>
  {:else if token.type === "blockquote"}
    {@const q = token as Tokens.Blockquote}
    <View style={{ direction: "row", gap: 12 }}>
      <View style={{ width: 3, radius: 2, background: c.borderStrong }} />
      <View style={{ flex: 1, minWidth: 0, gap: 8, opacity: 0.85 }}>
        <Blocks tokens={q.tokens} {ctx} />
      </View>
    </View>
  {:else if token.type === "code"}
    {@const code = token as Tokens.Code}
    <View style={{ radius: radii.control, background: c.raised, borderWidth: 1, borderColor: c.border, overflow: "hidden" }}>
      {#if code.lang}
        <Text style={{ fontSize: 11, color: c.faint, paddingX: 12, paddingTop: 8 }}>{code.lang}</Text>
      {/if}
      <Scroll horizontal style={{ maxWidth: "full" }}>
        <View style={{ padding: [code.lang ? 4 : 10, 12, 10, 12] }}>
          <Code code={code.text} lang={code.lang ?? ""} size={ctx.size} />
        </View>
      </Scroll>
    </View>
  {:else if token.type === "table"}
    {@const table = token as Tokens.Table}
    <Scroll horizontal style={{ maxWidth: "full" }}>
      <View style={{ radius: radii.control, borderWidth: 1, borderColor: c.border, overflow: "hidden" }}>
        <View style={{ direction: "row", background: c.raised }}>
          {#each table.header as cell, k (k)}
            <Text style={{ width: 160, padding: [7, 10], fontSize: ctx.size * 0.9, fontWeight: "semibold", color: c.text, textAlign: table.align[k] ?? "left" }}><Inline tokens={cell.tokens} /></Text>
          {/each}
        </View>
        {#each table.rows as row, r (r)}
          <View style={{ direction: "row", borderTopWidth: 1, borderColor: c.border }}>
            {#each row as cell, k (k)}
              <Text style={{ width: 160, padding: [7, 10], fontSize: ctx.size * 0.9, color: c.text, textAlign: table.align[k] ?? "left" }}><Inline tokens={cell.tokens} /></Text>
            {/each}
          </View>
        {/each}
      </View>
    </Scroll>
  {:else if token.type === "hr"}
    <View style={{ height: 1, background: c.border, marginY: ctx.size * 0.4 }} />
  {:else if token.type === "html"}
    <Text pre style={{ fontFamily: "mono", fontSize: ctx.size * 0.85, color: c.faint }}>{(token as Tokens.HTML).text.trim()}</Text>
  {/if}
{/each}
