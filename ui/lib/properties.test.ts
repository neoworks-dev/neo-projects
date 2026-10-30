import { expect, test } from "bun:test";
import { dateFromIso, isoFromDate, newKey, numberFrom, propertyKind } from "./properties.ts";

test("each value gets the editor of its type", () => {
  expect(propertyKind("hello")).toBe("text");
  expect(propertyKind(null)).toBe("text");
  expect(propertyKind("2026-10-30")).toBe("date");
  expect(propertyKind("2026-10-30T12:00")).toBe("text");
  expect(propertyKind(3)).toBe("number");
  expect(propertyKind(false)).toBe("checkbox");
  expect(propertyKind(["a", 2])).toBe("list");
  expect(propertyKind([])).toBe("list");
  expect(propertyKind([["nested"]])).toBe("other");
  expect(propertyKind({ a: 1 })).toBe("other");
});

test("numbers parse, anything else doesn't", () => {
  expect(numberFrom(" 42 ")).toBe(42);
  expect(numberFrom("-1.5")).toBe(-1.5);
  expect(numberFrom("")).toBeNull();
  expect(numberFrom("12px")).toBeNull();
});

test("a new key is one the note doesn't use", () => {
  expect(newKey(["id"])).toBe("property");
  expect(newKey(["property", "property 2"])).toBe("property 3");
});

test("dates go to and from the ISO day", () => {
  const date = dateFromIso("2026-02-03");
  expect(date?.getDate()).toBe(3);
  expect(isoFromDate(date!)).toBe("2026-02-03");
  expect(dateFromIso("2026-02-30")).toBeNull();
  expect(dateFromIso("tomorrow")).toBeNull();
});
