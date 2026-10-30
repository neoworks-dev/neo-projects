import { expect, test } from "bun:test";
import { Keymap, type KeyEvent } from "./keymap.ts";

function press(key: string, target: string | null, mods: Partial<KeyEvent> = {}): KeyEvent {
  return {
    key,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    shiftKey: false,
    defaultPrevented: false,
    preventDefault() {
      this.defaultPrevented = true;
    },
    target: target ? { localName: target } : null,
    ...mods,
  };
}

test("plain keys typed into a field skip the bindings", () => {
  const keymap = new Keymap();
  const heard: string[] = [];
  keymap.bind("n", () => void heard.push("n"));
  keymap.bind("backspace", () => void heard.push("backspace"));
  keymap.bind("escape", () => void heard.push("escape"));
  keymap.bind("ctrl-r", () => void heard.push("ctrl-r"));

  const typed = press("n", "g-input");
  expect(keymap.dispatch(typed)).toBe(false);
  expect(typed.defaultPrevented).toBe(false);
  expect(keymap.dispatch(press("Backspace", "g-input"))).toBe(false);
  expect(heard).toEqual([]);

  expect(keymap.dispatch(press("Escape", "g-input"))).toBe(true);
  expect(keymap.dispatch(press("r", "g-input", { ctrlKey: true }))).toBe(true);
  expect(keymap.dispatch(press("n", "g-button"))).toBe(true);
  expect(keymap.dispatch(press("n", null))).toBe(true);
  expect(heard).toEqual(["escape", "ctrl-r", "n", "n"]);
});
