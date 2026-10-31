import { expect, test } from "bun:test";
import { folderSteps } from "./folders.ts";

test("the project's own canvas has no folder steps", () => {
  expect(folderSteps("Garden", undefined)).toEqual([]);
});

test("each folder open within the project is a step with its directory", () => {
  expect(folderSteps("Work/Garden", "Work/Garden/Seeds/Tomatoes")).toEqual([
    { name: "Seeds", path: "Work/Garden/Seeds" },
    { name: "Tomatoes", path: "Work/Garden/Seeds/Tomatoes" },
  ]);
});
