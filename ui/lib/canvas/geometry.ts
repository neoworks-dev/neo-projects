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
