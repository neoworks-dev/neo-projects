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
