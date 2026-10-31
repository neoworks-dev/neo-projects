import { describe, expect, test } from "bun:test";
import { arranged, columnsFor, readingOrder } from "./arrange.ts";

const node = (id: string, x: number, y: number, w = 100, h = 60) => ({ id, x, y, w, h });
const ids = (rects: { id: string }[]) => rects.map((r) => r.id);

describe("readingOrder", () => {
  test("rows from the top, each from the left, even when tops differ a little", () => {
    const rects = [node("c", 0, 200), node("b", 300, 10), node("a", 0, 0), node("d", 300, 190)];
    expect(ids(readingOrder(rects))).toEqual(["a", "b", "c", "d"]);
  });
});

describe("columnsFor", () => {
  test("a grid is about square", () => {
    expect([4, 6, 10].map((n) => columnsFor("grid", n))).toEqual([2, 3, 4]);
  });

  test("a row is one row and a column one column", () => {
    expect(columnsFor("row", 5)).toBe(5);
    expect(columnsFor("column", 5)).toBe(1);
  });
});

describe("arranged", () => {
  test("a grid starts at the top left of the selection, columns as wide as their widest node", () => {
    const rects = [node("a", 50, 40, 200, 60), node("b", 400, 40), node("c", 60, 300, 100, 150), node("d", 500, 320)];
    const spots = arranged(rects, "grid", 24);
    expect(spots.get("a")).toEqual({ x: 50, y: 40 });
    expect(spots.get("b")).toEqual({ x: 274, y: 40 });
    expect(spots.get("c")).toEqual({ x: 50, y: 124 });
    expect(spots.get("d")).toEqual({ x: 274, y: 124 });
  });

  test("a row puts them side by side in reading order", () => {
    const spots = arranged([node("b", 500, 0), node("a", 0, 300, 80)], "row", 24);
    expect(spots.get("b")).toEqual({ x: 0, y: 0 });
    expect(spots.get("a")).toEqual({ x: 124, y: 0 });
  });

  test("a column stacks them, each row as tall as its node", () => {
    const spots = arranged([node("a", 0, 0, 100, 40), node("b", 300, 100)], "column", 24);
    expect(spots.get("b")).toEqual({ x: 0, y: 64 });
  });
});
