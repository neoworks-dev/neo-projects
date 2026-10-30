// The `keymap` service: the window's keys. A plugin binds a chord (`ctrl-s`, `shift-tab`,
// `escape`, `f6`) to a handler; unloading the plugin unbinds it. See keymap.ts for what a
// handler returns.
import { Service, type Context } from "@neoworks/extension-system";
import { keymap } from "../keymap-instance.ts";
import type { BindOptions, KeyHandler } from "../keymap.ts";

declare module "@neoworks/extension-system" {
  interface Context {
    keymap: KeymapService;
  }
}

export class KeymapService extends Service {
  constructor(ctx: Context) {
    super(ctx, "keymap");
  }

  /** Binds a chord; the newest binding of a chord gets the key first, unless another has a higher `priority`. */
  bind(chord: string, run: KeyHandler, options?: BindOptions): () => void {
    const dispose = this.ctx.effect(
      () => keymap.bind(chord, run, options),
      `keymap.bind(${chord})`,
    );
    return () => void dispose();
  }

  /** Hears every key press before the bindings, without using it. */
  observe(run: KeyHandler): () => void {
    const dispose = this.ctx.effect(() => keymap.observe(run), "keymap.observe");
    return () => void dispose();
  }
}
