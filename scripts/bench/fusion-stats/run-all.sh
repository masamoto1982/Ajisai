#!/usr/bin/env bash
# Fusion statistics end to end:  bash scripts/bench/fusion-stats/run-all.sh /tmp/fusion-stats
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"; out="${1:?output dir}"; mkdir -p "$out"
node "$here/selftest.mjs"
node "$here/collect-corpus.mjs" "$out/corpus.json" > "$out/corpus-list.tsv"
node "$here/trace.mjs" "$out/corpus.json" "$out/trace.json"
node "$here/stats.mjs" "$out/corpus.json" "$out/trace.json" "$out"
node "$here/report.mjs" "$out" > /dev/null
echo "tables: $out/tables.md"
