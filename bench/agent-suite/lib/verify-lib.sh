#!/usr/bin/env bash
# Shared verification library for the agent benchmark suite.
#
# A task's verify.sh sources this file and calls `run_task <cases.tsv>
# <solution.ajisai>`. For each case the harness builds a program by
# concatenating the candidate solution and the case's invocation, runs it
# through `ajisai run --json`, extracts one observable, and compares it to
# the expected value. Pass/fail is fully mechanical — no human judgement.
#
# This file contains no task answers; it only runs candidate solutions and
# checks their observable behavior against each task's recorded cases.
set -uo pipefail

# Resolve the ajisai CLI: explicit AJISAI_BIN wins, else the repo debug
# build, building it once if missing.
resolve_ajisai_bin() {
  if [[ -n "${AJISAI_BIN:-}" ]]; then
    [[ -x "$AJISAI_BIN" ]] || { echo "AJISAI_BIN not executable: $AJISAI_BIN" >&2; exit 3; }
    echo "$AJISAI_BIN"; return
  fi
  local root debug
  root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
  debug="$root/rust/target/debug/ajisai"
  if [[ ! -x "$debug" ]]; then
    echo "[verify] building ajisai CLI..." >&2
    ( cd "$root/rust" && cargo build --bin ajisai >&2 ) || { echo "build failed" >&2; exit 3; }
  fi
  echo "$debug"
}

# cases.tsv columns (tab-separated, '#' comment lines ignored):
#   id   invocation   expect_kind   expect_value
# expect_kind:
#   stack    - stackDisplay joined by a single space == expect_value
#   output   - output joined by ' | ' == expect_value
#   status   - top-level status ("ok"/"error") == expect_value
#   errorWhy - diagnosis.why == expect_value
#   scoreLE  - runtimeMetrics.vtu.energyProxyScore <= int(expect_value)
# An invocation of "-" runs the solution program by itself (tab is an
# IFS-whitespace char, so a literally empty column cannot be represented).
run_task() {
  local cases_file="$1" solution_file="$2"
  local bin; bin="$(resolve_ajisai_bin)"
  [[ -f "$cases_file" ]] || { echo "missing cases file: $cases_file" >&2; exit 3; }
  [[ -f "$solution_file" ]] || { echo "missing solution file: $solution_file" >&2; exit 3; }

  local solution; solution="$(cat "$solution_file")"
  local total=0 passed=0
  local tmp; tmp="$(mktemp)"; trap 'rm -f "$tmp"' RETURN

  while IFS=$'\t' read -r id invocation kind expected || [[ -n "$id" ]]; do
    [[ -z "$id" || "$id" == \#* ]] && continue
    total=$((total + 1))
    [[ "$invocation" == "-" ]] && invocation=""
    printf '%s\n%s\n' "$solution" "$invocation" > "$tmp"
    local out actual
    out="$("$bin" run "$tmp" --json 2>/dev/null)"
    actual="$(printf '%s' "$out" | EXPECT_KIND="$kind" python3 -c '
import json, os, sys
kind = os.environ["EXPECT_KIND"]
try:
    d = json.load(sys.stdin)
except Exception:
    print("<invalid-json>"); sys.exit(0)
if kind == "stack":
    print(" ".join(d.get("stackDisplay") or []))
elif kind == "output":
    print(" | ".join(d.get("output") or []))
elif kind == "status":
    print(d.get("status", "<none>"))
elif kind == "errorWhy":
    print((d.get("diagnosis") or {}).get("why", "<none>"))
elif kind == "scoreLE":
    print(((d.get("runtimeMetrics") or {}).get("vtu") or {}).get("energyProxyScore", "<none>"))
else:
    print("<unknown-kind:%s>" % kind)
')"
    local ok=0
    if [[ "$kind" == "scoreLE" ]]; then
      [[ "$actual" =~ ^[0-9]+$ && "$actual" -le "$expected" ]] && ok=1
    else
      [[ "$actual" == "$expected" ]] && ok=1
    fi
    if [[ "$ok" == 1 ]]; then
      passed=$((passed + 1))
      printf 'PASS  %-22s %s\n' "$id" "$kind"
    else
      printf 'FAIL  %-22s %s\n      expected: %s\n      actual:   %s\n' \
        "$id" "$kind" "$expected" "$actual"
    fi
  done < "$cases_file"

  echo "----"
  echo "$passed/$total cases passed"
  [[ "$passed" -eq "$total" ]]
}
