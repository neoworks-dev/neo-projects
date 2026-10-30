import { expect, test } from "bun:test";
import type { Tokens } from "marked";
import { imageLink, imageSources, images, links, onlyImages, parse, resolveSrc } from "./parse.ts";

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

test("reads Obsidian's embeds as images", () => {
  const paragraph = parse("![[My shot.png]] and ![[b.jpg|The garden]]")[0] as Tokens.Paragraph;
  expect(images(paragraph.tokens).map((i) => [i.href, i.text])).toEqual([
    ["My shot.png", ""],
    ["b.jpg", "The garden"],
  ]);
  expect(onlyImages((parse("![[a.png]] ![[b.png]]")[0] as Tokens.Paragraph).tokens)?.length).toBe(2);
});

test("finds the images of a paragraph that has text too", () => {
  const paragraph = parse("Before **![a](a.png)** after ![b](b.png)")[0] as Tokens.Paragraph;
  expect(images(paragraph.tokens).map((i) => i.href)).toEqual(["a.png", "b.png"]);
});

test("looks for a relative image in the note's folder, then the workspace", () => {
  expect(imageSources("a.png", "/w/Notes", "/w")).toEqual(["/w/Notes/a.png", "/w/a.png"]);
  expect(imageSources("a.png", "/w", "/w")).toEqual(["/w/a.png"]);
  expect(imageSources("a.png", null, "/w")).toEqual(["/w/a.png"]);
  expect(imageSources("a.png", null, null)).toEqual(["a.png"]);
  expect(imageSources("https://x.y/a.png", "/w/Notes", "/w")).toEqual(["https://x.y/a.png"]);
});

test("links a dropped image by a name that survives spaces", () => {
  expect(imageLink("My shot 2.png")).toBe("![](My%20shot%202.png)");
  const paragraph = parse(imageLink("My shot 2.png"))[0] as Tokens.Paragraph;
  expect(resolveSrc(images(paragraph.tokens)[0].href, "/w")).toBe("/w/My shot 2.png");
});
