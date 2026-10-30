// The overview: every project on one canvas. It fills the window while no project is open, and
// owns the breadcrumbs and the dialog that adds projects.
import type { Context } from "@neoworks/extension-system";
import "../shell/slots.ts";
import AddProjects from "./AddProjects.svelte";
import Crumbs from "./Crumbs.svelte";
import Overview from "./Overview.svelte";
import Rail from "./Rail.svelte";
import Status from "./Status.svelte";

export const overview = {
  name: "overview",
  inject: ["slots", "app"],
  apply({ slots, app }: Context) {
    slots.inject("shell.main", (slot) => {
      slot.add({ component: Overview, order: 10, select: () => app.state.workspace && app.state.route.view === "overview" });
    });
    slots.inject("shell.top", (slot) => {
      slot.add({ component: Crumbs, order: 0 });
    });
    slots.inject("shell.rail", (slot) => {
      slot.add({ component: Rail, order: 0 });
    });
    slots.inject("shell.status", (slot) => {
      slot.add({ component: Status, order: 0 });
    });
    slots.inject("shell.overlay", (slot) => {
      slot.add({ component: AddProjects });
    });
  },
};
