// A note's front matter as properties: which editor each value gets, and the values those editors
// write back. YAML has no date type here (the host reads `2026-10-30` as a string), so a date is a
// string in that shape.

/** The editor a property's value gets. "other" is shown as JSON and edited in the file. */
export type PropertyKind = "text" | "number" | "checkbox" | "date" | "list" | "other";

const ISO_DATE = /^\d{4}-\d{2}-\d{2}$/;

export function propertyKind(value: unknown): PropertyKind {
  if (value === null || typeof value === "string") {
    if (typeof value === "string" && ISO_DATE.test(value)) return "date";
    return "text";
  }
  if (typeof value === "number") return "number";
  if (typeof value === "boolean") return "checkbox";
  if (Array.isArray(value) && value.every((item) => typeof item === "string" || typeof item === "number")) {
    return "list";
  }
  return "other";
}

/** The number typed, or null when the text isn't one. */
export function numberFrom(text: string): number | null {
  const trimmed = text.trim();
  if (!trimmed) return null;
  const number = Number(trimmed);
  if (!Number.isFinite(number)) return null;
  return number;
}

/** A name for a new property that the note doesn't use yet: "property", then "property 2". */
export function newKey(existing: string[]): string {
  if (!existing.includes("property")) return "property";
  let n = 2;
  while (existing.includes(`property ${n}`)) n++;
  return `property ${n}`;
}

/** `2026-10-30` as that day at midnight, local time; null when it isn't a real day. */
export function dateFromIso(text: string): Date | null {
  if (!ISO_DATE.test(text)) return null;
  const [year, month, day] = text.split("-").map(Number);
  const date = new Date(year, month - 1, day);
  if (date.getMonth() !== month - 1) return null;
  return date;
}

/** A day as `2026-10-30`, local time. */
export function isoFromDate(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}
