<script lang="ts">
  // The sync at a glance, at the top bar's end: synced, syncing, edits waiting, conflicts or an
  // error. Press it to sync now; the popover lists what failed.
  import { Button, Popover, Text, View } from "@neoworks/svelte-gpui/ui";
  import { app } from "../../lib/state.svelte.ts";
  import { c, ghost, ghostHover } from "../../lib/theme.ts";

  let now = $state(Date.now());
  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 15000);
    return () => clearInterval(timer);
  });

  const s = $derived(app.sync);
  const ago = $derived.by(() => {
    if (!s.lastSync) return "never";
    const seconds = Math.max(0, (now - Date.parse(s.lastSync)) / 1000);
    if (seconds < 60) return "just now";
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    return `${Math.floor(seconds / 3600)}h ago`;
  });
  const color = $derived(
    s.error || s.failed.length > 0 || s.conflicts > 0 ? c.danger : s.pending > 0 || s.running ? c.warn : c.ok,
  );
  const label = $derived(
    s.running
      ? "Syncing…"
      : s.error
        ? "Sync failed"
        : s.conflicts > 0
          ? `${s.conflicts} conflict${s.conflicts === 1 ? "" : "s"}`
          : s.pending > 0
            ? `${s.pending} waiting`
            : `Synced ${ago}`,
  );
  const problems = $derived(Boolean(s.error) || s.failed.length > 0);
  let open = $state(false);
</script>

<Popover bind:open side="bottom" align="end">
  {#snippet trigger({ toggle })}
    <Button
      style={ghost}
      hover={ghostHover}
      onpress={() => {
        if (problems) toggle();
        else void app.api.syncNow();
      }}
    >
      <View style={{ width: 8, height: 8, radius: 4, background: color }} />
      <Text>{label}</Text>
      {#if s.login}<Text style={{ color: c.faint }}>· {s.login}</Text>{/if}
    </Button>
  {/snippet}
  {#snippet children({ close })}
    <View style={{ width: 380, padding: 12, gap: 8, radius: 8, background: c.surface, borderWidth: 1, borderColor: c.border }}>
      {#if s.error}<Text style={{ fontSize: 12, color: c.danger }}>{s.error}</Text>{/if}
      {#each s.failed as failure (failure.target + (failure.field ?? ""))}
        <Text style={{ fontSize: 12, color: c.text }}>
          {failure.field ?? "edit"}: <Text style={{ color: c.muted }}>{failure.error}</Text>
        </Text>
      {/each}
      <Button
        style={ghost}
        hover={ghostHover}
        onpress={() => {
          close();
          void app.api.syncNow();
        }}
      >
        <Text>Try again</Text>
      </Button>
    </View>
  {/snippet}
</Popover>
