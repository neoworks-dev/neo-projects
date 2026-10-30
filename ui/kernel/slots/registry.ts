// What is in each slot, as plain data. The `slots` service types and tracks it, `<Slot>` reads
// it. Kept free of Svelte so it runs in tests; `subscribe` is how a component hears of changes.
//
// A slot exists while the entry that declared it (or the registry, for `root`) is in its own
// slot. Registering into a slot that does not exist fails; when its owner leaves, the slot and
// everything in it go too, and what contributors put there is put again if the owner comes back.
import type { Component } from "svelte";

export type Dispose = () => void;
type Drawn = Component<never>;

interface Declared {
  kind: string;
  fallback?: Drawn;
}

/** What an entry is registered with, as the registry sees it: the typed API narrows this. */
export interface Spec {
  component: Drawn;
  slots?: Record<string, string | Declared>;
  order?: number;
  key?: string;
  select?: (props: never) => unknown;
  /** The context of the plugin that registered it, for the component to reach services with. */
  ctx?: unknown;
}

interface Entry {
  id: number;
  spec: Spec;
  slot: Slot;
  /** Whether the slots it declares exist. */
  active: boolean;
}

interface Slot extends Declared {
  name: string;
  entries: Entry[];
  /** What each contributor put here, to take away again. */
  applied: Map<Contributor, Dispose>;
}

type Contributor = (handle: { add(spec: Spec): Dispose }) => Dispose | void;

/** What `<Slot>` draws: a component and the context to draw it in. */
export interface Occupant {
  id: string;
  component: Drawn;
  ctx?: unknown;
}

export class SlotRegistry {
  private readonly slots = new Map<string, Slot>();
  private readonly contributors = new Map<string, Set<Contributor>>();
  private readonly listeners = new Set<() => void>();
  private ids = 0;

  /**
   * `onError` hears what fails where nobody is waiting for the result: a teardown step (the
   * others still run) and a contributor that throws once the slot's owner is already mounting.
   */
  constructor(private readonly onError: (error: Error) => void) {
    this.declare("root", { kind: "single" });
  }

  /** Whether `name` is declared right now. */
  has(name: string): boolean {
    return this.slots.has(name);
  }

