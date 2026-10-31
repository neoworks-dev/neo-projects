// Plain geometry for the canvas: kept free of Svelte so it is easy to test.

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Point {
  x: number;
  y: number;
}

/** Where the segment from the centre of `r` towards `to` leaves `r`. */
export function exit(r: Rect, to: Point): Point {
  const cx = r.x + r.w / 2;
  const cy = r.y + r.h / 2;
  const dx = to.x - cx;
  const dy = to.y - cy;
  if (dx === 0 && dy === 0) return { x: cx, y: cy };
  const sx = dx === 0 ? Infinity : r.w / 2 / Math.abs(dx);
  const sy = dy === 0 ? Infinity : r.h / 2 / Math.abs(dy);
  const s = Math.min(sx, sy);
  return { x: cx + dx * s, y: cy + dy * s };
}

export function center(r: Rect): Point {
  return { x: r.x + r.w / 2, y: r.y + r.h / 2 };
}

/** The two ends of an arrow from `a` to `b`, from border to border; null when they overlap. */
export function arrowEnds(a: Rect, b: Rect): [Point, Point] | null {
  if (a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h) return null;
  return [exit(a, center(b)), exit(b, center(a))];
}

/** The two short strokes of an arrowhead at `tip`, pointing away from `from`. */
export function arrowHead(from: Point, tip: Point, size: number): [Point, Point] {
  const angle = Math.atan2(tip.y - from.y, tip.x - from.x);
  const spread = 0.45;
  return [
    { x: tip.x - size * Math.cos(angle - spread), y: tip.y - size * Math.sin(angle - spread) },
    { x: tip.x - size * Math.cos(angle + spread), y: tip.y - size * Math.sin(angle + spread) },
  ];
}

export function contains(r: Rect, p: Point): boolean {
  return p.x >= r.x && p.x <= r.x + r.w && p.y >= r.y && p.y <= r.y + r.h;
}

/** The empty space between two rects, along the axis they are furthest apart on; 0 when they touch. */
export function gapBetween(a: Rect, b: Rect): number {
  const dx = Math.max(b.x - (a.x + a.w), a.x - (b.x + b.w), 0);
  const dy = Math.max(b.y - (a.y + a.h), a.y - (b.y + b.h), 0);
  return Math.max(dx, dy);
}

/** Nodes closer than this get a straight arrow: a curve has no room to bend between them. */
export const curveGap = 56;

/**
 * An arrow from `a` to `b` as points to draw through: a cubic bezier from the middle of the side
 * of `a` that faces `b` to the middle of the side of `b` that faces `a`, leaving and arriving
 * square to those sides. Close nodes get a straight line, border to border. Null when they overlap.
 */
export function arrowPath(a: Rect, b: Rect, segments = 24): Point[] | null {
  const ends = arrowEnds(a, b);
  if (!ends) return null;
  if (gapBetween(a, b) < curveGap) return ends;
  const ca = center(a);
  const cb = center(b);
  // Sides by which axis the gap is on, so stacked nodes connect top to bottom.
  const horizontal = Math.max(b.x - (a.x + a.w), a.x - (b.x + b.w)) >= Math.max(b.y - (a.y + a.h), a.y - (b.y + b.h));
  let p: Point;
  let q: Point;
  let out: Point;
  if (horizontal) {
    const sign = cb.x > ca.x ? 1 : -1;
    p = { x: sign > 0 ? a.x + a.w : a.x, y: ca.y };
    q = { x: sign > 0 ? b.x : b.x + b.w, y: cb.y };
    out = { x: sign, y: 0 };
  } else {
    const sign = cb.y > ca.y ? 1 : -1;
    p = { x: ca.x, y: sign > 0 ? a.y + a.h : a.y };
    q = { x: cb.x, y: sign > 0 ? b.y : b.y + b.h };
    out = { x: 0, y: sign };
  }
  const reach = Math.min(160, Math.max(40, Math.hypot(q.x - p.x, q.y - p.y) * 0.4));
  const c1 = { x: p.x + out.x * reach, y: p.y + out.y * reach };
  const c2 = { x: q.x - out.x * reach, y: q.y - out.y * reach };
  const points: Point[] = [];
  for (let i = 0; i <= segments; i++) {
    const t = i / segments;
    const u = 1 - t;
    points.push({
      x: u * u * u * p.x + 3 * u * u * t * c1.x + 3 * u * t * t * c2.x + t * t * t * q.x,
      y: u * u * u * p.y + 3 * u * u * t * c1.y + 3 * u * t * t * c2.y + t * t * t * q.y,
    });
  }
  return points;
}

/** The point halfway along a path, by length. */
export function midpoint(points: Point[]): Point {
  const lengths = points.slice(1).map((p, i) => Math.hypot(p.x - points[i].x, p.y - points[i].y));
  let left = lengths.reduce((sum, l) => sum + l, 0) / 2;
  for (let i = 0; i < lengths.length; i++) {
    if (left <= lengths[i]) {
      const t = lengths[i] === 0 ? 0 : left / lengths[i];
      return { x: points[i].x + (points[i + 1].x - points[i].x) * t, y: points[i].y + (points[i + 1].y - points[i].y) * t };
    }
    left -= lengths[i];
  }
  return points[points.length - 1];
}

/** Where a node is grabbed to resize it: a side or a corner, by compass point. */
export type Handle = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";

/** Sides first, so the corners are laid over their ends and win where they meet. */
export const handles: Handle[] = ["n", "e", "s", "w", "nw", "ne", "se", "sw"];

