<script lang="ts">
  // The window half of the keymap: every key goes to the bindings. The window hears a key before
  // the focused field types it, so the keymap leaves out keys being typed into a field.
  import { keymap } from "./keymap-instance.ts";
</script>

<svelte:window onkeydown={(event) => keymap.dispatch(event)} />
