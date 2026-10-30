// The typed shape of the window's slots. A slot is declared on `SlotMap` by augmenting this
// module, so its name, kind and props are checked wherever it is rendered or registered into:
//
//   declare module "../../kernel/slots/types.ts" {
//     interface SlotMap {
//       "shell.left": List;
//       "chat.item": Keyed<"tool" | "text", { item: ViewItem }>;
//     }
//   }
import type { Component } from "svelte";

// Props are declared as type literals, not interfaces, so they fit a component's props.

/** One occupant. Adding another replaces it until that one goes away. */
export interface Single<P extends object = {}> {
  kind: "single";
  props: P;
}

/** Entries in `order`, all drawn. */
export interface List<P extends object = {}> {
  kind: "list";
  props: P;
}

/** One renderer per key; the slot's fallback draws the keys nobody claimed. */
export interface Keyed<K extends string, P extends object = {}> {
  kind: "keyed";
  key: K;
  props: P;
}

/** Entries in `order`; the first whose `select` accepts the props draws, else the fallback. */
export interface Chain<P extends object = {}> {
  kind: "chain";
  props: P;
}

export interface SlotMap {
  /** The window's content: the shell fills it. */
  root: Single;
}

export type SlotName = Extract<keyof SlotMap, string>;
export type Kind<N extends SlotName> = SlotMap[N] extends { kind: infer K } ? K : never;
export type Props<N extends SlotName> = SlotMap[N] extends {
  props: infer P extends Record<string, unknown>;
}
  ? P
  : never;
export type KeyOf<N extends SlotName> = SlotMap[N] extends { key: infer K extends string }
  ? K
  : never;

/** A component an entry draws, with the props its slot passes. */
export type Drawn<N extends SlotName> = Component<Props<N>>;

/** What an entry says about the slots it renders: their kinds, and what draws when empty. */
export type Declare<N extends SlotName> = Kind<N> | { kind: Kind<N>; fallback?: Drawn<N> };
export type Declares = { [N in SlotName]?: Declare<N> };

interface Base<N extends SlotName> {
  component: Drawn<N>;
  /** The child slots `component` renders. They exist while this entry is in its slot. */
  slots?: Declares;
}

interface Specs<N extends SlotName> {
  single: Base<N>;
  list: Base<N> & { order?: number };
  keyed: Base<N> & { key: KeyOf<N> };
  chain: Base<N> & { order?: number; select: (props: Props<N>) => unknown };
}

/** What registering into slot `N` takes, by the slot's kind. */
export type EntrySpec<N extends SlotName> = Specs<N>[Kind<N> & keyof Specs<N>];

/** The slot as a contributor's callback sees it, while its owner is mounted. */
export interface SlotHandle<N extends SlotName> {
  add(spec: EntrySpec<N>): () => void;
}

/** What `<Slot>` takes besides the name: the key of a keyed slot, the props its occupants get. */
export type SlotArgs<N extends SlotName> = { name: N } & (SlotMap[N] extends { key: infer K }
  ? { key: K }
  : {}) &
  ({} extends Props<N> ? { props?: Props<N> } : { props: Props<N> });
