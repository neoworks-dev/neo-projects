<script lang="ts">
  // The window is whatever the plugins register into the root slot (the shell).
  import Keys from "./kernel/Keys.svelte";
  import Slot from "./kernel/slots/Slot.svelte";
</script>

<Keys />
<Slot name="root" />
