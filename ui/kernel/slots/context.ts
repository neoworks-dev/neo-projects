// How a component reaches the services: the context of the plugin whose entry it is.
import type { Context } from "@neoworks/extension-system";
import { getContext, setContext } from "svelte";

const KEY = Symbol("plugin context");

export function provideCtx(ctx: Context | undefined): void {
  setContext(KEY, ctx);
}

/** The context of the plugin that registered the component being drawn. Call it while it initialises. */
export function useCtx(): Context {
  const ctx = getContext<Context | undefined>(KEY);
  if (!ctx) throw new Error("useCtx: this component was not drawn from a slot entry");
  return ctx;
}
