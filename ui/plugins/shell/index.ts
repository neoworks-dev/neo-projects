// The shell owns the root slot and declares the slots the panes register into. It is the only
// plugin that knows how they are laid out.
import type { Context } from "@neoworks/extension-system";
import Empty from "./Empty.svelte";
import Shell from "./Shell.svelte";
import "./slots.ts";

export const shell = {
  name: "shell",
  inject: ["slots", "app"],
  apply({ slots, app }: Context) {
    app.follow();
    slots.add("root", {
      component: Shell,
      slots: {
        "shell.top": "list",
        "shell.top.right": "list",
        "shell.rail": "list",
        "shell.status": "list",
        "shell.status.right": "list",
        "shell.main": { kind: "chain", fallback: Empty },
        "shell.main.over": "list",
        "shell.panel": "chain",
        "shell.overlay": "list",
      },
    });
  },
};
