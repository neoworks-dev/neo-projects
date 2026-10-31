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

/** The node a dragged one lines up with along an axis: how far it moves to, and on which line. */
interface Match {
  shift: number;
  at: number;
  other: Rect;
}

/** The left edge, centre and right edge of `r` along `axis`, or its top, middle and bottom. */
function lines(r: Rect, axis: "x" | "y"): number[] {
  if (axis === "x") return [r.x, r.x + r.w / 2, r.x + r.w];
  return [r.y, r.y + r.h / 2, r.y + r.h];
}

/** Where `r` starts and ends across `axis`. */
function across(r: Rect, axis: "x" | "y"): [number, number] {
  if (axis === "x") return [r.y, r.y + r.h];
  return [r.x, r.x + r.w];
}

/** The stretch between `a` and `b` across `axis`: the gap between them, or where they overlap. */
function between(a: Rect, b: Rect, axis: "x" | "y"): [number, number] {
  const [a0, a1] = across(a, axis);
  const [b0, b1] = across(b, axis);
  const inner = Math.min(a1, b1);
  const outer = Math.max(a0, b0);
  return [Math.min(inner, outer), Math.max(inner, outer)];
}

/**
 * The line of another node that one of `moving`'s lines is nearest to along `axis`, within
 * `reach`. Of lines as near as each other, the node closest across the axis wins, so the guide
 * between them stays short.
 */
function nearestMatch(moving: Rect, others: Rect[], axis: "x" | "y", reach: number): Match | null {
  let best: Match | null = null;
  let bestDistance = reach;
  let bestGap = Infinity;
  for (const other of others) {
    const [from, to] = between(moving, other, axis);
    const gap = to - from;
    for (const at of lines(other, axis)) {
      for (const own of lines(moving, axis)) {
        const distance = Math.abs(at - own);
        const nearer = distance < bestDistance - 0.5;
        const asNear = Math.abs(distance - bestDistance) <= 0.5 && gap < bestGap;
        if (distance > reach || (!nearer && !asNear)) continue;
        best = { shift: at - own, at, other };
        bestDistance = distance;
        bestGap = gap;
      }
    }
  }
  return best;
}

/** The guide for `match`: along its line, across the stretch between `placed` and the other node. */
function guideFor(placed: Rect, match: Match, axis: "x" | "y"): Guide {
  const [from, to] = between(placed, match.other, axis);
  return { axis, at: match.at, from, to };
}

/**
 * Where `moving` should go so an edge or its centre lines up with one of `others` within
 * `reach`, each axis on its own, and a guide for each axis it snapped on.
 */
export function snap(moving: Rect, others: Rect[], reach: number): { dx: number; dy: number; guides: Guide[] } {
  const alongX = nearestMatch(moving, others, "x", reach);
  const alongY = nearestMatch(moving, others, "y", reach);
  const dx = alongX?.shift ?? 0;
  const dy = alongY?.shift ?? 0;
  const placed = { ...moving, x: moving.x + dx, y: moving.y + dy };
  const guides: Guide[] = [];
  if (alongX) guides.push(guideFor(placed, alongX, "x"));
  if (alongY) guides.push(guideFor(placed, alongY, "y"));
  return { dx, dy, guides };
}
