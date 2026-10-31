import { expect, test } from "bun:test";
import { dropFrame, folderOpening, scaledBox } from "./dropIn.ts";

const folder = { x: 1000, y: 0, w: 220, h: 160 };

test("a held node shrinks towards the point it was grabbed at", () => {
  const box = scaledBox({ x: 0, y: 0, w: 200, h: 100 }, { x: 20, y: 10 }, 0.5);
  expect(box).toEqual({ x: 10, y: 5, w: 100, h: 50 });
});

test("a dropped node starts where it was let go and flies over the folder's middle", () => {
  const held = { x: 10, y: 5, w: 100, h: 50 };
  const start = dropFrame(held, 200, 100, folder, 0);
  expect(start).toEqual({ x: 60 - 100, y: 30 - 50, scale: 0.5, opacity: 1 });
  const over = dropFrame(held, 200, 100, folder, 0.45);
  expect(over).toEqual({ x: 1110 - 100, y: 80 - 50, scale: 0.5, opacity: 1 });
});

test("then it shrinks away as it sinks in", () => {
  const end = dropFrame({ x: 10, y: 5, w: 100, h: 50 }, 200, 100, folder, 1);
  expect(end.scale).toBe(0);
  expect(end.opacity).toBe(0);
  expect(end.y).toBeGreaterThan(80 - 50);
});

test("the folder opens as the nodes arrive and closes once they are in", () => {
  expect(folderOpening(0)).toBe(0);
  expect(folderOpening(0.5)).toBe(1);
  expect(folderOpening(0.8)).toBe(1);
  expect(folderOpening(1)).toBe(0);
});
