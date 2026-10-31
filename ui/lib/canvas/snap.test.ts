import { describe, expect, test } from "bun:test";
import { snap } from "./snap.ts";

const box = (x: number, y: number, w = 100, h = 60) => ({ x, y, w, h });

describe("snap", () => {
  test("a left edge close to another's lines up with it", () => {
    const result = snap(box(204, 300), [box(200, 0)], 8);
    expect(result.dx).toBe(-4);
    expect(result.dy).toBe(0);
  });

  test("centres line up, and edges of other sizes", () => {
    // Centre 253 against centre 250 of a wider node.
    expect(snap(box(203, 300), [box(150, 0, 200)], 8).dx).toBe(-3);
    // Bottom 357 against the other's top 360: stacked just above it.
    expect(snap(box(0, 297), [box(400, 360)], 8).dy).toBe(3);
  });

  test("nothing within reach leaves it where it is, without guides", () => {
    expect(snap(box(230, 300), [box(100, 0)], 8)).toEqual({ dx: 0, dy: 0, guides: [] });
  });

  test("the nearest line wins", () => {
    expect(snap(box(205, 300), [box(200, 0), box(207, 600)], 8).dx).toBe(2);
  });

  test("one guide, across the gap between the two nodes", () => {
    const { guides } = snap(box(202, 300), [box(200, 0)], 8);
    expect(guides).toEqual([{ axis: "x", at: 200, from: 60, to: 300 }]);
  });

  test("of nodes on the same line, the guide goes to the nearest", () => {
    const { guides } = snap(box(202, 300), [box(200, 0), box(200, 420)], 8);
    expect(guides).toEqual([{ axis: "x", at: 200, from: 360, to: 420 }]);
  });

  test("a guide for each axis it snapped on", () => {
    const { guides } = snap(box(202, 3), [box(200, 300), box(500, 0)], 8);
    expect(guides.map((g) => g.axis).sort()).toEqual(["x", "y"]);
  });
});
