#!/usr/bin/env bash
# Rebuild the wasm bundle that ships in src/wasm/generated/.
#
# wasm-pack writes a publishable npm package (with package.json, README, etc.)
# into its --out-dir, but the runtime only consumes four files:
#   ajisai_core.js, ajisai_core.d.ts,
#   ajisai_core_bg.wasm, ajisai_core_bg.wasm.d.ts
#
# We build into a scratch directory and copy just those four files so the
# committed tree stays clean.
#
# --no-opt is intentional: wasm-opt 108 (the version available in the build
# environment) miscompiles wasm-bindgen 0.2.120 output and produces a
# corrupted module (see commit 89a0c7b). Skipping it costs ~6% binary size
# but keeps the runtime functional.
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
out_dir="${repo_root}/src/wasm/generated"
scratch_dir="$(mktemp -d)"
trap 'rm -rf "${scratch_dir}"' EXIT

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "rebuild-wasm: wasm-pack is not installed." >&2
  echo "  Install with:  bash ./scripts/install-wasm-pack.sh" >&2
  exit 1
fi

cd "${repo_root}/rust"
# The WASM/JS bindings are gated behind the `wasm` Cargo feature so the native
# Core build never pulls in wasm-bindgen. wasm-pack passes args after `--`
# straight to cargo.
wasm-pack build --target web --out-dir "${scratch_dir}" --no-opt -- --features wasm

mkdir -p "${out_dir}"
for f in ajisai_core.js ajisai_core.d.ts ajisai_core_bg.wasm ajisai_core_bg.wasm.d.ts; do
  cp "${scratch_dir}/${f}" "${out_dir}/${f}"
done

echo "rebuild-wasm: refreshed ${out_dir}"
