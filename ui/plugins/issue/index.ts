// The issue panel: at the right while an issue is selected.
import type { Context } from "@neoworks/extension-system";
import "../shell/slots.ts";
import Panel from "./Panel.svelte";

export const issuePanel = {
  name: "issue",
  inject: ["slots", "app"],
  apply({ slots, app }: Context) {
    slots.inject("shell.panel", (slot) => {
      slot.add({ component: Panel, order: 10, select: () => app.state.selection?.kind === "issue" });
    });
  },
};
