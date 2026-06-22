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
# wasm-opt remains opt-in for now: wasm-opt 108 (the version available in an
# earlier build environment) miscompiled wasm-bindgen 0.2.120 output and
# produced a corrupted module (see commit 89a0c7b). The Cargo profile below is
# speed-oriented (-O3), but the stable default still passes --no-opt until the
# optimized path has a pinned Binaryen version plus smoke/benchmark evidence.
# Set AJISAI_WASM_OPT=1 to exercise the speed-optimized profile.
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
wasm_pack_args=(build --target web --out-dir "${scratch_dir}")
if [[ "${AJISAI_WASM_OPT:-0}" == "1" ]]; then
  if command -v wasm-opt >/dev/null 2>&1; then
    echo "rebuild-wasm: AJISAI_WASM_OPT=1; using wasm-pack release wasm-opt profile ($(wasm-opt --version))."
  else
    echo "rebuild-wasm: AJISAI_WASM_OPT=1 but wasm-opt is not on PATH; wasm-pack may fail." >&2
  fi
else
  echo "rebuild-wasm: using stable no-opt wasm build; set AJISAI_WASM_OPT=1 to enable wasm-opt -O3."
  wasm_pack_args+=(--no-opt)
fi
wasm-pack "${wasm_pack_args[@]}" -- --features wasm

mkdir -p "${out_dir}"
for f in ajisai_core.js ajisai_core.d.ts ajisai_core_bg.wasm ajisai_core_bg.wasm.d.ts; do
  cp "${scratch_dir}/${f}" "${out_dir}/${f}"
done

echo "rebuild-wasm: refreshed ${out_dir}"
