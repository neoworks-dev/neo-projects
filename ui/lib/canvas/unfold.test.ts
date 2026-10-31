import { expect, test } from "bun:test";
import { bounds, unfoldLayout } from "./unfold.ts";

const folder = { x: 0, y: 100, w: 220, h: 64 };

test("contents go in rows by kind to the right of the folder, sorted by name", () => {
  const spots = unfoldLayout(folder, [
    { id: "link", kind: "link", title: "example.com", w: 320, h: 260 },
    { id: "b", kind: "note", title: "Beans", w: 280, h: 160 },
    { id: "a", kind: "note", title: "apples", w: 280, h: 120 },
    { id: "pdf", kind: "file", title: "report.pdf", w: 240, h: 340 },
  ]);
  expect(spots.get("a")).toEqual({ x: 268, y: 100 });
  expect(spots.get("b")).toEqual({ x: 268 + 280 + 16, y: 100 });
  // The next row starts under the tallest of the row before.
  expect(spots.get("pdf")).toEqual({ x: 268, y: 100 + 160 + 28 });
  expect(spots.get("link")).toEqual({ x: 268, y: 100 + 160 + 28 + 340 + 28 });
});

test("kinds without a row of their own come last", () => {
  const spots = unfoldLayout(folder, [
    { id: "sticky", kind: "text", title: "Sow in March", w: 240, h: 170 },
    { id: "issue", kind: "issue", title: "Plant tomatoes", w: 280, h: 120 },
  ]);
  expect(spots.get("issue")).toEqual({ x: 268, y: 100 });
  expect(spots.get("sticky")).toEqual({ x: 268, y: 100 + 120 + 28 });
});

test("the bounds hold every box", () => {
  expect(bounds([])).toBeNull();
  expect(bounds([{ x: 0, y: 0, w: 10, h: 10 }, { x: 20, y: -5, w: 10, h: 10 }])).toEqual({ x: 0, y: -5, w: 30, h: 15 });
});
