import { describe, expect, test } from "bun:test";
import { flatPoints, pointsOf, thinned, touchesStroke } from "./strokes.ts";

describe("thinned", () => {
  test("drops points closer than the gap, keeping the first and last", () => {
    const points = [
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 3, y: 0 },
      { x: 3.5, y: 0 },
    ];
    expect(thinned(points, 2)).toEqual([
      { x: 0, y: 0 },
      { x: 3, y: 0 },
      { x: 3.5, y: 0 },
    ]);
  });
});

describe("flatPoints", () => {
  test("keeps a tenth and comes back as points", () => {
    const flat = flatPoints([
      { x: 1.234, y: 5 },
      { x: -2.06, y: 0.05 },
    ]);
    expect(flat).toEqual([1.2, 5, -2.1, 0.1]);
    expect(pointsOf(flat)).toEqual([
      { x: 1.2, y: 5 },
      { x: -2.1, y: 0.1 },
    ]);
  });
});

describe("touchesStroke", () => {
  const line = [
    { x: 0, y: 0 },
    { x: 100, y: 0 },
  ];

  const at = (x: number, y: number) => ({ x, y });

  test("along a segment, not only at its points", () => {
    expect(touchesStroke(line, at(50, 6), at(50, 6), 8)).toBe(true);
    expect(touchesStroke(line, at(50, 12), at(50, 12), 8)).toBe(false);
  });

  test("past its ends only within reach", () => {
    expect(touchesStroke(line, at(106, 0), at(106, 0), 8)).toBe(true);
    expect(touchesStroke(line, at(110, 0), at(110, 0), 8)).toBe(false);
  });

  test("a jump from one side of it to the other crosses it", () => {
    expect(touchesStroke(line, at(50, -40), at(50, 40), 8)).toBe(true);
    expect(touchesStroke(line, at(150, -40), at(150, 40), 8)).toBe(false);
  });
});
