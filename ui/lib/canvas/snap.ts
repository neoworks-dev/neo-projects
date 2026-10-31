// Snapping a dragged node to the edges and centres of the others, kept free of Svelte so it is
// easy to test.
import type { Rect } from "./geometry.ts";

/**
 * A line two nodes are aligned on, to draw while dragging: `axis` "x" is a vertical line at
 * x = `at`, running from `from` to `to` along y; "y" is a horizontal one.
 */
export interface Guide {
  axis: "x" | "y";
  at: number;
  from: number;
  to: number;
}

/** The left edge, centre and right edge of `r` along `axis`, or its top, middle and bottom. */
function lines(r: Rect, axis: "x" | "y"): number[] {
  if (axis === "x") return [r.x, r.x + r.w / 2, r.x + r.w];
  return [r.y, r.y + r.h / 2, r.y + r.h];
}

/** How far to shift `moving` along `axis` so one of its lines meets one of another's; 0 when none is within `reach`. */
function nearestShift(moving: Rect, others: Rect[], axis: "x" | "y", reach: number): number {
  let best = 0;
  let bestDistance = reach;
  for (const other of others) {
    for (const target of lines(other, axis)) {
      for (const own of lines(moving, axis)) {
        const distance = Math.abs(target - own);
        if (distance <= bestDistance) {
          best = target - own;
          bestDistance = distance;
        }
      }
    }
  }
  return best;
}

/**
 * A guide for every line `placed` now shares with some of `others` along `axis`, running across
 * all the nodes on it.
 */
function guidesAlong(placed: Rect, others: Rect[], axis: "x" | "y"): Guide[] {
  const guides = new Map<number, Guide>();
  const across = axis === "x" ? "y" : "x";
  const span = axis === "x" ? "h" : "w";
  for (const other of others) {
    for (const at of lines(other, axis)) {
      if (!lines(placed, axis).some((own) => Math.abs(own - at) < 0.5)) continue;
      const guide = guides.get(at) ?? { axis, at, from: placed[across], to: placed[across] + placed[span] };
      guide.from = Math.min(guide.from, other[across]);
      guide.to = Math.max(guide.to, other[across] + other[span]);
      guides.set(at, guide);
    }
  }
  return [...guides.values()];
}

/**
 * Where `moving` should go so its edges or centre line up with those of `others` within
 * `reach`, each axis on its own, and the guides to show for the lines it now shares.
 */
export function snap(moving: Rect, others: Rect[], reach: number): { dx: number; dy: number; guides: Guide[] } {
  const dx = nearestShift(moving, others, "x", reach);
  const dy = nearestShift(moving, others, "y", reach);
  const placed = { ...moving, x: moving.x + dx, y: moving.y + dy };
  const guides = [...guidesAlong(placed, others, "x"), ...guidesAlong(placed, others, "y")];
  return { dx, dy, guides };
}
