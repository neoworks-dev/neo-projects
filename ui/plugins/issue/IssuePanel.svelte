<script lang="ts">
  // One issue, in the panel at the right. Everything is edited where it is shown and goes to
  // GitHub as you (through the outbox), showing at once: the title and description as text you
  // click into, each property as a row whose value opens a picker, then relationships and the
  // conversation, with the comment box kept at the bottom.
  import { Button, Icon, Input, Popover, Scroll, Text, View } from "@neoworks/svelte-gpui/ui";
  import { Tooltip } from "@neoworks/svelte-gpui-ui";
  import arrowUpRight from "@phosphor-icons/core/regular/arrow-square-out.svg";
  import reopen from "@phosphor-icons/core/regular/arrow-counter-clockwise.svg";
  import calendar from "@phosphor-icons/core/regular/calendar-blank.svg";
  import checkCircle from "@phosphor-icons/core/regular/check-circle.svg";
  import circleDashed from "@phosphor-icons/core/regular/circle-dashed.svg";
  import circleHalf from "@phosphor-icons/core/regular/circle-half.svg";
  import hash from "@phosphor-icons/core/regular/hash.svg";
  import lightning from "@phosphor-icons/core/regular/lightning.svg";
  import send from "@phosphor-icons/core/regular/paper-plane-right.svg";
  import plus from "@phosphor-icons/core/regular/plus.svg";
  import signal from "@phosphor-icons/core/regular/cell-signal-full.svg";
  import tag from "@phosphor-icons/core/regular/tag.svg";
  import textIcon from "@phosphor-icons/core/regular/text-t.svg";
  import users from "@phosphor-icons/core/regular/users.svg";
  import close from "@phosphor-icons/core/regular/x.svg";
  import notPlanned from "@phosphor-icons/core/regular/x-circle.svg";
  import type { Card, IssueDetail, Linked } from "../../lib/api.ts";
  import Picker from "../../lib/Picker.svelte";
  import { app } from "../../lib/state.svelte.ts";
  import { button, buttonHover, c, eyebrow, labelColor, radii, shadow, tone, tones } from "../../lib/theme.ts";

  let { node }: { node: string } = $props();

  let issue = $state<IssueDetail | null>(null);
  let title = $state("");
  let body = $state("");
  let reply = $state("");
  let search = $state("");
  let found = $state<Card[]>([]);
  let linking = $state<"blockedBy" | "blocking" | "parent" | null>(null);
  let searchField = $state<Input>();

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

  $effect(() => {
    if (linking) searchField?.focus();
  });

  const set = (field: string, value: unknown) => app.run(() => app.api.setField(node, field, value));

  const statusDef = $derived(issue?.fieldDefs.find((f) => f.id === issue?.statusField) ?? null);
  const otherDefs = $derived(issue?.fieldDefs.filter((f) => f.id !== issue?.statusField) ?? []);

  /** Priorities have no colour on GitHub; these match the cards'. */
  const priorityColor: Record<string, string> = {
    Urgent: tones.PINK,
    High: tones.RED,
    Medium: tones.YELLOW,
    Low: tones.GREEN,
  };

  function optionColor(defName: string, option: { name: string; color?: string }): string {
    if (option.color) return tone(option.color);
    if (defName === "Priority") return priorityColor[option.name] ?? tones.GRAY;
    return tones.GRAY;
  }

  function fieldIcon(name: string, kind: string): string {
    if (name === "Priority") return signal;
    if (name === "Effort") return lightning;
    if (kind === "date") return calendar;
    if (kind === "number") return hash;
    if (kind === "text") return textIcon;
    return circleHalf;
  }

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

  async function sendReply() {
    const text = reply.trim();
    if (!text) return;
    reply = "";
    await app.run(() => app.api.comment(node, text));
  }

  function ago(iso: string | null): string {
    if (!iso) return "";
    const days = Math.floor((Date.now() - Date.parse(iso)) / 86_400_000);
    if (days < 1) return "today";
    if (days === 1) return "yesterday";
    if (days < 30) return `${days} days ago`;
    return iso.slice(0, 10);
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

  const open = $derived(issue?.state === "OPEN");
  const stateLabel = $derived(open ? "Open" : issue?.stateReason === "NOT_PLANNED" ? "Not planned" : "Done");
  const stateColor = $derived(open ? tones.GREEN : issue?.stateReason === "NOT_PLANNED" ? tones.GRAY : tones.PURPLE);
  const subDone = $derived(issue?.subIssues.filter((s) => s.state !== "OPEN").length ?? 0);

  const iconButton = { size: 30, radius: 8, align: "center", justify: "center", color: c.muted, transition: 120 } as const;
  const iconHover = { background: c.hover, color: c.text };
</script>

{#snippet avatar(login: string | null, size: number)}
  {@const tint = login ? labelColor(null) : c.faint}
  <View
    style={{
      size,
      radius: size / 2,
      align: "center",
      justify: "center",
      background: c.hover,
      borderWidth: 1,
      borderColor: c.border,
      shrink: 0,
    }}
  >
    <Text style={{ fontSize: size * 0.42, fontWeight: "semibold", color: login ? c.muted : tint }}>
      {(login ?? "?").slice(0, 2).toUpperCase()}
    </Text>
  </View>
{/snippet}

{#snippet property(icon: string, label: string)}
  <View style={{ direction: "row", align: "center", gap: 8, width: 112, shrink: 0, height: 28 }}>
    <Icon svg={icon} size={15} color={c.faint} />
    <Text style={{ fontSize: 13, color: c.muted }}>{label}</Text>
  </View>
{/snippet}

{#snippet empty(text: string)}
  <Text style={{ fontSize: 13, color: c.faint }}>{text}</Text>
{/snippet}

{#snippet dot(color: string)}
  <View style={{ width: 8, height: 8, radius: 4, background: color, shrink: 0 }} />
{/snippet}

{#snippet linkedRow(kind: "blockedBy" | "blocking" | "parent" | "sub", other: Linked)}
  <View
    style={{ direction: "row", align: "center", gap: 9, paddingX: 8, height: 32, radius: 8, transition: 100 }}
    hover={{ background: c.hover }}
  >
    {@render dot(other.state === "OPEN" ? tones.GREEN : tones.PURPLE)}
    <View style={{ flex: 1, minWidth: 0, direction: "row", gap: 6 }} onpress={() => app.select({ kind: "issue", node: other.node })}>
      <Text style={{ fontSize: 12, color: c.faint, nowrap: true, fontFamily: "mono" }}>#{other.number ?? "?"}</Text>
      <Text style={{ fontSize: 13, truncate: true, flex: 1 }}>{other.title}</Text>
    </View>
    <Tooltip text="Remove" placement="left">
      <Button style={{ ...iconButton, size: 24 }} hover={iconHover} onpress={() => void unlink(kind, other)}>
        <Icon svg={close} size={12} />
      </Button>
    </Tooltip>
  </View>
{/snippet}

{#snippet group(label: string, color: string, list: Linked[], kind: "blockedBy" | "blocking" | "parent" | "sub")}
  {#if list.length > 0}
    <View style={{ gap: 2 }}>
      <Text style={{ fontSize: 12, color, paddingX: 8, paddingTop: 4 }}>{label}</Text>
      {#each list as other (other.node)}{@render linkedRow(kind, other)}{/each}
    </View>
  {/if}
{/snippet}

<View
  style={{
    width: 440,
    radius: radii.panel,
    borderWidth: 1,
    borderColor: c.borderSoft,
    background: c.surface,
    overflow: "hidden",
  }}
>
  {#if issue}
    <View style={{ direction: "row", align: "center", gap: 8, paddingLeft: 16, paddingRight: 8, height: 48, shrink: 0 }}>
      <View
        style={{
          direction: "row",
          align: "center",
          gap: 6,
          paddingX: 9,
          height: 24,
          radius: 12,
          borderWidth: 1,
          borderColor: c.border,
        }}
      >
        {@render dot(stateColor)}
        <Text style={{ fontSize: 12, color: c.text }}>{stateLabel}</Text>
      </View>
      <Text style={{ fontSize: 12, color: c.faint, fontFamily: "mono" }}>{issue.repoName}#{issue.number ?? "new"}</Text>
      {#if issue.pending}
        <View style={{ direction: "row", align: "center", gap: 5 }}>
          <View style={{ width: 6, height: 6, radius: 3, background: c.warn }} />
          <Text style={{ fontSize: 11, color: c.warn }}>Syncing</Text>
        </View>
      {/if}
      <View style={{ flex: 1 }} />
      {#if issue.url}
        <Tooltip text="Open on GitHub" placement="bottom">
          <Button style={iconButton} hover={iconHover} onpress={() => void app.api.open(issue!.url!)}>
            <Icon svg={arrowUpRight} size={16} />
          </Button>
        </Tooltip>
      {/if}
      <Tooltip text="Close  Esc" placement="bottom">
        <Button style={iconButton} hover={iconHover} onpress={() => app.select(null)}>
          <Icon svg={close} size={16} />
        </Button>
      </Tooltip>
    </View>

    <Scroll style={{ flex: 1 }}>
      <View style={{ paddingX: 12, paddingBottom: 24, gap: 18 }}>
        <View style={{ gap: 6 }}>
          <Input
            bind:value={title}
            multiline
            rows={1}
            style={{ fontSize: 21, lineHeight: 28, color: c.text, paddingX: 6, paddingY: 4, radius: 8, background: "transparent", borderWidth: 1, borderColor: "transparent" }}
            hover={{ background: c.raised }}
            focus={{ background: c.bg, borderColor: c.accent }}
            oncommit={() => title.trim() && title !== issue?.title && void set("title", title.trim())}
          />
          <Text style={{ fontSize: 12, color: c.faint, paddingX: 7 }}>
            {issue.author ? `@${issue.author} opened this ${ago(issue.created)}` : `Opened ${ago(issue.created)}`}
          </Text>
        </View>

        {#each issue.conflicts as conflict (conflict.field)}
          <View style={{ padding: 12, gap: 8, radius: radii.card, borderWidth: 1, borderColor: c.danger, background: c.dangerSoft }}>
            <Text style={{ fontSize: 13, fontWeight: "medium" }}>Changed here and on GitHub: {conflict.field}</Text>
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

        <View style={{ gap: 2, paddingX: 6 }}>
          {#if statusDef}
            <View style={{ direction: "row", align: "center" }}>
              {@render property(circleDashed, "Status")}
              <Picker
                options={statusDef.options.map((o) => ({ id: o.id, label: o.name, color: optionColor("Status", o) }))}
                chosen={statusDef.value?.optionId ? [statusDef.value.optionId] : []}
                noneLabel="No status"
                onpick={(id) => {
                  if (id === null) void set(`field:${statusDef.id}`, null);
                  else void app.run(() => app.api.moveCard(node, id));
                }}
              >
                {#if statusDef.value?.name}
                  {@const option = statusDef.options.find((o) => o.id === statusDef.value?.optionId)}
                  {@render dot(option ? optionColor("Status", option) : tones.GRAY)}
                  <Text style={{ fontSize: 13 }}>{statusDef.value.name}</Text>
                {:else}
                  {@render empty("Set status")}
                {/if}
              </Picker>
            </View>
          {/if}
          {#each otherDefs as def (def.id)}
            <View style={{ direction: "row", align: "center" }}>
              {@render property(fieldIcon(def.name, def.kind), def.name)}
              {#if def.kind === "single_select"}
                <Picker
                  options={def.options.map((o) => ({ id: o.id, label: o.name, color: optionColor(def.name, o) }))}
                  chosen={def.value?.optionId ? [def.value.optionId] : []}
                  noneLabel="None"
                  onpick={(id) => setOption(def.id, id)}
                >
                  {#if def.value?.name}
                    {@const option = def.options.find((o) => o.id === def.value?.optionId)}
                    {@render dot(option ? optionColor(def.name, option) : tones.GRAY)}
                    <Text style={{ fontSize: 13 }}>{def.value.name}</Text>
                  {:else}
                    {@render empty(`Set ${def.name.toLowerCase()}`)}
                  {/if}
                </Picker>
              {:else}
                <Input
                  value={String(def.value?.value ?? "")}
                  placeholder={def.kind === "date" ? `Set ${def.name.toLowerCase()} · YYYY-MM-DD` : `Set ${def.name.toLowerCase()}`}
                  style={{ flex: 1, fontSize: 13, color: c.text, paddingX: 8, height: 28, radius: 7, background: "transparent", borderWidth: 1, borderColor: "transparent" }}
                  hover={{ background: c.hover }}
                  focus={{ background: c.bg, borderColor: c.accent }}
                  oncommit={(text) => {
                    const v = text.trim();
                    void set(`field:${def.id}`, v ? { kind: def.kind, value: def.kind === "number" ? Number(v) : v } : null);
                  }}
                />
              {/if}
            </View>
          {/each}
          <View style={{ direction: "row", align: "start" }}>
            {@render property(tag, "Labels")}
            <View style={{ flex: 1, direction: "row", wrap: true, gap: 4, align: "center", paddingTop: 2 }}>
              <Picker
                options={issue.repoLabels.map((l) => ({ id: l.id, label: l.name, color: labelColor(l.color) }))}
                chosen={issue.labelsFull.map((l) => l.id)}
                multiple
                placeholder="Filter labels…"
                onpick={(id) => id && toggleLabel(id)}
              >
                {#if issue.labelsFull.length === 0}
                  {@render empty("Add label")}
                {:else}
                  <View style={{ direction: "row", wrap: true, gap: 4 }}>
                    {#each issue.labelsFull as label (label.id)}
                      <View style={{ direction: "row", align: "center", gap: 5, paddingX: 8, height: 22, radius: 11, borderWidth: 1, borderColor: c.border }}>
                        {@render dot(labelColor(label.color))}
                        <Text style={{ fontSize: 12 }}>{label.name}</Text>
                      </View>
                    {/each}
                  </View>
                {/if}
              </Picker>
            </View>
          </View>
          <View style={{ direction: "row", align: "start" }}>
            {@render property(users, "Assignees")}
            <View style={{ flex: 1, paddingTop: 2 }}>
              <Picker
                options={issue.repoUsers.map((u) => ({ id: u.id, label: `@${u.login}` }))}
                chosen={issue.assigneesFull.map((u) => u.id)}
                multiple
                placeholder="Filter people…"
                onpick={(id) => id && toggleAssignee(id)}
              >
                {#if issue.assigneesFull.length === 0}
                  {@render empty("Add assignee")}
                {:else}
                  <View style={{ direction: "row", wrap: true, gap: 8 }}>
                    {#each issue.assigneesFull as user (user.id)}
                      <View style={{ direction: "row", align: "center", gap: 6 }}>
                        {@render avatar(user.login, 20)}
                        <Text style={{ fontSize: 13 }}>{user.login}</Text>
                      </View>
                    {/each}
                  </View>
                {/if}
              </Picker>
            </View>
          </View>
        </View>

        <View style={{ gap: 6 }}>
          <Text style={{ ...eyebrow, paddingX: 7 }}>Description</Text>
          <Input
            bind:value={body}
            multiline
            rows={4}
            placeholder="Add a description…"
            style={{ fontSize: 14, lineHeight: 22, color: c.text, paddingX: 6, paddingY: 6, radius: 8, background: "transparent", borderWidth: 1, borderColor: "transparent" }}
            hover={{ background: c.raised }}
            focus={{ background: c.bg, borderColor: c.accent }}
            oncommit={() => body !== issue?.body && void set("body", body)}
          />
        </View>

        <View style={{ gap: 4 }}>
          <View style={{ direction: "row", align: "center", paddingX: 7, height: 26 }}>
            <Text style={{ ...eyebrow, flex: 1 }}>Relationships</Text>
            {#if issue.subIssues.length > 0}
              <Text style={{ fontSize: 12, color: c.faint, paddingRight: 6 }}>{subDone}/{issue.subIssues.length} sub-issues done</Text>
            {/if}
            <Popover side="bottom" align="end" style={{ width: 200, padding: 5, radius: radii.card, background: c.surface, borderWidth: 1, borderColor: c.border, shadow }}>
              {#snippet trigger({ toggle })}
                <Tooltip text="Add a relationship" placement="left">
                  <Button style={{ ...iconButton, size: 26 }} hover={iconHover} onpress={toggle}>
                    <Icon svg={plus} size={14} />
                  </Button>
                </Tooltip>
              {/snippet}
              {#snippet children({ close: done })}
                {#each [["blockedBy", "Blocked by…"], ["blocking", "Blocking…"], ["parent", "Parent…"]] as const as [kind, label] (kind)}
                  <Button
                    style={{ paddingX: 9, height: 30, radius: 7, transition: 100 }}
                    hover={{ background: c.hover }}
                    onpress={() => {
                      linking = kind;
                      done("select");
                    }}
                  >
                    <Text style={{ fontSize: 13 }}>{label}</Text>
                  </Button>
                {/each}
              {/snippet}
            </Popover>
          </View>
          {#if issue.parent}{@render group("Parent", c.muted, [issue.parent], "parent")}{/if}
          {@render group("Blocked by", c.danger, issue.blockedByList, "blockedBy")}
          {@render group("Blocking", c.muted, issue.blockingList, "blocking")}
          {@render group("Sub-issues", c.muted, issue.subIssues, "sub")}
          {#if linking}
            <View style={{ gap: 4, padding: 6, radius: radii.card, background: c.raised }}>
              <Input
                bind:this={searchField}
                bind:value={search}
                placeholder={linking === "blockedBy" ? "Blocked by… title or #number" : linking === "blocking" ? "Blocking… title or #number" : "Parent… title or #number"}
                style={{ paddingX: 9, height: 32, fontSize: 13, color: c.text, radius: 7, background: c.bg, borderWidth: 1, borderColor: c.accent }}
                onkey={(e) => {
                  if (e.key === "Escape") linking = null;
                }}
                onsubmit={() => {
                  if (found[0]) void link(found[0]);
                }}
              />
              {#each found.slice(0, 8) as card (card.node)}
                <Button style={{ direction: "row", gap: 6, paddingX: 9, height: 30, radius: 7 }} hover={{ background: c.hover }} onpress={() => void link(card)}>
                  <Text style={{ fontSize: 12, color: c.faint, fontFamily: "mono" }}>#{card.number}</Text>
                  <Text style={{ fontSize: 13, flex: 1, truncate: true }}>{card.title}</Text>
                  <Text style={{ fontSize: 11, color: c.faint }}>{card.repoName}</Text>
                </Button>
              {/each}
            </View>
          {:else if !issue.parent && issue.blockedByList.length + issue.blockingList.length + issue.subIssues.length === 0}
            <Text style={{ fontSize: 13, color: c.faint, paddingX: 7 }}>Nothing blocks this or depends on it.</Text>
          {/if}
        </View>

        <View style={{ gap: 10 }}>
          <Text style={{ ...eyebrow, paddingX: 7 }}>Activity · {issue.commentList.length}</Text>
          {#each issue.commentList as comment (comment.id)}
            <View style={{ direction: "row", gap: 10, paddingX: 6, opacity: comment.pending ? 0.6 : 1 }}>
              {@render avatar(comment.author, 26)}
              <View style={{ flex: 1, minWidth: 0, gap: 3 }}>
                <View style={{ direction: "row", gap: 6, align: "center" }}>
                  <Text style={{ fontSize: 13, fontWeight: "medium" }}>{comment.author ?? "?"}</Text>
                  <Text style={{ fontSize: 12, color: c.faint }}>{comment.pending ? "sending…" : ago(comment.created)}</Text>
                </View>
                <Text style={{ fontSize: 14, lineHeight: 21, color: c.text }}>{comment.body}</Text>
              </View>
            </View>
          {/each}
        </View>

        <View style={{ direction: "row", gap: 6, paddingX: 6 }}>
          {#if open}
            <Button style={button} hover={buttonHover} onpress={() => void set("state", { state: "CLOSED", reason: "COMPLETED" })}>
              <Icon svg={checkCircle} size={15} color={tones.PURPLE} />
              <Text>Close as done</Text>
            </Button>
            <Button style={button} hover={buttonHover} onpress={() => void set("state", { state: "CLOSED", reason: "NOT_PLANNED" })}>
              <Icon svg={notPlanned} size={15} color={c.muted} />
              <Text>Not planned</Text>
            </Button>
          {:else}
            <Button style={button} hover={buttonHover} onpress={() => void set("state", { state: "OPEN", reason: null })}>
              <Icon svg={reopen} size={15} color={tones.GREEN} />
              <Text>Reopen</Text>
            </Button>
          {/if}
        </View>
      </View>
    </Scroll>

    <View style={{ padding: 10, borderTopWidth: 1, borderColor: c.border, shrink: 0 }}>
      <View
        style={{ direction: "row", align: "end", gap: 6, padding: 4, paddingLeft: 10, radius: radii.card, background: c.raised, borderWidth: 1, borderColor: c.border }}
      >
        <Input
          bind:value={reply}
          multiline
          rows={1}
          placeholder="Leave a comment…  Ctrl+Enter sends"
          style={{ flex: 1, fontSize: 14, lineHeight: 20, color: c.text, paddingY: 5, background: "transparent" }}
          onkey={(e) => {
            if (e.key === "Enter" && (e.ctrl || e.meta)) void sendReply();
          }}
        />
        <Button
          style={{ size: 30, radius: 9, align: "center", justify: "center", background: reply.trim() ? c.fill : "transparent", color: reply.trim() ? c.onFill : c.faint, transition: 140 }}
          hover={{ background: reply.trim() ? c.fillHover : c.hover }}
          onpress={() => void sendReply()}
        >
          <Icon svg={send} size={15} />
        </Button>
      </View>
    </View>
  {:else}
    <View style={{ flex: 1, align: "center", justify: "center" }}>
      <Text style={{ color: c.faint }}>Loading…</Text>
    </View>
  {/if}
</View>
