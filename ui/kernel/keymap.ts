// The window's keys, as data: plugins bind chords and the window hands every key press to
// `dispatch`. Plain code, so it runs in tests; `Keys.svelte` is the window half and the `keymap`
// service the plugin half.

/** What a handler needs of a key press (a `KeyboardEvent` has all of it). */
export interface KeyEvent {
  key: string;
  ctrlKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  defaultPrevented: boolean;
  preventDefault(): void;
  /** The focused element, which the host names `g-input` for a text field. */
  target?: unknown;
}

/**
 * Whether the press is typing into a text field: a plain key while one has focus. The window
 * hears keys before the field does, so a binding of `n` or `backspace` would otherwise take
 * them from the text. Escape and chords with Ctrl, Alt or Meta still reach the bindings.
 */
export function typing(event: KeyEvent): boolean {
  const target = event.target as { localName?: string } | null | undefined;
  if (target?.localName !== "g-input") return false;
  if (event.ctrlKey || event.altKey || event.metaKey) return false;
  return event.key !== "Escape";
}

/** Returning `false` says the key was not used, so the next binding of the chord gets it. */
export type KeyHandler = (event: KeyEvent) => boolean | void;

type Dispose = () => void;

const MODIFIER_KEYS = new Set(["control", "shift", "alt", "meta"]);

/**
 * The chord a press makes, such as `ctrl-s`, `shift-tab`, `escape` or `f6`: the modifiers held
 * (in the order ctrl, alt, meta, shift) and the key, lower case. A modifier on its own is no chord.
 */
export function chord(event: KeyEvent): string | undefined {
  const key = event.key === " " ? "space" : event.key.toLowerCase();
  if (MODIFIER_KEYS.has(key)) return undefined;
  const held = [
    event.ctrlKey && "ctrl",
    event.altKey && "alt",
    event.metaKey && "meta",
    event.shiftKey && "shift",
  ].filter(Boolean);
  return [...held, key].join("-");
}

export interface BindOptions {
  /**
   * Bindings of a higher priority get the key before those of a lower one, whichever is newer. An
   * overlay that covers the window takes its Esc first this way, however late it mounted. Default 0.
   */
  priority?: number;
}

interface Binding {
  run: KeyHandler;
  priority: number;
}

export class Keymap {
  private readonly bindings = new Map<string, Binding[]>();
  private readonly observers = new Set<KeyHandler>();

  /** Binds a chord. Among bindings of one priority, the newest gets the key first. */
  bind(chord: string, run: KeyHandler, { priority = 0 }: BindOptions = {}): Dispose {
    const binding = { run, priority };
    const list = [...(this.bindings.get(chord) ?? [])];
    const at = list.findIndex((other) => other.priority <= priority);
    list.splice(at < 0 ? list.length : at, 0, binding);
    this.bindings.set(chord, list);
    return () => {
      const rest = (this.bindings.get(chord) ?? []).filter((other) => other !== binding);
      if (rest.length > 0) this.bindings.set(chord, rest);
      else this.bindings.delete(chord);
    };
  }

  /** Hears every key press (not bare modifiers) before the bindings do, and never uses it. */
  observe(run: KeyHandler): Dispose {
    this.observers.add(run);
    return () => this.observers.delete(run);
  }

  /** Runs the bindings of the press's chord until one uses it, and keeps the focused input from typing it. */
  dispatch(event: KeyEvent): boolean {
    const pressed = chord(event);
    if (pressed === undefined) return false;
    for (const observer of [...this.observers]) observer(event);
    if (typing(event)) return false;
    for (const { run } of [...(this.bindings.get(pressed) ?? [])]) {
      if (run(event) === false) continue;
      event.preventDefault();
      return true;
    }
    return false;
  }
}
