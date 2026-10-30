// The note panel: at the right while a note is selected.
import type { Context } from "@neoworks/extension-system";
import "../shell/slots.ts";
import Panel from "./Panel.svelte";

export const notePanel = {
  name: "note",
  inject: ["slots", "app"],
  apply({ slots, app }: Context) {
    slots.inject("shell.panel", (slot) => {
      slot.add({ component: Panel, order: 20, select: () => app.state.selection?.kind === "note" });
    });
  },
};
