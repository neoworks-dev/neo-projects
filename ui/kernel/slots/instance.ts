// The window's one registry: the `slots` service registers into it and `<Slot>` draws from it.
import { SlotRegistry } from "./registry.ts";

// What fails with nobody to tell is rejected, which the host reports.
export const slotRegistry = new SlotRegistry((error) => void Promise.reject(error));
