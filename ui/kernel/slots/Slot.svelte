<script lang="ts" generics="N extends SlotName">
  // Draws what is registered into the slot `name`, and again whenever that changes. `props` go to
  // every occupant, `key` picks the renderer in a keyed slot. See the kernel README.
  import { createSubscriber } from "svelte/reactivity";
  import { slotRegistry } from "./instance.ts";
  import Occupant from "./Occupant.svelte";
  import type { SlotArgs, SlotName } from "./types.ts";

  const args: SlotArgs<N> = $props();

  const subscribe = createSubscriber((update) => slotRegistry.subscribe(update));
  const occupants = $derived.by(() => {
    subscribe();
    const { name, key } = args as { name: string; key?: string };
    return slotRegistry.resolve(name, key, args.props);
  });
</script>

{#each occupants as occupant (occupant.id)}
  <Occupant {occupant} props={args.props} />
{/each}
