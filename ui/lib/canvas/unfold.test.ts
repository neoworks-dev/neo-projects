import { expect, test } from "bun:test";
import { bounds, outOf, peekCard, pushAside, rowsBox, shown, stagger, unfoldLayout } from "./unfold.ts";

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

test("each thing sets off a little after the one before, and all arrive together", () => {
  expect(stagger(0, 0, 3)).toBe(0);
  expect(stagger(0.1, 0, 3)).toBeGreaterThan(stagger(0.1, 2, 3));
  expect(stagger(0.05, 2, 3)).toBe(0);
  for (const index of [0, 1, 2]) expect(stagger(1, index, 3)).toBe(1);
  // However many there are, the last still sets off before the end.
  expect(stagger(0.6, 39, 40)).toBeGreaterThan(0);
});

test("each card peeking out of a folder is a little lower than the one behind it", () => {
  const tall = { x: 0, y: 100, w: 220, h: 170 };
  expect(peekCard(tall, 0)).toEqual({ x: 10, y: 100, w: 200, h: 156 });
  expect(peekCard(tall, 2)).toEqual({ x: 10, y: 120, w: 200, h: 136 });
});

test("on the way out, a thing starts as its card and grows into its spot", () => {
  const card = { x: 10, y: 100, w: 200, h: 156 };
  expect(outOf(card, { x: 268, y: 100 }, 400, 0)).toEqual({ x: 10, y: 100, scale: 0.5 });
  expect(outOf(card, { x: 268, y: 100 }, 400, 1)).toEqual({ x: 268, y: 100, scale: 1 });
  expect(outOf(card, { x: 268, y: 100 }, 400, 0.5)).toEqual({ x: 139, y: 100, scale: 0.75 });
});

test("a thing fades in as its card fades out, early on its way", () => {
  expect(shown(0)).toBe(0);
  expect(shown(0.2)).toBeCloseTo(0.6);
  expect(shown(0.5)).toBe(1);
});

test("the bounds hold every box", () => {
  expect(bounds([])).toBeNull();
  expect(bounds([{ x: 0, y: 0, w: 10, h: 10 }, { x: 20, y: -5, w: 10, h: 10 }])).toEqual({ x: 0, y: -5, w: 30, h: 15 });
});

const area = { x: 300, y: 100, w: 600, h: 400 };

test("nodes clear of the contents stay where they are", () => {
  const offsets = pushAside(area, [
    { id: "left", x: 0, y: 100, w: 200, h: 100 },
    { id: "below", x: 300, y: 600, w: 200, h: 100 },
  ]);
  expect(offsets.size).toBe(0);
});

test("a node in the way leaves by the shorter way, right or down", () => {
  const offsets = pushAside(area, [
    { id: "near the right edge", x: 800, y: 200, w: 200, h: 100 },
    { id: "near the bottom", x: 400, y: 450, w: 200, h: 100 },
  ]);
  expect(offsets.get("near the right edge")).toEqual({ x: 900 + 32 - 800, y: 0 });
  expect(offsets.get("near the bottom")).toEqual({ x: 0, y: 500 + 32 - 450 });
});

test("a pushed node pushes what it lands on the same way", () => {
  const offsets = pushAside(area, [
    { id: "pushed", x: 400, y: 450, w: 200, h: 100 },
    { id: "under it", x: 400, y: 560, w: 200, h: 100 },
    { id: "beside it", x: 1200, y: 450, w: 200, h: 100 },
  ]);
  expect(offsets.get("pushed")).toEqual({ x: 0, y: 82 });
  // The pushed node now ends at 632, so the one under it goes to 632 + 32.
  expect(offsets.get("under it")).toEqual({ x: 0, y: 664 - 560 });
  expect(offsets.has("beside it")).toBe(false);
});

test("a thing dragged out of the rows leaves no gap behind", () => {
  const things = [
    { id: "a", kind: "note", title: "a", w: 280, h: 160 },
    { id: "b", kind: "note", title: "b", w: 280, h: 160 },
    { id: "c", kind: "note", title: "c", w: 280, h: 160 },
  ];
  const staying = things.filter((thing) => thing.id !== "b");
  const spots = unfoldLayout(folder, staying);
  expect(spots.get("c")).toEqual({ x: 268 + 280 + 16, y: 100 });
  // The rows' box holds only what has a spot, so it shrinks with them.
  expect(rowsBox(spots, things)).toEqual({ x: 268, y: 100, w: 280 + 16 + 280, h: 160 });
  expect(rowsBox(new Map(), things)).toBeNull();
});
