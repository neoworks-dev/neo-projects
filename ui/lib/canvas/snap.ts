// Snapping a dragged node to the others: its edges or centre onto theirs, or the space beside it
// onto the canvas's spacing. Kept free of Svelte so it is easy to test.
import type { Rect } from "./geometry.ts";

/** The space a dragged node snaps to leave between itself and a neighbour, in canvas units. */
export const SPACING = 24;

/**
 * A guide to draw while dragging: `axis` "x" is a vertical line at x = `at`, running from `from`
 * to `to` along y; "y" is a horizontal one. `snaps` is the direction of the snap it shows, so
 * each direction has one guide at most.
 */
export interface Guide {
  snaps: "x" | "y";
  axis: "x" | "y";
  at: number;
  from: number;
  to: number;
}

/**
 * One way to snap along an axis: how far the node moves, how far that is, how far away the node
 * it snaps to lies across the axis, and the guide to show once it is placed.
 */
interface Match {
  shift: number;
  distance: number;
  gap: number;
  guide: (placed: Rect) => Guide;
}

const otherAxis = (axis: "x" | "y"): "x" | "y" => (axis === "x" ? "y" : "x");

/** Where `r` starts and ends along `axis`. */
function span(r: Rect, axis: "x" | "y"): [number, number] {
  if (axis === "x") return [r.x, r.x + r.w];
  return [r.y, r.y + r.h];
}

/** The left edge, centre and right edge of `r` along `axis`, or its top, middle and bottom. */
function lines(r: Rect, axis: "x" | "y"): number[] {
  const [start, end] = span(r, axis);
  return [start, (start + end) / 2, end];
}

/** The stretch between `a` and `b` along `axis`: the gap between them, or where they overlap. */
function between(a: Rect, b: Rect, axis: "x" | "y"): [number, number] {
  const [a0, a1] = span(a, axis);
  const [b0, b1] = span(b, axis);
  const inner = Math.min(a1, b1);
  const outer = Math.max(a0, b0);
  return [Math.min(inner, outer), Math.max(inner, outer)];
}

/** How much empty space lies between `a` and `b` along `axis`; 0 when they overlap. */
function gapAlong(a: Rect, b: Rect, axis: "x" | "y"): number {
  const [a0, a1] = span(a, axis);
  const [b0, b1] = span(b, axis);
  return Math.max(0, b0 - a1, a0 - b1);
}

/** Ways to line an edge or the centre of `moving` up with one of `other`'s along `axis`. */
function alignments(moving: Rect, other: Rect, axis: "x" | "y"): Match[] {
  const gap = gapAlong(moving, other, otherAxis(axis));
  return lines(other, axis).flatMap((at) =>
    lines(moving, axis).map((own) => ({
      shift: at - own,
      distance: Math.abs(at - own),
      gap,
      guide: (placed: Rect) => {
        const [from, to] = between(placed, other, otherAxis(axis));
        return { snaps: axis, axis, at, from, to };
      },
    })),
  );
}

/**
 * Ways to leave `SPACING` between `moving` and `other` along `axis`, on either side of it, when
 * they sit beside each other: overlapping across the axis. The guide crosses the space at the
 * middle of where they overlap.
 */
function spacings(moving: Rect, other: Rect, axis: "x" | "y"): Match[] {
  const across = otherAxis(axis);
  if (gapAlong(moving, other, across) > 0) return [];
  const [own0, own1] = span(moving, axis);
  const [other0, other1] = span(other, axis);
  const guide = (placed: Rect) => {
    const [overlap0, overlap1] = between(placed, other, across);
    const [from, to] = between(placed, other, axis);
    return { snaps: axis, axis: across, at: (overlap0 + overlap1) / 2, from, to };
  };
  const after = other1 + SPACING - own0;
  const before = other0 - SPACING - own1;
  return [
    { shift: after, distance: Math.abs(after), gap: 0, guide },
    { shift: before, distance: Math.abs(before), gap: 0, guide },
  ];
}

/**
 * The nearest way to snap `moving` along `axis` within `reach`, to the nodes no further than
 * `range` from it. Of ways as near as each other, the closest node wins, so the guide stays short.
 */
function nearestMatch(moving: Rect, others: Rect[], axis: "x" | "y", reach: number, range: number): Match | null {
  let best: Match | null = null;
  for (const other of others) {
    if (gapAlong(moving, other, otherAxis(axis)) > range || gapAlong(moving, other, axis) > range) continue;
    for (const match of [...alignments(moving, other, axis), ...spacings(moving, other, axis)]) {
      if (match.distance > reach) continue;
      const nearer = !best || match.distance < best.distance - 0.5;
      const asNear = best && Math.abs(match.distance - best.distance) <= 0.5 && match.gap < best.gap;
      if (nearer || asNear) best = match;
    }
  }
  return best;
}

/**
 * Where `moving` should go so an edge or its centre lines up with one of `others`, or it sits
 * `SPACING` from a neighbour, within `reach`, each axis on its own; and a guide for each axis it
 * snapped on. Only nodes within `range` of it count: lining up with something far off isn't
 * what a drag is after.
 */
export function snap(moving: Rect, others: Rect[], reach: number, range = Infinity): { dx: number; dy: number; guides: Guide[] } {
  const alongX = nearestMatch(moving, others, "x", reach, range);
  const alongY = nearestMatch(moving, others, "y", reach, range);
  const dx = alongX?.shift ?? 0;
  const dy = alongY?.shift ?? 0;
  const placed = { ...moving, x: moving.x + dx, y: moving.y + dy };
  const guides: Guide[] = [];
  if (alongX) guides.push(alongX.guide(placed));
  if (alongY) guides.push(alongY.guide(placed));
  return { dx, dy, guides };
}
