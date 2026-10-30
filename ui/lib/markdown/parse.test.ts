import { expect, test } from "bun:test";
import type { Tokens } from "marked";
import { links, onlyImages, parse, resolveSrc } from "./parse.ts";

test("parses GitHub's extensions", () => {
  const tokens = parse("| a | b |\n|---|---|\n| 1 | 2 |\n\n- [x] done\n- [ ] todo\n\n~~gone~~");
  expect(tokens.map((t) => t.type).filter((t) => t !== "space")).toEqual(["table", "list", "paragraph"]);
  const list = tokens.find((t) => t.type === "list") as Tokens.List;
  expect(list.items.map((i) => [i.task, i.checked])).toEqual([
    [true, true],
    [true, false],
  ]);
});

test("finds links in nested inline tokens", () => {
  const paragraph = parse("See **[the docs](https://a.b/docs)** and <https://c.d>.")[0] as Tokens.Paragraph;
  expect(links(paragraph.tokens)).toEqual(["https://a.b/docs", "https://c.d"]);
});

test("a paragraph of images only is a gallery", () => {
  const gallery = parse("![a](a.png) ![b](b.png)")[0] as Tokens.Paragraph;
  expect(onlyImages(gallery.tokens)?.map((i) => i.href)).toEqual(["a.png", "b.png"]);
  const mixed = parse("Look: ![a](a.png)")[0] as Tokens.Paragraph;
  expect(onlyImages(mixed.tokens)).toBeNull();
});

test("resolves image paths against the note's folder", () => {
  expect(resolveSrc("https://x.y/a.png", "/w/Notes")).toBe("https://x.y/a.png");
  expect(resolveSrc("/abs/a.png", "/w/Notes")).toBe("/abs/a.png");
  expect(resolveSrc("img/My%20shot.png", "/w/Notes")).toBe("/w/Notes/img/My shot.png");
  expect(resolveSrc("../a.png", "/w/Notes/")).toBe("/w/a.png");
});
