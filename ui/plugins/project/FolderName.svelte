<script lang="ts">
  // A selected folder's name, renamed (with its directory) on Enter or when focus leaves.
  import { TextField } from "@neoworks/svelte-gpui-ui";

  let { name, onrename }: { name: string; onrename: (name: string) => void } = $props();

  // Starts from the folder's name; the view is keyed by the folder, so it never needs to follow.
  // svelte-ignore state_referenced_locally
  let value = $state(name);
</script>

<TextField
  bind:value
  label="Folder name"
  variant="outlined"
  style={{ width: 260 }}
  onsubmit={() => onrename(value)}
  onblur={() => onrename(value)}
/>
