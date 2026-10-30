// Bundles the UI for the svelte-gpui host. svelte-gpui and extension-system are checked out next
// to this repo. `bun build.mjs --watch` rebuilds on every save (run the binary with --watch too).
import { fileURLToPath } from "node:url";
import { build } from "../svelte-gpui/build.mjs";

const here = (path) => fileURLToPath(new URL(path, import.meta.url));

await build({
  entry: here("./ui/main.ts"),
  outfile: here("./dist/app.js"),
  watch: process.argv.includes("--watch"),
  alias: { "@neoworks/extension-system": here("../extension-system/src/index.ts") },
});
