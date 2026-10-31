// Where an unfolded folder's contents go: in rows to the right of the folder, one row per kind of
// thing (notes, then issues, images, other files, links, anything else), each sorted by name.
import type { Point, Rect } from "./geometry.ts";

export interface Folded {
  id: string;
  kind: string;
  title: string;
  w: number;
  h: number;
}

const ROWS = ["note", "issue", "image", "file", "folder", "link"];
/** Between the folder and its contents, between things in a row, and between rows. */
const AWAY = 48;
const GAP = 16;
const ROW_GAP = 28;

function row(kind: string): number {
  const index = ROWS.indexOf(kind);
  if (index === -1) return ROWS.length;
  return index;
}

/** Each thing's top left, by id. */
export function unfoldLayout(folder: Rect, things: Folded[]): Map<string, Point> {
  const sorted = [...things].sort((a, b) => row(a.kind) - row(b.kind) || a.title.localeCompare(b.title));
  const spots = new Map<string, Point>();
  const left = folder.x + folder.w + AWAY;
  let x = left;
  let y = folder.y;
  let height = 0;
  let current: number | null = null;
  for (const thing of sorted) {
    if (current !== null && row(thing.kind) !== current) {
      x = left;
      y += height + ROW_GAP;
      height = 0;
    }
    current = row(thing.kind);
    spots.set(thing.id, { x, y });
    x += thing.w + GAP;
    height = Math.max(height, thing.h);
  }
  return spots;
}

/** The box around everything placed, or null when nothing is. */
export function bounds(rects: Rect[]): Rect | null {
  if (rects.length === 0) return null;
  const left = Math.min(...rects.map((r) => r.x));
  const top = Math.min(...rects.map((r) => r.y));
  const right = Math.max(...rects.map((r) => r.x + r.w));
  const bottom = Math.max(...rects.map((r) => r.y + r.h));
  return { x: left, y: top, w: right - left, h: bottom - top };
}
