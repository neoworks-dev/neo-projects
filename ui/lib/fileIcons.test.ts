import { expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { fileIcon } from "./fileIcons.ts";

test("a file is known by its name first, then its longest extension", () => {
  expect(fileIcon("package.json")).toBe("file-icons/nodejs.svg");
  expect(fileIcon("README.md")).toBe("file-icons/readme.svg");
  expect(fileIcon("geometry.test.ts")).toBe("file-icons/test-ts.svg");
  expect(fileIcon("geometry.ts")).toBe("file-icons/typescript.svg");
  expect(fileIcon("Report.PDF")).toBe("file-icons/pdf.svg");
});

test("files nobody knows get the plain icon, and our own get theirs", () => {
  expect(fileIcon("notes")).toBe("file-icons/file.svg");
  expect(fileIcon("data.xyz123")).toBe("file-icons/file.svg");
  expect(fileIcon("Garden.canvas")).toBe("file-icons/excalidraw.svg");
});

test("folders are known by name, open or closed", () => {
  expect(fileIcon("src", true)).toBe("file-icons/folder-src.svg");
  expect(fileIcon("src", true, true)).toBe("file-icons/folder-src-open.svg");
  expect(fileIcon("Garden", true)).toBe("file-icons/folder.svg");
});

test("every icon named exists in the theme", () => {
  const names = ["package.json", "a.test.ts", "x.pdf", "a.canvas", "a.board", "notes", "src"];
  for (const name of names) {
    const icon = fileIcon(name, name === "src").replace("file-icons/", "");
    expect(existsSync(`node_modules/material-icon-theme/icons/${icon}`)).toBe(true);
  }
});
