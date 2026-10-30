// A project's canvas with its board. Fills the window while a project is open.
import type { Context } from "@neoworks/extension-system";
import "../shell/slots.ts";
import { projectKeys } from "./keys.svelte.ts";
import Main from "./Main.svelte";

export const project = {
  name: "project",
  inject: ["slots", "keymap", "keyboard", "app"],
  apply({ slots, keymap, app }: Context) {
    slots.inject("shell.main", (slot) => {
      slot.add({ component: Main, order: 20, select: () => app.state.workspace && app.state.route.view === "project" });
    });
    const remove = () => (app.state.route.view === "project" && projectKeys.remove?.()) || false;
    keymap.bind("delete", remove);
    keymap.bind("backspace", remove);
    // Letters reach the keymap only when no field has focus, so these never eat typing.
    keymap.bind("n", () => {
      if (app.state.route.view !== "project" || !projectKeys.note || app.state.selection?.kind === "note") return false;
      projectKeys.note();
    });
    keymap.bind("l", () => {
      if (app.state.route.view !== "project" || !projectKeys.link || app.state.selection?.kind === "note") return false;
      projectKeys.link();
    });
    keymap.bind("s", () => {
      if (app.state.route.view !== "project" || !projectKeys.sticky || app.state.selection?.kind === "note") return false;
      projectKeys.sticky();
    });
    keymap.bind("f", () => {
      if (app.state.route.view !== "project" || !projectKeys.files || app.state.selection?.kind === "note") return false;
      projectKeys.files();
    });
    keymap.bind("escape", () => {
      if (app.state.selection) {
        app.state.select(null);
        return true;
      }
      if (app.state.route.view === "project") {
        app.state.open({ view: "overview" });
        return true;
      }
      return false;
    });
  },
};
