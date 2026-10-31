<script lang="ts">
  // A selected folder's name, in a bar above the canvas like the canvas's other bars, written in
  // place. Enter, or focus leaving, renames the folder and its directory.
  import { Icon, Input, View } from "@neoworks/svelte-gpui/ui";
  import { icons } from "../../lib/icons.ts";
  import { c, radii, shadow } from "../../lib/theme.ts";

  let { name, onrename }: { name: string; onrename: (name: string) => void } = $props();

  // Starts from the folder's name; the bar is keyed by the folder, so it never needs to follow.
  // svelte-ignore state_referenced_locally
  let value = $state(name);
</script>

<View
  style={{
    direction: "row",
    align: "center",
    gap: 8,
    height: 40,
    paddingX: 12,
    radius: radii.card,
    background: c.surface,
    borderWidth: 1,
    borderColor: c.border,
    shadow,
  }}
  onpress={() => {}}
>
  <Icon svg={icons.folder} size={15} color={c.faint} />
  <Input
    bind:value
    placeholder="Folder name"
    style={{ width: 200, fontSize: 13, color: c.text, background: "transparent", padding: 0 }}
    onsubmit={() => onrename(value)}
    oncommit={() => onrename(value)}
  />
</View>
