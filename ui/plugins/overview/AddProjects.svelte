<script lang="ts">
  // A new project: local, with its issues as files in its folder, or from GitHub repositories
  // (yours and your organizations', newest first, or one typed as owner/name). Each becomes a
  // folder with its project note.
  import { Button, Icon, Input, Presence, Scroll, Text, View } from "@neoworks/svelte-gpui/ui";
  import { cubicOut } from "svelte/easing";
  import { fade, fly } from "svelte/transition";
  import type { GithubRepo } from "../../lib/api.ts";
  import { app } from "../../lib/state.svelte.ts";
  import { c, eyebrow, field, fieldFocus, ghost, ghostHover, primary, primaryHover, radii, shadow } from "../../lib/theme.ts";
  import { dialogs } from "./dialogs.svelte.ts";
  import { icons } from "../../lib/icons.ts";
  const { check, folderClosed: folder, github } = icons;

  let mode = $state<"local" | "github">("local");
  let name = $state("");
  let repos = $state<GithubRepo[] | null>(null);
  let filter = $state("");
  let picked = $state<string[]>([]);
  let busy = $state(false);
  let nameField = $state<Input>();
  let filterField = $state<Input>();

  $effect(() => {
    if (!dialogs.addProjects || mode !== "github" || repos) return;
    void app.run(async () => {
      repos = await app.api.githubRepos();
    });
  });

  $effect(() => {
    if (!dialogs.addProjects) return;
    if (mode === "local") nameField?.focus();
    else filterField?.focus();
  });

  const typed = $derived(/^[\w.-]+\/[\w.-]+$/.test(filter.trim()) ? filter.trim() : null);
  const shown = $derived(
    (repos ?? []).filter((r) => !r.isArchived && r.nameWithOwner.toLowerCase().includes(filter.trim().toLowerCase())),
  );
  const ready = $derived(mode === "local" ? name.trim().length > 0 : picked.length > 0 || typed !== null);

  function toggle(repo: string) {
    if (picked.includes(repo)) picked = picked.filter((p) => p !== repo);
    else picked = [...picked, repo];
  }

  function close() {
    dialogs.addProjects = false;
    picked = [];
    name = "";
    filter = "";
  }

  async function create() {
    if (!ready || busy) return;
    busy = true;
    if (mode === "local") {
      const project = await app.run(() => app.api.createProject(name.trim(), null));
      busy = false;
      close();
      app.version++;
      if (project) app.open({ view: "project", project: project.folder });
      return;
    }
    const names = [...picked];
    if (typed && !names.includes(typed) && shown.length === 0) names.push(typed);
    for (const repo of names) {
      await app.run(() => app.api.createProject(repo.split("/")[1], repo));
    }
    busy = false;
    repos = null;
    close();
    app.version++;
    void app.api.syncNow();
  }

  function age(iso: string): string {
    const days = Math.floor((Date.now() - Date.parse(iso)) / 86_400_000);
    if (days < 1) return "today";
    if (days < 30) return `${days}d ago`;
    if (days < 365) return `${Math.floor(days / 30)}mo ago`;
    return `${Math.floor(days / 365)}y ago`;
  }
</script>

