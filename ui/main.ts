// The library must be imported before anything that pulls in Svelte's client runtime: it
// installs the DOM globals that runtime reads at module scope.
import { start } from "@neoworks/svelte-gpui";
import App from "./App.svelte";
import { boot } from "./kernel/boot.ts";
import { root } from "./kernel/context.ts";
import { services } from "./kernel/services.ts";
import { plugins } from "./plugins/index.ts";

// The window opens once the plugins are mounted, and also when some failed: the host reports the
// rejection, and the panes that did load are still of use.
void boot(root, services, plugins).finally(() =>
  start(
    App,
    {},
    { window: { title: "Neoworks Projects", width: 1400, height: 900, minWidth: 720, minHeight: 480 } },
  ),
);
