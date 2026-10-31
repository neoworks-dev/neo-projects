// Strokes drawn on the canvas's background, kept free of Svelte so it is easy to test.
import type { Point } from "./geometry.ts";

/** `points` with every point closer than `gap` to the one kept before it dropped; the last always stays. */
export function thinned(points: Point[], gap: number): Point[] {
  if (points.length < 3) return points;
  const kept = [points[0]];
  for (const p of points.slice(1, -1)) {
    const last = kept[kept.length - 1];
    if (Math.hypot(p.x - last.x, p.y - last.y) >= gap) kept.push(p);
  }
  kept.push(points[points.length - 1]);
  return kept;
}

/** Points as the canvas file keeps them: x, y, x, y, …, to a tenth. */
export function flatPoints(points: Point[]): number[] {
  return points.flatMap((p) => [Math.round(p.x * 10) / 10, Math.round(p.y * 10) / 10]);
}

export function pointsOf(flat: number[]): Point[] {
  const points: Point[] = [];
  for (let i = 0; i + 1 < flat.length; i += 2) points.push({ x: flat[i], y: flat[i + 1] });
  return points;
}

/** How far `p` is from the segment from `a` to `b`. */
function distanceToSegment(p: Point, a: Point, b: Point): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const length = dx * dx + dy * dy;
  let t = 0;
  if (length > 0) t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / length));
  return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
}

/** Which side of the line through `a` and `b` the point `p` is on: positive, negative or 0 on it. */
function side(a: Point, b: Point, p: Point): number {
  return (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
}

/** How far apart the segments `a`–`b` and `c`–`d` come; 0 when they cross. */
function segmentGap(a: Point, b: Point, c: Point, d: Point): number {
  const crosses = side(a, b, c) * side(a, b, d) < 0 && side(c, d, a) * side(c, d, b) < 0;
  if (crosses) return 0;
  return Math.min(distanceToSegment(a, c, d), distanceToSegment(b, c, d), distanceToSegment(c, a, b), distanceToSegment(d, a, b));
}

/**
 * Whether the eraser, moved from `from` to `to`, came within `reach` of a stroke through
 * `points`. The whole way between counts, as the pointer jumps when moved fast.
 */
export function touchesStroke(points: Point[], from: Point, to: Point, reach: number): boolean {
  if (points.length === 1) return distanceToSegment(points[0], from, to) <= reach;
  for (let i = 1; i < points.length; i++) {
    if (segmentGap(from, to, points[i - 1], points[i]) <= reach) return true;
  }
  return false;
}
