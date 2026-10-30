// The `app` service: the window's state and the host's commands, for plugins. The state lives in
// a `.svelte.ts` store (no `$state` on a Service, see the kernel README); this only hands it out.
import { Service, type Context } from "@neoworks/extension-system";
import { api } from "../../lib/api.ts";
import { app } from "../../lib/state.svelte.ts";

declare module "@neoworks/extension-system" {
  interface Context {
    app: AppService;
  }
}

export class AppService extends Service {
  constructor(ctx: Context) {
    super(ctx, "app");
  }

  get state() {
    return app;
  }

  get api() {
    return api;
  }

  /** Follows the host until the plugin that called it unloads. */
  follow(): void {
    this.ctx.effect(() => app.start(), "app.follow");
  }
}