/** How thick a side's grab strip is and how large a corner's square, in screen pixels. */
const EDGE = 8;
const CORNER = 16;

/**
 * The invisible area that grabs `handle` of `r` drawn at `zoom`, in screen pixels: a strip
 * along a side or a square on a corner, half inside the node and half outside.
 */
export function grabZone(r: Rect, handle: Handle, zoom: number): Rect {
  const left = r.x * zoom;
  const top = r.y * zoom;
  const right = (r.x + r.w) * zoom;
  const bottom = (r.y + r.h) * zoom;
  if (handle.length === 2) {
    const x = handle.includes("w") ? left : right;
    const y = handle.includes("n") ? top : bottom;
    return { x: x - CORNER / 2, y: y - CORNER / 2, w: CORNER, h: CORNER };
  }
  if (handle === "n" || handle === "s") {
    const y = handle === "n" ? top : bottom;
    return { x: left, y: y - EDGE / 2, w: right - left, h: EDGE };
  }
  const x = handle === "w" ? left : right;
  return { x: x - EDGE / 2, y: top, w: EDGE, h: bottom - top };
}

export type Side = "n" | "e" | "s" | "w";

/**
 * Where the arrow dot on `side` of `r` sits once it is out, in screen pixels: `out` past the
 * middle of that side. `back` is the offset that tucks it under the node again.
 */
export function dotPoint(r: Rect, side: Side, zoom: number, out: number): { at: Point; back: Point } {
  const middle = { x: (r.x + r.w / 2) * zoom, y: (r.y + r.h / 2) * zoom };
  if (side === "n") return { at: { x: middle.x, y: r.y * zoom - out }, back: { x: 0, y: out } };
  if (side === "s") return { at: { x: middle.x, y: (r.y + r.h) * zoom + out }, back: { x: 0, y: -out } };
  if (side === "w") return { at: { x: r.x * zoom - out, y: middle.y }, back: { x: out, y: 0 } };
  return { at: { x: (r.x + r.w) * zoom + out, y: middle.y }, back: { x: -out, y: 0 } };
}

/** The largest size of the proportions `aspect` (width divided by height) that fits `w`×`h`. */
export function contain(w: number, h: number, aspect: number): { w: number; h: number } {
  if (w / h > aspect) return { w: h * aspect, h };
  return { w, h: w / aspect };
}

/** The side of `r` that `at` is closest to, inside or outside it. */
export function nearestSide(r: Rect, at: Point): Side {
  const distances: [Side, number][] = [
    ["n", Math.abs(at.y - r.y)],
    ["e", Math.abs(at.x - (r.x + r.w))],
    ["s", Math.abs(at.y - (r.y + r.h))],
    ["w", Math.abs(at.x - r.x)],
  ];
  distances.sort((a, b) => a[1] - b[1]);
  return distances[0][0];
}

/** The topmost of `nodes` (the last drawn) within `margin` of `at`, or null. */
export function near<R extends Rect>(nodes: R[], at: Point, margin: number): R | null {
  for (let i = nodes.length - 1; i >= 0; i--) {
    const r = nodes[i];
    const grown = { x: r.x - margin, y: r.y - margin, w: r.w + 2 * margin, h: r.h + 2 * margin };
    if (contains(grown, at)) return r;
  }
  return null;
}

/**
 * `start` after dragging its `handle` by `dx`, `dy`: the opposite side or corner stays where it
 * was, and the size stays at least `min`. With `keepAspect` (Shift held) the proportions of
 * `start` are kept, the larger of the two changes deciding the size.
 */
export function resized(start: Rect, handle: Handle, dx: number, dy: number, min: { w: number; h: number }, keepAspect: boolean): Rect {
  let w = start.w;
  if (handle.includes("e")) w = start.w + dx;
  if (handle.includes("w")) w = start.w - dx;
  let h = start.h;
  if (handle.includes("s")) h = start.h + dy;
  if (handle.includes("n")) h = start.h - dy;

  if (keepAspect) {
    const sideways = handle === "e" || handle === "w";
    const upright = handle === "n" || handle === "s";
    let scale = Math.max(w / start.w, h / start.h);
    if (sideways) scale = w / start.w;
    if (upright) scale = h / start.h;
    scale = Math.max(scale, min.w / start.w, min.h / start.h);
    w = start.w * scale;
    h = start.h * scale;
  }
  w = Math.round(Math.max(min.w, w));
  h = Math.round(Math.max(min.h, h));

  let x = start.x;
  if (handle.includes("w")) x = start.x + start.w - w;
  let y = start.y;
  if (handle.includes("n")) y = start.y + start.h - h;
  return { x, y, w, h };
}

/** A free spot for a new node of size w×h near `at`, stepping right then down past `taken`. */
export function freeSpot(taken: Rect[], at: Point, w: number, h: number, gap = 24): Point {
  let candidate = { x: at.x, y: at.y, w, h };
  for (let tries = 0; tries < 200; tries++) {
    const hit = taken.find(
      (r) =>
        candidate.x < r.x + r.w + gap &&
        r.x < candidate.x + candidate.w + gap &&
        candidate.y < r.y + r.h + gap &&
        r.y < candidate.y + candidate.h + gap,
    );
    if (!hit) return { x: candidate.x, y: candidate.y };
    candidate = { ...candidate, x: hit.x + hit.w + gap };
    if (candidate.x > at.x + 4 * (w + gap)) candidate = { ...candidate, x: at.x, y: candidate.y + h + gap };
  }
  return { x: candidate.x, y: candidate.y };
}
