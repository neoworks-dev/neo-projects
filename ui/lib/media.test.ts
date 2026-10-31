import { expect, test } from "bun:test";
import { duration, pages } from "./media.ts";

test("a length reads as a player shows it", () => {
  expect(duration(7.4)).toBe("0:07");
  expect(duration(754)).toBe("12:34");
  expect(duration(3723)).toBe("1:02:03");
  expect(duration(59.6)).toBe("1:00");
});

test("a page count says page once and pages otherwise", () => {
  expect(pages(1)).toBe("1 page");
  expect(pages(12)).toBe("12 pages");
});
