import { expect, test } from "bun:test";
import { pasted, pastedImageName, pastedPaths, pastedUrl } from "./paste.ts";

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

test("files a file manager copied are their paths, decoded", () => {
  expect(pastedPaths("file:///home/me/a%20b.png\nfile:///home/me/c.pdf\n")).toEqual(["/home/me/a b.png", "/home/me/c.pdf"]);
  expect(pastedPaths("/home/me/notes.md")).toEqual(["/home/me/notes.md"]);
  expect(pastedPaths("/home/me/a.png\nnot a path")).toBeNull();
  expect(pastedPaths("hello")).toBeNull();
});

test("short text is a sticky note, longer text a note named after its first line", () => {
  expect(pasted("Water on Sundays")).toEqual({ kind: "sticky", text: "Water on Sundays" });
  const long = `# Spring planting\n\n${"The beds get compost in March. ".repeat(12).trim()}`;
  expect(pasted(long)).toEqual({ kind: "note", title: "Spring planting", body: long });
  const manyLines = "a\nb\nc\nd\ne\nf\ng";
  expect(pasted(manyLines)).toEqual({ kind: "note", title: "a", body: manyLines });
});

test("a paste is a link or files before it is text, and nothing when empty", () => {
  expect(pasted(" https://example.com ")).toEqual({ kind: "link", url: "https://example.com" });
  expect(pasted("file:///tmp/a.png")).toEqual({ kind: "files", paths: ["/tmp/a.png"] });
  expect(pasted("  \n ")).toBeNull();
});

test("a pasted image is named after when it was pasted", () => {
  expect(pastedImageName(new Date(2026, 9, 31, 16, 4), "image/png")).toBe("Pasted image 2026-10-31 16-04.png");
  expect(pastedImageName(new Date(2026, 9, 31, 16, 4), "image/jpeg")).toBe("Pasted image 2026-10-31 16-04.jpg");
});
