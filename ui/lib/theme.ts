// Colours and the few styles everything shares, from the NeoWorks design system's dark theme:
// warm greys, hairline borders, a white fill for the one main action, lake blue for selection and
// focus. Colour only where it means something (a status, a label, a selection).
import { themes } from "@neoworks/svelte-gpui-ui";
import type { Shadow, Style } from "@neoworks/svelte-gpui/ui";

const t = themes.dark;

export const c = {
  bg: t.colors.canvas,
  dots: t.scales.grey[4],
  surface: t.colors.surface,
  raised: t.colors.sunken,
  hover: t.colors.hover,
  pressed: t.colors.pressed,
  border: t.colors.line,
  borderSoft: t.scales.grey[5],
  borderStrong: t.colors.lineStrong,
  text: t.colors.text,
  muted: t.colors.muted,
  faint: t.colors.faint,
  fill: t.colors.fill,
  fillHover: t.colors.fillHover,
  onFill: t.colors.onFill,
  accent: t.scales.lake[9],
  accentHover: t.scales.lake[10],
  accentText: t.scales.lake[11],
  accentSoft: "#4576b433",
  onAccent: t.colors.text,
  danger: t.colors.danger,
  dangerSoft: t.colors.dangerSoft,
  warn: t.colors.warning,
  ok: t.colors.success,
  pine: t.colors.pine,
};

export const radii = { control: 8, card: 12, panel: 16 };

/** Floating things: menus, sheets, the toolbar. */
export const shadow: Shadow[] = [
  { x: 0, y: 0, blur: 0, spread: 1, color: "rgba(255, 255, 255, 0.04)" },
  { x: 0, y: 12, blur: 32, spread: -8, color: "rgba(0, 0, 0, 0.55)" },
];

/** Easing for things that arrive: fast out, soft landing. */
export const ease = { duration: 180, easing: "ease-out" } as const;

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
  paddingX: 12,
  height: 30,
  radius: 15,
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
  paddingX: 10,
  height: 30,
  radius: 15,
  color: c.muted,
  fontSize: 12,
  gap: 6,
  align: "center",
  transition: 120,
};

export const ghostHover: Style = { background: c.hover, color: c.text };

export const field: Style = {
  paddingX: 10,
  paddingY: 7,
  radius: 8,
  background: c.bg,
  borderWidth: 1,
  borderColor: c.border,
  color: c.text,
  fontSize: 13,
};

export const fieldFocus: Style = { borderColor: c.accent };

/** The one main action: white on dark, as on neoworks.dev. */
export const primary: Style = { ...button, background: c.fill, borderColor: c.fill, color: c.onFill, fontWeight: "medium" };

export const primaryHover: Style = { background: c.fillHover, borderColor: c.fillHover };

/** A small uppercase heading over a group, as the site's section labels. */
export const eyebrow: Style = { fontSize: 11, fontWeight: "medium", color: c.faint, uppercase: true };
