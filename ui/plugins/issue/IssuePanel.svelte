<script lang="ts">
  // One issue, at the right: everything editable goes to GitHub as you (through the outbox), and
  // shows at once. Title, body, status and fields, labels and assignees, what blocks what, and the
  // comments.
  import { Button, Input, Popover, Scroll, Select, Text, View } from "@neoworks/svelte-gpui/ui";
  import type { Card, IssueDetail, Linked } from "../../lib/api.ts";
  import { app } from "../../lib/state.svelte.ts";
  import { button, buttonHover, c, field, fieldFocus, ghost, ghostHover, labelColor } from "../../lib/theme.ts";

  let { node }: { node: string } = $props();

  let issue = $state<IssueDetail | null>(null);
  let title = $state("");
  let body = $state("");
  let reply = $state("");
  let search = $state("");
  let found = $state<Card[]>([]);
  let linking = $state<"blockedBy" | "blocking" | "parent" | null>(null);

  $effect(() => {
    void app.version;
    const n = node;
    void app.run(async () => {
      const next = await app.api.issue(n);
      if (n !== node) return;
      // Keep what is being typed; take the rest.
      if (!issue || issue.node !== n || title === issue.title) title = next?.title ?? "";
      if (!issue || issue.node !== n || body === issue.body) body = next?.body ?? "";
      issue = next;
    });
  });

  $effect(() => {
    const q = search.trim();
    if (!linking || q.length < 1) {
      found = [];
      return;
    }
    const timer = setTimeout(() => {
      void app.api.search(q).then((cards) => (found = cards.filter((card) => card.node !== node)));
    }, 150);
    return () => clearTimeout(timer);
  });

  const set = (field: string, value: unknown) => app.run(() => app.api.setField(node, field, value));

  const statusDef = $derived(issue?.fieldDefs.find((f) => f.id === issue?.statusField) ?? null);
  const otherDefs = $derived(issue?.fieldDefs.filter((f) => f.id !== issue?.statusField) ?? []);

  function setOption(defId: string, optionId: string | null) {
    const def = issue?.fieldDefs.find((f) => f.id === defId);
    const option = def?.options.find((o) => o.id === optionId);
    void set(`field:${defId}`, option ? { kind: "single_select", optionId: option.id, name: option.name } : null);
  }

  function toggleLabel(id: string) {
    if (!issue) return;
    const has = issue.labelsFull.some((l) => l.id === id);
    const next = has
      ? issue.labelsFull.filter((l) => l.id !== id)
      : [...issue.labelsFull, issue.repoLabels.find((l) => l.id === id)!];
    void set("labels", next);
  }

  function toggleAssignee(id: string) {
    if (!issue) return;
    const has = issue.assigneesFull.some((u) => u.id === id);
    const next = has
      ? issue.assigneesFull.filter((u) => u.id !== id)
      : [...issue.assigneesFull, issue.repoUsers.find((u) => u.id === id)!];
    void set("assignees", next);
  }

  async function link(other: Card) {
    const kind = linking;
    linking = null;
    search = "";
    if (!issue || !kind) return;
    if (kind === "blockedBy") {
      await set("blockedBy", [...issue.blockedByList.map((l) => l.node), other.node]);
    } else if (kind === "blocking") {
      const detail = await app.api.issue(other.node);
      if (detail) {
        await app.run(() =>
          app.api.setField(other.node, "blockedBy", [...detail.blockedByList.map((l) => l.node), node]),
        );
      }
    } else {
      await set("parent", other.node);
    }
  }

  async function unlink(kind: "blockedBy" | "blocking" | "parent" | "sub", other: Linked) {
    if (!issue) return;
    if (kind === "blockedBy") {
      await set("blockedBy", issue.blockedByList.filter((l) => l.node !== other.node).map((l) => l.node));
    } else if (kind === "blocking") {
      const detail = await app.api.issue(other.node);
      if (detail) {
        await app.run(() =>
          app.api.setField(
            other.node,
            "blockedBy",
            detail.blockedByList.filter((l) => l.node !== node).map((l) => l.node),
          ),
        );
      }
    } else if (kind === "parent") {
      await set("parent", null);
    } else {
      await app.run(() => app.api.setField(other.node, "parent", null));
    }
  }

  async function send() {
    const text = reply.trim();
    if (!text) return;
    reply = "";
    await app.run(() => app.api.comment(node, text));
  }

  function date(iso: string | null): string {
    return iso ? iso.slice(0, 10) : "";
  }

  const shown = (raw: string) => {
    try {
      const v = JSON.parse(raw);
      if (typeof v === "string") return v.length > 120 ? `${v.slice(0, 120)}…` : v;
      return v?.name ?? v?.state ?? JSON.stringify(v);
    } catch {
      return raw;
    }
  };
