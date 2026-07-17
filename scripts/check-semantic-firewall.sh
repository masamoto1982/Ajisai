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

# External payloads must not expose disallowed camelCase fields.
check_absent "nilReason external field" 'nilReason' rust/src src
check_absent "top-level errorCategory external field" 'errorCategory' rust/src src

# Machine-readable WASM/TS/AI-facing outputs must use protocol strings,
# not Rust Debug formatting.
check_absent \
  'Debug formatting in external protocol payload code' \
  'format!\("\{:\?' \
  rust/src/wasm_interpreter_bindings/wasm_interpreter_state.rs rust/src/interpreter/debug_diagnosis.rs src

# TypeScript and the WASM boundary must not depend on Rust Debug variant names.
check_absent \
  'Rust Debug-derived protocol literals in TS/WASM boundary' \
  'DivisionByZero|SafeCaught|ExecuteWord|ParseStructure|ResolveWord' \
  src rust/src/wasm_interpreter_bindings/wasm_interpreter_state.rs

# ── Internal-vocabulary firewall over user-visible strings ────────────────
# Two-tier disclosure (docs/dev/user-surface-information-hiding.md, S2):
# language users see optimizations only as speed, so error text, NIL
# diagnostics, LOOKUP/hover content, and GUI labels must never name a
# routing or optimization mechanism. Builder/AI channels are exempt:
#   - tests and benches (globs below)
#   - rust/src/elastic/            trace + hedged engine (feature-gated)
#   - rust/src/cli/                agent-facing --json contract / explain
#   - wasm_interpreter_bindings/   machine protocol keys (metric names)
#   - trace eprintln ("[trace-*", "[hedged]"), panics/expect, cfg attrs
# The pattern is applied to string literals only, so code comments stay free
# to name mechanisms for builders.
check_user_visible_absent() {
  local description="$1"
  local pattern="$2"
  echo "[semantic-firewall] checking: user-visible ${description}"
  if rg -n --color never "\"[^\"]*(${pattern})[^\"]*\"" \
      rust/src src \
      -g '!*test*' -g '!**/elastic/**' -g '!**/cli/**' \
      -g '!**/wasm_interpreter_bindings/**' -g '!**/benches/**' \
    | rg -v '\.expect\(|eprintln!|\[trace-|debug_assert|panic!|#\[cfg\(feature'
  then
    echo "[semantic-firewall] FAIL: user-visible ${description}" >&2
    failed=1
  fi
}

check_user_visible_absent 'fast-kernel vocabulary' '[Ff]ast kernel'
check_user_visible_absent 'fast-path vocabulary' '[Ff]ast.?[Pp]ath'
check_user_visible_absent 'quantized-block vocabulary' '[Qq]uantized block'
check_user_visible_absent 'compiled-plan vocabulary' '[Cc]ompiled plan'
check_user_visible_absent 'inline-cache vocabulary' '[Ii]nline cache|shape IC'
check_user_visible_absent 'hedged-engine vocabulary' '[Hh]edged'
check_user_visible_absent 'epoch vocabulary' '[Ee]poch'
check_user_visible_absent 'internal roadmap phase numbers' '[Pp]hase [0-9]'
check_user_visible_absent 'memoization vocabulary' '[Mm]emoiz'

if [[ "$failed" -ne 0 ]]; then
  echo "[semantic-firewall] residue checks failed" >&2
  exit 1
fi

echo "[semantic-firewall] residue checks passed"
