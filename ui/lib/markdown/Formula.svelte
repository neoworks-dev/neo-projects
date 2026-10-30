<script lang="ts" module>
  import type { Formula } from "../api.ts";

  /** Each formula is typeset once a session, and every place that shows it waits on the same answer. */
  const typeset = new Map<string, Promise<Formula>>();
</script>

<script lang="ts">
  // A LaTeX formula, typeset by the host's MathJax and drawn as an icon in the text's colour.
  // Inside a `Text` it sits in the line, lowered by its depth so its baseline meets the text's.
  // Until it is ready it shows its source, and in red when TeX can't read it.
  import { Icon, Text } from "@neoworks/svelte-gpui/ui";
  import { app } from "../state.svelte.ts";
  import { c } from "../theme.ts";

  let { tex, display, size }: { tex: string; display: boolean; size: number } = $props();

  let formula = $state<Formula | null>(null);
  let failed = $state(false);

  $effect(() => {
    const key = `${display ? "display" : "inline"}:${tex}`;
    let pending = typeset.get(key);
    if (!pending) {
      pending = app.api.math(tex, display);
      typeset.set(key, pending);
    }
    formula = null;
    failed = false;
    let current = true;
    pending.then(
      (done) => {
        if (current) formula = done;
      },
      () => {
        if (current) failed = true;
      },
    );
    return () => {
      current = false;
    };
  });
</script>

{#if formula}
  <Icon
    svg={formula.svg}
    style={{ width: formula.width * size, height: formula.height * size, verticalAlign: -formula.depth * size }}
  />
{:else}
  <Text style={{ fontFamily: "mono", fontSize: size * 0.9, color: failed ? c.danger : c.faint }}>{tex}</Text>
{/if}
