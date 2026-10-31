// Arranging selected nodes into a grid, a row or a column, kept free of Svelte so it is easy to test.
import type { Point, Rect } from "./geometry.ts";

export type Arrangement = "grid" | "row" | "column";

/** The space between neighbouring cells, in canvas units. */
export const ARRANGE_GAP = 24;

/**
 * `rects` in the order they are read now: in rows from the top, each from the left. A node joins
 * a row while its top is above the middle of the row's first node.
 */
export function readingOrder<R extends Rect>(rects: R[]): R[] {
  const byTop = [...rects].sort((a, b) => a.y - b.y);
  const rows: R[][] = [];
  for (const r of byTop) {
    const row = rows[rows.length - 1];
    if (row && r.y < row[0].y + row[0].h / 2) row.push(r);
    else rows.push([r]);
  }
  return rows.flatMap((row) => row.sort((a, b) => a.x - b.x));
}

/** How many columns `count` nodes take: about the square root, so the grid comes out roughly square. */
export function columnsFor(arrangement: Arrangement, count: number): number {
  if (arrangement === "row") return count;
  if (arrangement === "column") return 1;
  return Math.ceil(Math.sqrt(count));
}

/**
 * Where each of `rects` goes, by id, laid out in reading order from the top left of the box
 * around them. Each column is as wide as its widest node and each row as tall as its tallest,
 * and every node sits in the top left of its cell, keeping its size.
 */
export function arranged<R extends Rect & { id: string }>(rects: R[], arrangement: Arrangement, gap = ARRANGE_GAP): Map<string, Point> {
  const spots = new Map<string, Point>();
  if (rects.length === 0) return spots;
  const order = readingOrder(rects);
  const columns = columnsFor(arrangement, order.length);
  const widths: number[] = [];
  const heights: number[] = [];
  order.forEach((r, i) => {
    const column = i % columns;
    const row = Math.floor(i / columns);
    widths[column] = Math.max(widths[column] ?? 0, r.w);
    heights[row] = Math.max(heights[row] ?? 0, r.h);
  });
  const left = Math.min(...rects.map((r) => r.x));
  const top = Math.min(...rects.map((r) => r.y));
  order.forEach((r, i) => {
    const column = i % columns;
    const row = Math.floor(i / columns);
    const x = left + widths.slice(0, column).reduce((sum, w) => sum + w + gap, 0);
    const y = top + heights.slice(0, row).reduce((sum, h) => sum + h + gap, 0);
    spots.set(r.id, { x, y });
  });
  return spots;
}
