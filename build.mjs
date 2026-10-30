// Bundles the UI for the svelte-gpui host. svelte-gpui and extension-system are checked out next
// to this repo. `bun build.mjs --watch` rebuilds on every save (run the binary with --watch too).
import { existsSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import * as esbuild from "../svelte-gpui/node_modules/esbuild/lib/main.js";
import { build } from "../svelte-gpui/build.mjs";

const here = (path) => fileURLToPath(new URL(path, import.meta.url));

// MathJax for the math worker, a script of its own beside the UI. It only changes with MathJax,
// so it is bundled when the lockfile is newer than it.
const mathjax = here("./dist/mathjax.js");
if (!existsSync(mathjax) || statSync(here("./bun.lock")).mtimeMs > statSync(mathjax).mtimeMs) {
  await esbuild.build({
    entryPoints: [here("./math/mathjax.js")],
    outfile: mathjax,
    bundle: true,
    format: "iife",
    target: "es2020",
    minify: true,
    logLevel: "warning",
  });
}

await build({
  entry: here("./ui/main.ts"),
  outfile: here("./dist/app.js"),
  watch: process.argv.includes("--watch"),
  alias: { "@neoworks/extension-system": here("../extension-system/src/index.ts") },
});
