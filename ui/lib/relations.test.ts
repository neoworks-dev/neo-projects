import { describe, expect, test } from "bun:test";
import { relationOptions } from "./relations.ts";

const github = { local: false };
const local = { local: true };
const types = (options: { type: string }[]) => options.map((o) => o.type);

describe("relationOptions", () => {
  test("two GitHub issues can have GitHub's relationships", () => {
    const options = relationOptions([github], github);
    expect(types(options)).toEqual(["blocks", "parent-of", "relates"]);
    expect(options[0].where).toBe("On GitHub");
  });

  test("a note and an issue have the workspace's relationships", () => {
    expect(types(relationOptions([null], github))).toEqual(["blocks", "derived-from", "relates"]);
  });

  test("several issues can't all be one issue's parent", () => {
    expect(types(relationOptions([github, github], github))).toEqual(["blocks", "relates"]);
  });

  test("a mixed selection keeps only what every pair can have", () => {
    const options = relationOptions([github, null], github);
    expect(types(options)).toEqual(["blocks", "relates"]);
    expect(options[0].where).toBe("Kept where each pair keeps it");
    expect(options[1].where).toBe("Kept in relations.jsonl");
  });

  test("local and GitHub issues together have the workspace's relationships", () => {
    expect(types(relationOptions([local], github))).toEqual(["blocks", "derived-from", "relates"]);
  });
});
