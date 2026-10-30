import { FiberState, type Context, type Plugin } from "@neoworks/extension-system";

/**
 * Mounts the host services, then the plugins, each in the order listed, so what a plugin
 * registers into lands where the list says. A host service that fails stops the boot. A plugin
 * that fails is left out, so one broken pane does not blank the window; once all are mounted,
 * those that failed, and those still waiting for a service, are reported by rejecting.
 */
export async function boot(ctx: Context, services: Plugin[], plugins: Plugin[]): Promise<void> {
  for (const service of services) await ctx.plugin(service);
  const problems: string[] = [];
  const mounted = [];
  for (const plugin of plugins) {
    const fiber = ctx.plugin(plugin);
    mounted.push(fiber);
    await fiber.then(undefined, (error: unknown) => {
      problems.push(`plugin ${fiber.name} failed: ${String(error)}`);
    });
  }
  // A plugin may need a service one of its siblings provides, so waiting is judged at the end.
  for (const fiber of mounted) {
    if (fiber.state === FiberState.PENDING) {
      problems.push(`plugin ${fiber.name} is waiting for a service`);
    }
  }
  if (problems.length > 0) throw new Error(problems.join("\n"));
}
