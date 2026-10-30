// The welcome screen fills the window until a workspace is open.
import type { Context } from "@neoworks/extension-system";
import "../shell/slots.ts";
import Welcome from "./Welcome.svelte";

export const welcome = {
  name: "welcome",
  inject: ["slots", "app"],
  apply({ slots, app }: Context) {
    slots.inject("shell.main", (slot) => {
      slot.add({ component: Welcome, order: 0, select: () => app.state.ready && !app.state.workspace });
    });
  },
};
