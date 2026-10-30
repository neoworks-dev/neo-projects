// The root every service and plugin of the window mounts onto. Anything a plugin does through its
// own `ctx` is undone when it unloads, so this module is only imported by the boot and the
// components that need to reach a service from outside one.
import { Context } from "@neoworks/extension-system";

export const root = new Context();
