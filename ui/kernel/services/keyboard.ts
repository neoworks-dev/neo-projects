// The `keyboard` service: who has the keyboard. The input takes it unless a pane claims it (the
// sidebar, the agents list, the GitHub view), because svelte-gpui can't hand focus back yet
// (neoworks-dev/svelte-gpui#41): the input asks `taken` before it grabs focus.
import { Service, type Context } from "@neoworks/extension-system";
import { Claims } from "../claims.svelte.ts";

declare module "@neoworks/extension-system" {
  interface Context {
    keyboard: KeyboardService;
  }
}

const claims = new Claims();

export class KeyboardService extends Service {
  constructor(ctx: Context) {
    super(ctx, "keyboard");
  }

  /** Whether a pane has the keyboard now; reactive. */
  get taken(): boolean {
    return claims.held;
  }

  /** The pane has the keyboard while `held` says so (it may read reactive state). */
  claim(held: () => boolean): () => void {
    const dispose = this.ctx.effect(() => claims.add(held), "keyboard.claim");
    return () => void dispose();
  }
}