  /** Calls `listener` whenever a slot's occupants change. Returns the call that stops it. */
  subscribe(listener: () => void): Dispose {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  /** Registers into a declared slot; throws for one that is not. Returns the call that undoes it. */
  add(name: string, spec: Spec): Dispose {
    const slot = this.slots.get(name);
    if (!slot) {
      throw new Error(
        `slot "${name}" is not declared: only a slot declared by a mounted entry takes entries`,
      );
    }
    return this.addTo(slot, spec);
  }

  /**
   * Runs `contribute` with the slot every time it is declared, now if it is, and undoes what it
   * did when the slot goes away or the returned call is made. This is how a plugin that may mount
   * before the slot's owner (or outlive it) registers.
   */
  contribute(name: string, contribute: Contributor): Dispose {
    let contributors = this.contributors.get(name);
    if (!contributors) this.contributors.set(name, (contributors = new Set()));
    contributors.add(contribute);
    const slot = this.slots.get(name);
    if (slot) {
      try {
        this.apply(slot, contribute, true);
      } catch (error) {
        contributors.delete(contribute);
        throw error;
      }
    }
    return () => {
      contributors.delete(contribute);
      const current = this.slots.get(name);
      const applied = current?.applied.get(contribute);
      current?.applied.delete(contribute);
      this.settle(applied);
    };
  }

  /** What `<Slot>` draws for `name`. `key` picks in a keyed slot, `props` feed a chain's `select`. */
  resolve(name: string, key?: string, props?: unknown): Occupant[] {
    const slot = this.slots.get(name);
    if (!slot) return [];
    const found = this.pick(slot, key, props);
    if (found.length === 0 && slot.fallback) {
      return [{ id: "fallback", component: slot.fallback }];
    }
    return found.map(({ id, spec }) => ({
      id: String(id),
      component: spec.component,
      ctx: spec.ctx,
    }));
  }

  private pick(slot: Slot, key: string | undefined, props: unknown): Entry[] {
    switch (slot.kind) {
      case "single":
        return slot.entries.slice(-1);
      case "keyed":
        return slot.entries.filter((entry) => entry.spec.key === key).slice(-1);
      case "chain":
        return slot.entries.filter((entry) => entry.spec.select?.(props as never)).slice(0, 1);
      default:
        return slot.entries;
    }
  }

  private addTo(slot: Slot, spec: Spec): Dispose {
    if (this.slots.get(slot.name) !== slot) throw new Error(`slot "${slot.name}" is gone`);
    if (slot.kind === "keyed" && spec.key === undefined) {
      throw new Error(`slot "${slot.name}" is keyed: its entries need a key`);
    }
    if (slot.kind === "chain" && !spec.select) {
      throw new Error(`slot "${slot.name}" is a chain: its entries need a select`);
    }
    const entry: Entry = { id: (this.ids += 1), spec, slot, active: false };
    // Ordered kinds go after the last entry that is not later; a tie keeps the order of adding.
    const ordered = slot.kind === "list" || slot.kind === "chain";
    let at = slot.entries.length;
    while (ordered && at > 0 && (slot.entries[at - 1]?.spec.order ?? 0) > (spec.order ?? 0))
      at -= 1;
    slot.entries.splice(at, 0, entry);
    try {
      this.reconcile(slot);
    } catch (error) {
      slot.entries.splice(slot.entries.indexOf(entry), 1);
      this.reconcile(slot);
      throw error;
    }
    this.notify();
    return () => this.remove(entry);
  }

  private remove(entry: Entry): void {
    const { slot } = entry;
    // A slot that went away took its entries with it.
    if (this.slots.get(slot.name) !== slot) return;
    const at = slot.entries.indexOf(entry);
    if (at < 0) return;
    slot.entries.splice(at, 1);
    if (entry.active) this.deactivate(entry);
    this.reconcile(slot);
    this.notify();
  }

  /**
   * Makes the slots of the entries that draw exist, and those of the others not. In a `single`
   * only the last entry draws, so replacing it frees the slots the old one declared first. In
   * the other kinds every entry is mounted, whether or not it draws just now.
   */
  private reconcile(slot: Slot): void {
    const mounted = slot.kind === "single" ? slot.entries.slice(-1) : slot.entries;
    for (const entry of slot.entries) {
      if (entry.active && !mounted.includes(entry)) this.deactivate(entry);
    }
    for (const entry of mounted) {
      if (!entry.active) this.activate(entry);
    }
  }

  private activate(entry: Entry): void {
    const declared: string[] = [];
    try {
      for (const [name, declare] of Object.entries(entry.spec.slots ?? {})) {
        this.declare(name, typeof declare === "string" ? { kind: declare } : declare);
        declared.push(name);
      }
    } catch (error) {
      for (const name of declared) this.undeclare(name);
      throw error;
    }
    entry.active = true;
  }

  private deactivate(entry: Entry): void {
    entry.active = false;
    for (const name of Object.keys(entry.spec.slots ?? {})) this.undeclare(name);
  }

  private declare(name: string, declared: Declared): void {
    if (this.slots.has(name)) throw new Error(`slot "${name}" is already declared`);
    const slot: Slot = { ...declared, name, entries: [], applied: new Map() };
    this.slots.set(name, slot);
    for (const contribute of this.contributors.get(name) ?? []) {
      this.apply(slot, contribute, false);
    }
  }

  private undeclare(name: string): void {
    const slot = this.slots.get(name);
    if (!slot) return;
    this.slots.delete(name);
    for (const dispose of slot.applied.values()) this.settle(dispose);
    for (const entry of slot.entries) {
      if (entry.active) this.deactivate(entry);
    }
    slot.entries = [];
  }

  /** Runs one contributor against a slot and keeps what it takes to undo that. */
  private apply(slot: Slot, contribute: Contributor, rethrow: boolean): void {
    const added: Dispose[] = [];
    const undo = (): void => {
      for (const dispose of added.splice(0).reverse()) this.settle(dispose);
    };
    try {
      const own = contribute({
        add: (spec) => {
          const dispose = this.addTo(slot, spec);
          added.push(dispose);
          return dispose;
        },
      });
      slot.applied.set(contribute, () => {
        undo();
        this.settle(own);
      });
    } catch (error) {
      undo();
      // Its owner is mounting; what a contributor does wrong is not the owner's failure.
      if (rethrow) throw error;
      this.onError(new Error(`contributing to slot "${slot.name}" failed`, { cause: error }));
    }
  }

  /** Runs a teardown step so that a failing one does not keep the others from running. */
  private settle(dispose: Dispose | void): void {
    try {
      dispose?.();
    } catch (error) {
      this.onError(new Error("slot teardown failed", { cause: error }));
    }
  }

  private notify(): void {
    for (const listener of [...this.listeners]) listener();
  }
}
