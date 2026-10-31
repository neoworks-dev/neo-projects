import { expect, test } from "bun:test";
import { pastedUrl } from "./paste.ts";

test("a web URL is a link, without the whitespace around it", () => {
  expect(pastedUrl("  https://example.com/a?b=c#d\n")).toBe("https://example.com/a?b=c#d");
  expect(pastedUrl("http://localhost:8080")).toBe("http://localhost:8080");
});

test("text that isn't a single web URL is not a link", () => {
  expect(pastedUrl("")).toBeNull();
  expect(pastedUrl("hello world")).toBeNull();
  expect(pastedUrl("notes.md")).toBeNull();
  expect(pastedUrl("see https://example.com")).toBeNull();
  expect(pastedUrl("https://a.com https://b.com")).toBeNull();
  expect(pastedUrl("file:///home/me/a.png")).toBeNull();
});
