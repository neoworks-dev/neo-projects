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

/** The box the things of `things` that have a spot take in the rows; null when none has. */
export function rowsBox(spots: Map<string, Point>, things: Folded[]): Rect | null {
  const placed = things.flatMap((thing) => {
    const spot = spots.get(thing.id);
    if (!spot) return [];
    return [{ ...spot, w: thing.w, h: thing.h }];
  });
  return bounds(placed);
}

/** Kept clear between an unfolded folder's contents and the nodes they push aside. */
const CLEAR = 32;

type Way = "right" | "down";

interface Pusher {
  rect: Rect;
  /** The way it was pushed itself, which it pushes others; null for the contents' area. */
  way: Way | null;
}

/**
 * How far each of `nodes` moves, by id, so none comes within CLEAR of `area`, the box an unfolded
 * folder's contents take. A node in the way leaves by the shorter way, right or down, and pushes
 * what it then lands on the same way. Nodes that stay are left out.
 */
export function pushAside(area: Rect, nodes: (Rect & { id: string })[]): Map<string, Point> {
  const placed = nodes.map((n) => ({ ...n }));
  const pushers: Pusher[] = [{ rect: area, way: null }];
  // Pushes only go right and down, so they settle; the limit guards against a chain that doesn't.
  const limit = 1 + placed.length * placed.length;
  for (let i = 0; i < pushers.length && i < limit; i++) {
    const { rect, way } = pushers[i];
    for (const node of placed) {
      if (node === rect || !withinClear(rect, node)) continue;
      const nodeWay = way ?? shorterWay(rect, node);
      if (nodeWay === "right") node.x = rect.x + rect.w + CLEAR;
      else node.y = rect.y + rect.h + CLEAR;
      pushers.push({ rect: node, way: nodeWay });
    }
  }
  const offsets = new Map<string, Point>();
  for (const [index, node] of placed.entries()) {
    const start = nodes[index];
    if (node.x !== start.x || node.y !== start.y) offsets.set(node.id, { x: node.x - start.x, y: node.y - start.y });
  }
  return offsets;
}

function withinClear(a: Rect, b: Rect): boolean {
  return b.x < a.x + a.w + CLEAR && a.x - CLEAR < b.x + b.w && b.y < a.y + a.h + CLEAR && a.y - CLEAR < b.y + b.h;
}

/** Whether `node` gets clear of `rect` sooner by moving right or down. */
function shorterWay(rect: Rect, node: Rect): Way {
  const right = rect.x + rect.w + CLEAR - node.x;
  const down = rect.y + rect.h + CLEAR - node.y;
  if (right <= down) return "right";
  return "down";
}

/** At most this much of the unfolding passes between one thing setting off and the next. */
const STAGGER = 0.08;

/**
 * How far along the thing at `index` of `count` is when the whole unfolding is at `progress`,
 * both 0 to 1: each sets off a little after the one before, and all arrive together.
 */
export function stagger(progress: number, index: number, count: number): number {
  const step = Math.min(STAGGER, 0.5 / Math.max(1, count - 1));
  const span = 1 - step * (count - 1);
  return Math.min(1, Math.max(0, (progress - index * step) / span));
}

/** How many of a folder's first things peek out of it as cards, as the host sends them. */
export const PEEK = 3;
/** How far the peeking cards stay in from the folder's sides and bottom, and how much lower each is than the one behind. */
export const PEEK_INSET = 10;
export const PEEK_BOTTOM = 14;
export const PEEK_STEP = 10;

/** The card the folder's thing at `index` peeks out as, down behind the folder's front. */
export function peekCard(folder: Rect, index: number): Rect {
  const top = folder.y + index * PEEK_STEP;
  return { x: folder.x + PEEK_INSET, y: top, w: folder.w - 2 * PEEK_INSET, h: folder.y + folder.h - PEEK_BOTTOM - top };
}

/**
 * Where a thing `w` wide is at `t` on its way from its card `card` to `spot`, and how big: it
 * starts as wide as the card at its top left and grows to its own size.
 */
export function outOf(card: Rect, spot: Point, w: number, t: number): { x: number; y: number; scale: number } {
  const from = card.w / w;
  return { x: card.x + (spot.x - card.x) * t, y: card.y + (spot.y - card.y) * t, scale: from + (1 - from) * t };
}

/** How far a thing on its way out has faded in, and so its card out: within the first third of the way. */
export function shown(t: number): number {
  return Math.min(1, t * 3);
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
