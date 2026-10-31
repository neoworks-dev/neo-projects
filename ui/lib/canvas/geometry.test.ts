import { describe, expect, test } from "bun:test";
import { arrowPath, dotPoint, gapBetween, grabZone, midpoint, near, resized } from "./geometry.ts";

const box = (x: number, y: number) => ({ x, y, w: 100, h: 60 });

describe("arrowPath", () => {
  test("far apart side by side: a curve from the right side to the left side", () => {
    const path = arrowPath(box(0, 0), box(400, 200))!;
    expect(path.length).toBeGreaterThan(2);
    expect(path[0]).toEqual({ x: 100, y: 30 });
    expect(path[path.length - 1]).toEqual({ x: 400, y: 230 });
  });

  test("stacked: a curve from the bottom to the top", () => {
    const path = arrowPath(box(0, 0), box(20, 300))!;
    expect(path[0]).toEqual({ x: 50, y: 60 });
    expect(path[path.length - 1]).toEqual({ x: 70, y: 300 });
  });

  test("close together: a straight line", () => {
    expect(arrowPath(box(0, 0), box(130, 0))).toHaveLength(2);
  });

  test("overlapping: nothing", () => {
    expect(arrowPath(box(0, 0), box(50, 10))).toBeNull();
  });
});

test("gapBetween", () => {
  expect(gapBetween(box(0, 0), box(130, 0))).toBe(30);
  expect(gapBetween(box(0, 0), box(50, 10))).toBe(0);
});

describe("resized", () => {
  const start = { x: 100, y: 100, w: 200, h: 100 };
  const min = { w: 80, h: 50 };

  test("a corner moves while the opposite one stays", () => {
    expect(resized(start, "se", 40, 20, min, false)).toEqual({ x: 100, y: 100, w: 240, h: 120 });
    expect(resized(start, "nw", 40, 20, min, false)).toEqual({ x: 140, y: 120, w: 160, h: 80 });
  });

  test("a side only changes its own dimension", () => {
    expect(resized(start, "w", -30, 99, min, false)).toEqual({ x: 70, y: 100, w: 230, h: 100 });
    expect(resized(start, "n", 99, -10, min, false)).toEqual({ x: 100, y: 90, w: 200, h: 110 });
  });

  test("it stops at the minimum size, with the opposite side still in place", () => {
    expect(resized(start, "nw", 500, 500, min, false)).toEqual({ x: 220, y: 150, w: 80, h: 50 });
  });

  test("Shift keeps the proportions, the larger change deciding", () => {
    expect(resized(start, "se", 100, 10, min, true)).toEqual({ x: 100, y: 100, w: 300, h: 150 });
    expect(resized(start, "e", 100, 0, min, true)).toEqual({ x: 100, y: 100, w: 300, h: 150 });
    expect(resized(start, "se", -500, -500, min, true)).toEqual({ x: 100, y: 100, w: 100, h: 50 });
  });
});

test("grab zones straddle the border, in screen pixels", () => {
  const r = { x: 10, y: 20, w: 100, h: 60 };
  expect(grabZone(r, "ne", 2)).toEqual({ x: 212, y: 32, w: 16, h: 16 });
  expect(grabZone(r, "s", 2)).toEqual({ x: 20, y: 156, w: 200, h: 8 });
  expect(grabZone(r, "w", 1)).toEqual({ x: 6, y: 20, w: 8, h: 60 });
});

test("arrow dots sit past the middle of each side and tuck back under it", () => {
  const r = { x: 10, y: 20, w: 100, h: 60 };
  expect(dotPoint(r, "n", 2, 14)).toEqual({ at: { x: 120, y: 26 }, back: { x: 0, y: 14 } });
  expect(dotPoint(r, "e", 1, 14)).toEqual({ at: { x: 124, y: 50 }, back: { x: -14, y: 0 } });
  expect(dotPoint(r, "s", 1, 14)).toEqual({ at: { x: 60, y: 94 }, back: { x: 0, y: -14 } });
  expect(dotPoint(r, "w", 1, 14)).toEqual({ at: { x: -4, y: 50 }, back: { x: 14, y: 0 } });
});

test("near finds the topmost node within the margin", () => {
  const under = { id: "under", x: 0, y: 0, w: 100, h: 100 };
  const over = { id: "over", x: 50, y: 50, w: 100, h: 100 };
  expect(near([under, over], { x: 75, y: 75 }, 0)?.id).toBe("over");
  expect(near([under, over], { x: 180, y: 100 }, 40)?.id).toBe("over");
  expect(near([under, over], { x: -30, y: 10 }, 40)?.id).toBe("under");
  expect(near([under, over], { x: 300, y: 300 }, 40)).toBeNull();
});

test("midpoint is halfway by length", () => {
  expect(midpoint([{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 30 }])).toEqual({ x: 10, y: 10 });
});