{#snippet choice(value: "local" | "github", icon: string, title: string, text: string)}
  {@const on = mode === value}
  <Button
    style={{
      flex: 1,
      direction: "column",
      align: "start",
      gap: 8,
      padding: 14,
      radius: radii.card,
      borderWidth: 1,
      borderColor: on ? c.accent : c.border,
      background: on ? c.accentSoft : c.raised,
      transition: 140,
    }}
    hover={{ borderColor: on ? c.accent : c.borderStrong }}
    onpress={() => (mode = value)}
  >
    <View style={{ direction: "row", align: "center", gap: 8 }}>
      <Icon svg={icon} size={18} color={on ? c.accentText : c.muted} />
      <Text style={{ fontSize: 14, fontWeight: "medium", color: c.text }}>{title}</Text>
    </View>
    <Text style={{ fontSize: 12, lineHeight: 17, color: c.muted }}>{text}</Text>
  </Button>
{/snippet}

{#if dialogs.addProjects}
  <Presence
    in={[fade, { duration: 160 }]}
    out={[fade, { duration: 140 }]}
    style={{ position: "absolute", inset: 0, background: "#000000a0", align: "center", justify: "center" }}
  >
    <View style={{ position: "absolute", inset: 0 }} onpress={close} />
    <Presence
      in={[fly, { y: 16, duration: 240, easing: cubicOut }]}
      style={{
        width: 560,
        height: mode === "github" ? 620 : undefined,
        padding: 20,
        gap: 16,
        radius: radii.panel + 4,
        background: c.surface,
        borderWidth: 1,
        borderColor: c.border,
        shadow,
      }}
    >
      <View style={{ gap: 4 }} onpress={() => {}}>
        <Text style={{ fontSize: 22, color: c.text }}>New project</Text>
        <Text style={{ fontSize: 13, color: c.muted }}>A folder in your workspace, with its board, notes and canvas.</Text>
      </View>
      <View style={{ direction: "row", gap: 10 }}>
        {@render choice("local", folder, "Local", "Issues are Markdown files in the project's folder. Nothing leaves your computer.")}
        {@render choice("github", github, "From GitHub", "One project per repository. Issues sync both ways, as you.")}
      </View>

      {#if mode === "local"}
        <View style={{ gap: 6 }}>
          <Text style={eyebrow}>Name</Text>
          <Input
            bind:this={nameField}
            bind:value={name}
            placeholder="Garden, Thesis, Website…"
            style={{ ...field, fontSize: 15, paddingY: 10 }}
            focus={fieldFocus}
            onsubmit={() => void create()}
          />
        </View>
      {:else}
        <Input
          bind:this={filterField}
          bind:value={filter}
          placeholder="Search your repositories, or type owner/name"
          style={{ ...field, paddingY: 9 }}
          focus={fieldFocus}
          onsubmit={() => void create()}
        />
        <Scroll style={{ flex: 1, radius: radii.card, borderWidth: 1, borderColor: c.border }}>
          {#if !repos}
            <Text style={{ padding: 14, color: c.muted }}>Loading your repositories…</Text>
          {:else if shown.length === 0}
            <Text style={{ padding: 14, color: c.muted }}>
              {typed ? `Press Create to add ${typed}.` : "No repository matches."}
            </Text>
          {/if}
          {#each shown as repo (repo.nameWithOwner)}
            {@const on = picked.includes(repo.nameWithOwner)}
            <View
              style={{ direction: "row", align: "center", gap: 12, paddingX: 14, paddingY: 9, opacity: repo.taken ? 0.45 : 1, transition: 100 }}
              hover={{ background: c.hover }}
              onpress={() => !repo.taken && toggle(repo.nameWithOwner)}
            >
              <View
                style={{
                  size: 18,
                  radius: 5,
                  borderWidth: 1,
                  borderColor: on ? c.accent : c.borderStrong,
                  background: on ? c.accent : "transparent",
                  align: "center",
                  justify: "center",
                  transition: 120,
                }}
              >
                {#if on}<Icon svg={check} size={11} color={c.onAccent} />{/if}
              </View>
              <View style={{ flex: 1, minWidth: 0, gap: 2 }}>
                <Text style={{ fontSize: 13, truncate: true }}>{repo.nameWithOwner}</Text>
                {#if repo.description}
                  <Text style={{ fontSize: 12, color: c.faint, truncate: true }}>{repo.description}</Text>
                {/if}
              </View>
              <Text style={{ fontSize: 11, color: c.faint }}>{repo.taken ? "added" : age(repo.updatedAt)}</Text>
            </View>
          {/each}
        </Scroll>
      {/if}

      <View style={{ direction: "row", gap: 8, justify: "end", align: "center" }}>
        {#if mode === "github" && repos}
          <Text style={{ flex: 1, fontSize: 12, color: c.faint }}>{repos.length} repositories</Text>
        {/if}
        <Button style={ghost} hover={ghostHover} onpress={close}>
          <Text>Cancel</Text>
        </Button>
        <Button
          style={primary}
          hover={primaryHover}
          disabled={busy || !ready}
          disabledStyle={{ opacity: 0.35 }}
          onpress={() => void create()}
        >
          <Text>
            {busy ? "Creating…" : mode === "github" && picked.length > 1 ? `Create ${picked.length} projects` : "Create project"}
          </Text>
        </Button>
      </View>
    </Presence>
  </Presence>
{/if}
