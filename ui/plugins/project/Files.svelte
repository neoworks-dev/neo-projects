<script lang="ts" module>
  /** What a row of the explorer carries when dragged: a file on disk, linked where it is. */
  export interface FileDrag {
    kind: "file";
    path: string;
    name: string;
  }

  export function isFileDrag(data: unknown): data is FileDrag {
    return typeof data === "object" && data !== null && (data as { kind?: string }).kind === "file";
  }
</script>

<script lang="ts">
  // The project's files, as Android Studio's Project pane: its folder and the folders it links,
  // as a tree that loads each folder when it is opened. Drag a file onto the canvas to place a
  // link to it there; double-press it to open it.
  import { Button, Draggable, Icon, Image, Text, Tree, View } from "@neoworks/svelte-gpui/ui";
  import { Tooltip } from "@neoworks/svelte-gpui-ui";
  import type { DirEntry, Project } from "../../lib/api.ts";
  import { fileIcon } from "../../lib/fileIcons.ts";
  import { app } from "../../lib/state.svelte.ts";
  import { c, eyebrow, radii, shadow } from "../../lib/theme.ts";
  import { icons } from "../../lib/icons.ts";
  const { close } = icons;

  let { project, onclose }: { project: Project; onclose: () => void } = $props();

  const tops = $derived<DirEntry[]>([
    { name: project.name, path: project.abs, dir: true },
    ...project.roots.map((r) => ({ name: r.name, path: r.path, dir: true })),
  ]);

  let children = $state<Record<string, DirEntry[]>>({});
  let expanded = $state(new Set<string>());
  let selected = $state<string | undefined>(undefined);

  async function load(entry: DirEntry) {
    const list = await app.run(() => app.api.listDir(entry.path));
    if (list) children = { ...children, [entry.path]: list };
  }

  // The project's own folder starts open; folders that are open load again when anything changes.
  $effect(() => {
    void app.version;
    const top = tops[0];
    if (!expanded.has(top.path)) expanded = new Set([...expanded, top.path]);
    for (const path of expanded) void load({ name: "", path, dir: true });
  });


  /** A note of the workspace opens in its sheet; anything else in its own program. */
  function activate(entry: DirEntry) {
    if (entry.dir) return;
    const root = app.workspace ? `${app.workspace}/` : null;
    if (root && entry.path.startsWith(root) && entry.name.endsWith(".md")) {
      app.select({ kind: "note", path: entry.path.slice(root.length) });
      return;
    }
    void app.run(() => app.api.open(entry.path));
  }
</script>

<View
  style={{
    width: 272,
    radius: radii.card + 2,
    background: c.surface,
    borderWidth: 1,
    borderColor: c.border,
    shadow,
    overflow: "hidden",
  }}
  onpress={() => {}}
>
  <View style={{ direction: "row", align: "center", paddingLeft: 14, paddingRight: 6, height: 40, shrink: 0 }}>
    <Text style={{ ...eyebrow, flex: 1 }}>Files</Text>
    <Tooltip text="Close  F" placement="bottom">
      <Button
        style={{ size: 28, radius: 8, align: "center", justify: "center", color: c.muted, transition: 120 }}
        hover={{ background: c.hover, color: c.text }}
        onpress={onclose}
      >
        <Icon svg={close} size={14} />
      </Button>
    </Tooltip>
  </View>
  <Tree
    items={tops}
    key={(entry) => entry.path}
    childrenOf={(entry) => children[entry.path]}
    isBranch={(entry) => entry.dir}
    bind:expanded
    bind:selected
    rowHeight={28}
    indent={14}
    onexpand={(entry) => void load(entry)}
    onactivate={activate}
    style={{ flex: 1, paddingX: 6, paddingBottom: 8 }}
    rowStyle={{ radius: 7, paddingRight: 6 }}
    selectedStyle={{ background: c.hover }}
  >
    {#snippet row(entry, info)}
      {#if entry.dir}
        <View style={{ flex: 1, minWidth: 0, direction: "row", align: "center", gap: 7 }}>
          <Image src={fileIcon(entry.name, true, info.expanded)} style={{ size: 16, shrink: 0 }} />
          <Text style={{ flex: 1, fontSize: 13, color: c.text, truncate: true, fontWeight: info.depth === 0 ? "medium" : "normal" }}>{entry.name}</Text>
        </View>
      {:else}
        <Draggable data={{ kind: "file", path: entry.path, name: entry.name }} style={{ flex: 1, minWidth: 0 }}>
          <View style={{ direction: "row", align: "center", gap: 7, height: 26 }}>
            <Image src={fileIcon(entry.name)} style={{ size: 16, shrink: 0 }} />
            <Text style={{ flex: 1, fontSize: 13, color: c.muted, truncate: true }}>{entry.name}</Text>
          </View>
        </Draggable>
      {/if}
    {/snippet}
  </Tree>
  <Text style={{ fontSize: 11, color: c.faint, padding: [8, 14, 12, 14], borderTopWidth: 1, borderColor: c.border }}>
    Drag a file onto the canvas to link it there.
  </Text>
</View>
