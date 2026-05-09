#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

failed=0

check_absent() {
  local description="$1"
  local pattern="$2"
  shift 2
  echo "[semantic-firewall] checking: ${description}"
  if rg -n --color never "$pattern" "$@"; then
    echo "[semantic-firewall] FAIL: ${description}" >&2
    failed=1
  fi
}

# External payloads must not reintroduce legacy camelCase fields.
check_absent "legacy nilReason external field" 'nilReason' rust/src src
check_absent "legacy top-level errorCategory external field" 'errorCategory' rust/src src

# Machine-readable WASM/TS/AI-facing outputs must use protocol strings,
# not Rust Debug formatting.
check_absent \
  'Debug formatting in external protocol payload code' \
  'format!\("\{:\?' \
  rust/src/wasm-interpreter-state.rs rust/src/interpreter/debug-diagnosis.rs src

# TypeScript and the WASM boundary must not depend on Rust Debug variant names.
check_absent \
  'Rust Debug-derived protocol literals in TS/WASM boundary' \
  'DivisionByZero|SafeCaught|ExecuteWord|ParseStructure|ResolveWord' \
  src rust/src/wasm-interpreter-state.rs

if [[ "$failed" -ne 0 ]]; then
  echo "[semantic-firewall] residue checks failed" >&2
  exit 1
fi

echo "[semantic-firewall] residue checks passed"
