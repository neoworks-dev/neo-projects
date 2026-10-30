// Colours and the few styles everything shares. Dark, quiet surfaces; colour only where it means
// something (a status, a label, a selection).
import type { Style } from "@neoworks/svelte-gpui/ui";

export const c = {
  bg: "#0e0f11",
  dots: "#1c1e22",
  surface: "#16171a",
  raised: "#1d1f23",
  hover: "#25272c",
  border: "#2a2c32",
  borderStrong: "#3a3d44",
  text: "#e7e7ea",
  muted: "#8d9099",
  faint: "#5d6068",
  accent: "#7c8cff",
  accentSoft: "#7c8cff33",
  danger: "#f2555a",
  warn: "#d6a03a",
  ok: "#4cc27a",
};

/** GitHub's option colours (issue fields, our own Open/Closed). */
export const tones: Record<string, string> = {
  GRAY: "#8b949e",
  BLUE: "#4c8dff",
  GREEN: "#3fb950",
  YELLOW: "#d29922",
  ORANGE: "#db6d28",
  RED: "#f85149",
  PINK: "#db61a2",
  PURPLE: "#a371f7",
};

export function tone(name: string | null | undefined): string {
  return (name && tones[name.toUpperCase()]) || tones.GRAY;
}

/** A label's colour: GitHub gives six hex digits without the #. */
export function labelColor(hex: string | null | undefined): string {
  return hex && /^[0-9a-f]{6}$/i.test(hex) ? `#${hex}` : tones.GRAY;
}

export const text = {
  title: { fontSize: 15, fontWeight: "semibold", color: c.text } satisfies Style,
  body: { fontSize: 13, color: c.text, lineHeight: 19 } satisfies Style,
  small: { fontSize: 12, color: c.muted } satisfies Style,
  tiny: { fontSize: 11, color: c.faint } satisfies Style,
};

export const button: Style = {
  paddingX: 10,
  paddingY: 5,
  radius: 6,
  background: c.raised,
  borderWidth: 1,
  borderColor: c.border,
  color: c.text,
  fontSize: 12,
  gap: 6,
  align: "center",
  transition: 120,
};

export const buttonHover: Style = { background: c.hover, borderColor: c.borderStrong };

export const ghost: Style = {
  paddingX: 8,
  paddingY: 4,
  radius: 6,
  color: c.muted,
  fontSize: 12,
  gap: 6,
  align: "center",
  transition: 120,
};

export const ghostHover: Style = { background: c.hover, color: c.text };

export const field: Style = {
  paddingX: 8,
  paddingY: 6,
  radius: 6,
  background: c.bg,
  borderWidth: 1,
  borderColor: c.border,
  color: c.text,
  fontSize: 13,
};

export const fieldFocus: Style = { borderColor: c.accent };
