<script lang="ts">
  // Pick repositories to make projects of: yours and your organizations', newest first, or one
  // typed as owner/name. Each becomes a folder with its project note.
  import { Button, Input, Scroll, Text, View } from "@neoworks/svelte-gpui/ui";
  import type { GithubRepo } from "../../lib/api.ts";
  import { app } from "../../lib/state.svelte.ts";
  import { button, buttonHover, c, field, fieldFocus, ghost, ghostHover } from "../../lib/theme.ts";
  import { dialogs } from "./dialogs.svelte.ts";

  let repos = $state<GithubRepo[] | null>(null);
  let filter = $state("");
  let picked = $state<string[]>([]);
  let busy = $state(false);

  $effect(() => {
    if (!dialogs.addProjects || repos) return;
    void app.run(async () => {
      repos = await app.api.githubRepos();
    });
  });

  const typed = $derived(/^[\w.-]+\/[\w.-]+$/.test(filter.trim()) ? filter.trim() : null);
  const shown = $derived(
    (repos ?? []).filter((r) => !r.isArchived && r.nameWithOwner.toLowerCase().includes(filter.trim().toLowerCase())),
  );

  function toggle(name: string) {
    picked = picked.includes(name) ? picked.filter((p) => p !== name) : [...picked, name];
  }

  async function add() {
    const names = [...picked];
    if (typed && !names.includes(typed) && shown.length === 0) names.push(typed);
    busy = true;
    for (const name of names) {
      await app.run(() => app.api.createProject(name.split("/")[1], name));
    }
    busy = false;
    picked = [];
    repos = null;
    dialogs.addProjects = false;
    app.version++;
    void app.api.syncNow();
  }
</script>

{#if dialogs.addProjects}
  <View style={{ position: "absolute", inset: 0, background: "#00000088", align: "center", justify: "center" }}>
    <View
      style={{
        width: 560,
        height: 600,
        padding: 18,
        gap: 12,
        radius: 12,
        background: c.surface,
        borderWidth: 1,
        borderColor: c.border,
      }}
    >
      <Text style={{ fontSize: 16, fontWeight: "semibold" }}>Add projects</Text>
      <Input
        bind:value={filter}
        placeholder="Filter, or type owner/name"
        style={field}
        focus={fieldFocus}
        onsubmit={() => void add()}
      />
      <Scroll style={{ flex: 1, radius: 8, borderWidth: 1, borderColor: c.border }}>
        {#if !repos}
          <Text style={{ padding: 12, color: c.muted }}>Loading your repositories…</Text>
        {:else if shown.length === 0}
          <Text style={{ padding: 12, color: c.muted }}>
            {typed ? `Add ${typed} with the button below.` : "No repository matches."}
          </Text>
        {/if}
        {#each shown as repo (repo.nameWithOwner)}
          {@const on = picked.includes(repo.nameWithOwner)}
          <View
            style={{ direction: "row", align: "center", gap: 10, paddingX: 12, paddingY: 8, opacity: repo.taken ? 0.45 : 1 }}
            hover={{ background: c.hover }}
            onpress={() => !repo.taken && toggle(repo.nameWithOwner)}
          >
            <View
              style={{
                width: 16,
                height: 16,
                radius: 4,
                borderWidth: 1,
                borderColor: on ? c.accent : c.borderStrong,
                background: on ? c.accent : "transparent",
                align: "center",
                justify: "center",
              }}
            >
              {#if on}<Text style={{ fontSize: 11, color: "#fff" }}>✓</Text>{/if}
            </View>
            <View style={{ flex: 1, minWidth: 0 }}>
              <Text style={{ fontSize: 13, truncate: true }}>{repo.nameWithOwner}</Text>
              {#if repo.description}
                <Text style={{ fontSize: 11, color: c.faint, truncate: true }}>{repo.description}</Text>
              {/if}
            </View>
            {#if repo.taken}<Text style={{ fontSize: 11, color: c.faint }}>added</Text>{/if}
          </View>
        {/each}
      </Scroll>
      <View style={{ direction: "row", gap: 8, justify: "end" }}>
        <Button style={ghost} hover={ghostHover} onpress={() => (dialogs.addProjects = false)}>
          <Text>Cancel</Text>
        </Button>
        <Button
          style={{ ...button, background: c.accent, borderColor: c.accent }}
          hover={{ background: "#8d9bff" }}
          disabled={busy || (picked.length === 0 && !typed)}
          disabledStyle={{ opacity: 0.4 }}
          onpress={() => void add()}
        >
          <Text style={{ color: "#fff" }}>
            {busy ? "Adding…" : picked.length > 1 ? `Add ${picked.length} projects` : "Add project"}
          </Text>
        </Button>
      </View>
    </View>
  </View>
{/if}
