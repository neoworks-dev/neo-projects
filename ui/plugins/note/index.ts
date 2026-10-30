// The note sheet: risen over the canvas while a note is selected.
import type { Context } from "@neoworks/extension-system";
import "../shell/slots.ts";
import Panel from "./Panel.svelte";

export const notePanel = {
  name: "note",
  inject: ["slots"],
  apply({ slots }: Context) {
    slots.inject("shell.main.over", (slot) => {
      slot.add({ component: Panel, order: 20 });
    });
  },
};
