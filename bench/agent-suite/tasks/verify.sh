#!/usr/bin/env bash
# Per-task verification entry point.
#
# Usage:
#   verify.sh <task-name> <solution.ajisai>
#   verify.sh <solution.ajisai>            # when run as tasks/<task>/... ; not used here
#
# <task-name> is the base name of a spec in this directory (e.g.
# "exact-rational-calculator"); its machine-readable cases live alongside as
# "<task-name>.cases.tsv". Exit 0 iff every case passes.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../lib/verify-lib.sh
source "$here/../lib/verify-lib.sh"

if [[ $# -ne 2 ]]; then
  echo "usage: verify.sh <task-name> <solution.ajisai>" >&2
  echo "tasks:" >&2
  for f in "$here"/*.cases.tsv; do
    [[ -e "$f" ]] || continue
    echo "  - $(basename "${f%.cases.tsv}")" >&2
  done
  exit 2
fi

task="$1"
solution="$2"
cases="$here/${task}.cases.tsv"
if [[ ! -f "$cases" ]]; then
  echo "unknown task '$task' (no $cases)" >&2
  exit 2
fi

echo "== verifying task '$task' with solution '$solution' =="
run_task "$cases" "$solution"
