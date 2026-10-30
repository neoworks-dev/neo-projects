import { expect, test } from "bun:test";
import { plainText } from "./plain.ts";

test("drops the syntax and keeps the words", () => {
  const md = [
    "# Plan",
    "",
    "Some **bold**, _soft_ and `code` with a [link](https://x.y) and [[Other note|alias]].",
    "- [ ] a task",
    "1. first",
    "> quoted",
    "![shot](a.png) ![[b.png]]",
    "---",
    "| a | b |",
    "|---|---|",
    "```ts",
    "const hidden = 1;",
    "```",
    "<br>end",
  ].join("\n");
  expect(plainText(md)).toBe("Plan Some bold, soft and code with a link and alias. a task first quoted end");
});

test("cuts long text with an ellipsis", () => {
  expect(plainText("word ".repeat(100), 20)).toBe("word word word word…");
});

test("empty stays empty", () => {
  expect(plainText("\n\n```\nonly code\n```\n")).toBe("");
});
