import { describe, expect, test } from "bun:test";
import { arrowPath, gapBetween, midpoint } from "./geometry.ts";

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

test("midpoint is halfway by length", () => {
  expect(midpoint([{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 30 }])).toEqual({ x: 10, y: 10 });
});