</script>

{#snippet section(label: string)}
  <Text style={{ fontSize: 11, color: c.faint, uppercase: true, paddingTop: 6 }}>{label}</Text>
{/snippet}

{#snippet linkedRow(kind: "blockedBy" | "blocking" | "parent" | "sub", other: Linked)}
  <View style={{ direction: "row", align: "center", gap: 8, paddingY: 3 }}>
    <View style={{ width: 7, height: 7, radius: 4, background: other.state === "OPEN" ? c.ok : "#a371f7" }} />
    <View
      style={{ flex: 1, minWidth: 0 }}
      onpress={() => app.select({ kind: "issue", node: other.node })}
    >
      <Text style={{ fontSize: 12, truncate: true }}>
        <Text style={{ color: c.faint }}>{other.repoName ?? ""}#{other.number ?? "?"}</Text>
        {other.title}
      </Text>
    </View>
    <Button style={{ ...ghost, paddingX: 5 }} hover={ghostHover} onpress={() => void unlink(kind, other)}>
      <Text>×</Text>
    </Button>
  </View>
{/snippet}

<View style={{ width: 460, borderLeftWidth: 1, borderColor: c.border, background: c.surface }}>
  {#if issue}
    <View style={{ direction: "row", align: "center", gap: 8, paddingX: 14, height: 44, shrink: 0 }}>
      <View style={{ paddingX: 8, height: 20, radius: 10, justify: "center", background: issue.state === "OPEN" ? "#3fb95033" : "#a371f733" }}>
        <Text style={{ fontSize: 11, nowrap: true, color: issue.state === "OPEN" ? c.ok : "#a371f7" }}>
          {issue.state === "OPEN" ? "Open" : issue.stateReason === "NOT_PLANNED" ? "Not planned" : "Closed"}
        </Text>
      </View>
      <Text style={{ fontSize: 12, color: c.muted }}>{issue.repoName}#{issue.number ?? "new"}</Text>
      {#if issue.pending}<Text style={{ fontSize: 11, color: c.warn }}>· syncing</Text>{/if}
      <View style={{ flex: 1 }} />
      {#if issue.url}
        <Button style={ghost} hover={ghostHover} onpress={() => void app.api.open(issue!.url!)}>
          <Text>GitHub ↗</Text>
        </Button>
      {/if}
      <Button style={ghost} hover={ghostHover} onpress={() => app.select(null)}>
        <Text>✕</Text>
      </Button>
    </View>
    <Scroll style={{ flex: 1 }}>
      <View style={{ padding: [4, 14, 20, 14], gap: 8 }}>
        <Input
          bind:value={title}
          multiline
          rows={1}
          style={{ ...field, fontSize: 16, fontWeight: "semibold", background: "transparent", borderColor: "transparent", paddingX: 2 }}
          focus={{ borderColor: c.accent, background: c.bg }}
          oncommit={() => title.trim() && title !== issue?.title && void set("title", title.trim())}
        />

        {#each issue.conflicts as conflict (conflict.field)}
          <View style={{ padding: 10, gap: 6, radius: 8, borderWidth: 1, borderColor: c.danger, background: "#f2555a14" }}>
            <Text style={{ fontSize: 12, fontWeight: "semibold" }}>Changed here and on GitHub: {conflict.field}</Text>
            <Text style={{ fontSize: 12, color: c.muted }}>Yours: {shown(conflict.local)}</Text>
            <Text style={{ fontSize: 12, color: c.muted }}>GitHub's: {shown(conflict.remote)}</Text>
            <View style={{ direction: "row", gap: 6 }}>
              <Button style={button} hover={buttonHover} onpress={() => void app.run(() => app.api.resolve(node, conflict.field, true))}>
                <Text>Keep mine</Text>
              </Button>
              <Button style={button} hover={buttonHover} onpress={() => void app.run(() => app.api.resolve(node, conflict.field, false))}>
                <Text>Take GitHub's</Text>
              </Button>
            </View>
          </View>
        {/each}

        <View style={{ gap: 6, padding: 10, radius: 8, background: c.bg }}>
          {#if statusDef}
            <View style={{ direction: "row", align: "center", gap: 8 }}>
              <Text style={{ width: 84, fontSize: 12, color: c.muted }}>Status</Text>
              <Select
                value={statusDef.value?.optionId ?? null}
                options={[{ value: null, label: "No status" }, ...statusDef.options.map((o) => ({ value: o.id, label: o.name }))]}
                style={{ ...field, flex: 1, paddingY: 4 }}
                onchange={(v) => {
                  if (v === null) void set(`field:${statusDef.id}`, null);
                  else void app.run(() => app.api.moveCard(node, v as string));
                }}
              />
            </View>
          {/if}
          {#each otherDefs as def (def.id)}
            <View style={{ direction: "row", align: "center", gap: 8 }}>
              <Text style={{ width: 84, fontSize: 12, color: c.muted }}>{def.name}</Text>
              {#if def.kind === "single_select"}
                <Select
                  value={def.value?.optionId ?? null}
                  options={[{ value: null, label: "None" }, ...def.options.map((o) => ({ value: o.id, label: o.name }))]}
                  style={{ ...field, flex: 1, paddingY: 4 }}
                  onchange={(v) => setOption(def.id, v as string | null)}
                />
              {:else}
                <Input
                  value={String(def.value?.value ?? "")}
                  placeholder={def.kind === "date" ? "YYYY-MM-DD" : ""}
                  style={{ ...field, flex: 1, paddingY: 4 }}
                  focus={fieldFocus}
                  oncommit={(text) => {
                    const v = text.trim();
                    void set(`field:${def.id}`, v ? { kind: def.kind, value: def.kind === "number" ? Number(v) : v } : null);
                  }}
                />
              {/if}
            </View>
          {/each}
          <View style={{ direction: "row", align: "start", gap: 8 }}>
            <Text style={{ width: 84, fontSize: 12, color: c.muted, paddingTop: 3 }}>Labels</Text>
            <View style={{ flex: 1, direction: "row", wrap: true, gap: 4, align: "center" }}>
              {#each issue.labelsFull as label (label.id)}
                <View style={{ direction: "row", align: "center", gap: 4, paddingX: 7, height: 22, radius: 11, borderWidth: 1, borderColor: c.border }}>
                  <View style={{ width: 7, height: 7, radius: 4, background: labelColor(label.color) }} />
                  <Text style={{ fontSize: 11 }}>{label.name}</Text>
                </View>
              {/each}
              <Popover side="bottom" align="start">
                {#snippet trigger({ toggle })}
                  <Button style={{ ...ghost, paddingX: 6, paddingY: 2 }} hover={ghostHover} onpress={toggle}><Text>+ label</Text></Button>
                {/snippet}
                {#snippet children()}
                  <Scroll style={{ width: 240, maxHeight: 300, padding: 6, radius: 8, background: c.surface, borderWidth: 1, borderColor: c.border }}>
                    {#each issue!.repoLabels as label (label.id)}
                      {@const on = issue!.labelsFull.some((l) => l.id === label.id)}
                      <View style={{ direction: "row", align: "center", gap: 8, paddingX: 8, paddingY: 5, radius: 4 }} hover={{ background: c.hover }} onpress={() => toggleLabel(label.id)}>
                        <Text style={{ width: 12, fontSize: 11, color: c.accent }}>{on ? "✓" : ""}</Text>
                        <View style={{ width: 8, height: 8, radius: 4, background: labelColor(label.color) }} />
                        <Text style={{ fontSize: 12 }}>{label.name}</Text>
                      </View>
                    {/each}
                  </Scroll>
                {/snippet}
              </Popover>
            </View>
          </View>
          <View style={{ direction: "row", align: "start", gap: 8 }}>
            <Text style={{ width: 84, fontSize: 12, color: c.muted, paddingTop: 3 }}>Assignees</Text>
            <View style={{ flex: 1, direction: "row", wrap: true, gap: 4, align: "center" }}>
              {#each issue.assigneesFull as user (user.id)}
                <Text style={{ fontSize: 12, paddingX: 6 }}>@{user.login}</Text>
              {/each}
              <Popover side="bottom" align="start">
                {#snippet trigger({ toggle })}
                  <Button style={{ ...ghost, paddingX: 6, paddingY: 2 }} hover={ghostHover} onpress={toggle}><Text>+ assignee</Text></Button>
                {/snippet}
                {#snippet children()}
                  <View style={{ width: 220, padding: 6, radius: 8, background: c.surface, borderWidth: 1, borderColor: c.border }}>
                    {#each issue!.repoUsers as user (user.id)}
                      {@const on = issue!.assigneesFull.some((u) => u.id === user.id)}
                      <View style={{ direction: "row", gap: 8, paddingX: 8, paddingY: 5, radius: 4 }} hover={{ background: c.hover }} onpress={() => toggleAssignee(user.id)}>
                        <Text style={{ width: 12, fontSize: 11, color: c.accent }}>{on ? "✓" : ""}</Text>
                        <Text style={{ fontSize: 12 }}>@{user.login}</Text>
                      </View>
                    {/each}
                  </View>
                {/snippet}
              </Popover>
            </View>
          </View>
          <View style={{ direction: "row", gap: 6, paddingTop: 4 }}>
            {#if issue.state === "OPEN"}
              <Button style={button} hover={buttonHover} onpress={() => void set("state", { state: "CLOSED", reason: "COMPLETED" })}>
                <Text>Close as done</Text>
              </Button>
              <Button style={button} hover={buttonHover} onpress={() => void set("state", { state: "CLOSED", reason: "NOT_PLANNED" })}>
                <Text>Not planned</Text>
              </Button>
            {:else}
              <Button style={button} hover={buttonHover} onpress={() => void set("state", { state: "OPEN", reason: null })}>
                <Text>Reopen</Text>
              </Button>
            {/if}
          </View>
        </View>

        {@render section("Description")}
        <Input
          bind:value={body}
          multiline
          rows={8}
          placeholder="Add a description…"
          style={{ ...field, lineHeight: 19 }}
          focus={fieldFocus}
          oncommit={() => body !== issue?.body && void set("body", body)}
        />

        {@render section("Relationships")}
        {#if issue.parent}
          <Text style={{ fontSize: 11, color: c.muted }}>Parent</Text>
          {@render linkedRow("parent", issue.parent)}
        {/if}
        {#if issue.blockedByList.length > 0}
          <Text style={{ fontSize: 11, color: c.danger }}>Blocked by</Text>
          {#each issue.blockedByList as other (other.node)}{@render linkedRow("blockedBy", other)}{/each}
        {/if}
        {#if issue.blockingList.length > 0}
          <Text style={{ fontSize: 11, color: c.muted }}>Blocking</Text>
          {#each issue.blockingList as other (other.node)}{@render linkedRow("blocking", other)}{/each}
        {/if}
        {#if issue.subIssues.length > 0}
          <Text style={{ fontSize: 11, color: c.muted }}>Sub-issues</Text>
          {#each issue.subIssues as other (other.node)}{@render linkedRow("sub", other)}{/each}
        {/if}
        {#if linking}
          <View style={{ gap: 6 }}>
            <Input
              bind:value={search}
              placeholder="Search issues in every project: title or #number"
              style={field}
              focus={fieldFocus}
            />
            {#each found.slice(0, 8) as card (card.node)}
              <View style={{ paddingX: 8, paddingY: 5, radius: 4 }} hover={{ background: c.hover }} onpress={() => void link(card)}>
                <Text style={{ fontSize: 12, truncate: true }}>
                  <Text style={{ color: c.faint }}>{card.repoName}#{card.number}</Text>
                  {card.title}
                </Text>
              </View>
            {/each}
            <Button style={ghost} hover={ghostHover} onpress={() => (linking = null)}><Text>Cancel</Text></Button>
          </View>
        {:else}
          <View style={{ direction: "row", gap: 4, wrap: true }}>
            <Button style={ghost} hover={ghostHover} onpress={() => (linking = "blockedBy")}><Text>+ Blocked by</Text></Button>
            <Button style={ghost} hover={ghostHover} onpress={() => (linking = "blocking")}><Text>+ Blocking</Text></Button>
            <Button style={ghost} hover={ghostHover} onpress={() => (linking = "parent")}><Text>+ Parent</Text></Button>
          </View>
        {/if}

        {@render section(`Comments · ${issue.commentList.length}`)}
        {#each issue.commentList as comment (comment.id)}
          <View style={{ gap: 4, padding: 10, radius: 8, background: c.bg, opacity: comment.pending ? 0.7 : 1 }}>
            <Text style={{ fontSize: 11, color: c.muted }}>
              @{comment.author ?? "?"} · {comment.pending ? "sending…" : date(comment.created)}
            </Text>
            <Text style={{ fontSize: 13, lineHeight: 19 }}>{comment.body}</Text>
          </View>
        {/each}
        <Input bind:value={reply} multiline rows={3} placeholder="Write a comment…" style={field} focus={fieldFocus} />
        <Button
          style={{ ...button, alignSelf: "end", background: c.accent, borderColor: c.accent }}
          hover={{ background: "#8d9bff" }}
          disabled={!reply.trim()}
          disabledStyle={{ opacity: 0.4 }}
          onpress={() => void send()}
        >
          <Text style={{ color: "#fff" }}>Send</Text>
        </Button>
      </View>
    </Scroll>
  {:else}
    <View style={{ flex: 1, align: "center", justify: "center" }}>
      <Text style={{ color: c.muted }}>Loading…</Text>
    </View>
  {/if}
</View>
