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

  test("a guide runs from one node to the other along the shared line", () => {
    const { guides } = snap(box(202, 300), [box(200, 0)], 8);
    expect(guides).toContainEqual({ axis: "x", at: 200, from: 0, to: 360 });
    expect(guides).toContainEqual({ axis: "x", at: 250, from: 0, to: 360 });
    expect(guides).toContainEqual({ axis: "x", at: 300, from: 0, to: 360 });
  });

  test("nodes on the same line share one guide across all of them", () => {
    const { guides } = snap(box(202, 300), [box(200, 0), box(200, 600)], 8);
    expect(guides.filter((g) => g.axis === "x" && g.at === 200)).toEqual([{ axis: "x", at: 200, from: 0, to: 660 }]);
  });
});
