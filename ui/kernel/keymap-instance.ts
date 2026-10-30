// The window's one keymap: the `keymap` service binds into it and `Keys.svelte` feeds it.
import { Keymap } from "./keymap.ts";

export const keymap = new Keymap();
