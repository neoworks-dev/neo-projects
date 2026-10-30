<script lang="ts">
  // The window half of the keymap: every key that reaches the window, because the focused input or
  // panel did not use it, goes to the bindings.
  import { keymap } from "./keymap-instance.ts";
</script>

<svelte:window onkeydown={(event) => keymap.dispatch(event)} />
