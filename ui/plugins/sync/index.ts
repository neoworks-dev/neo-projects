// The sync's state in the top bar.
import type { Context } from "@neoworks/extension-system";
import "../shell/slots.ts";
import SyncState from "./SyncState.svelte";

export const sync = {
  name: "sync",
  inject: ["slots", "keymap", "app"],
  apply({ slots, keymap, app }: Context) {
    slots.inject("shell.top.right", (slot) => {
      slot.add({ component: SyncState, order: 100 });
    });
    keymap.bind("ctrl-r", () => void app.api.syncNow());
  },
};
