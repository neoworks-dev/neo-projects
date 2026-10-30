#!/usr/bin/env bash
# Builds the UI and the window, then runs it. svelte-gpui and extension-system are checked out
# next to this repository. NP_WORKSPACE opens a workspace folder; NP_GH_USER picks the gh account.
#
#   scripts/run.sh            build and run
#   scripts/run.sh --watch    rebuild the UI on every save and reload the window
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
if [ ! -d target/release/deps ] && [ -d ../svelte-gpui/target/release/deps ]; then
  # GPUI takes long to compile: start from svelte-gpui's build of it (a reflink on btrfs).
  mkdir -p target && cp -r --reflink=auto ../svelte-gpui/target/release target/
  rm -rf target/release/.fingerprint/svelte_gpui_host-* target/release/build/svelte_gpui_host-*
fi
cargo build --release -p neoworks-projects
if [ "${1:-}" = "--watch" ]; then
  bun build.mjs --watch &
  trap 'kill %1' EXIT
  sleep 1
  target/release/neoworks-projects --bundle dist/app.js --watch
else
  bun build.mjs
  target/release/neoworks-projects --bundle dist/app.js
fi
