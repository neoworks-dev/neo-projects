import { expect, test } from "bun:test";
import { cubicBezier } from "./motion.ts";

test("a cubic bezier runs from 0 to 1", () => {
  const out = cubicBezier([0.22, 1, 0.36, 1]);
  expect(out(0)).toBe(0);
  expect(out(1)).toBe(1);
});

test("the linear bezier is time itself", () => {
  const linear = cubicBezier([0, 0, 1, 1]);
  for (const t of [0.1, 0.25, 0.5, 0.9]) expect(linear(t)).toBeCloseTo(t, 4);
});

test("an ease-out curve is most of the way there by half time", () => {
  const out = cubicBezier([0.22, 1, 0.36, 1]);
  expect(out(0.5)).toBeGreaterThan(0.85);
  expect(out(0.25)).toBeLessThan(out(0.5));
});
