// Every plugin of the window, in the order they mount. Plugins are compiled into the bundle; a
// third-party plugin would be one more entry here, built on the same services and slots.
import type { Plugin } from "@neoworks/extension-system";
import { issuePanel } from "./issue/index.ts";
import { notePanel } from "./note/index.ts";
import { overview } from "./overview/index.ts";
import { project } from "./project/index.ts";
import { shell } from "./shell/index.ts";
import { sync } from "./sync/index.ts";
import { welcome } from "./welcome/index.ts";

export const plugins: Plugin[] = [shell, welcome, overview, project, issuePanel, notePanel, sync];
