#!/usr/bin/env bash
# Step 2 end to end: profile each case at n=100000 and n=100, subtract the
# setup-only profile, write <out>/perfn.json and print the factor table.
#   bash scripts/bench/unfused/profile-all.sh /tmp/ajisai-prof
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"; out="${1:?output dir}"; mkdir -p "$out"; : > "$out/runs.txt"
for c in B C K F D E G BB B@setup D@setup E@setup; do
  rl=$(node --cpu-prof --cpu-prof-dir="$out/$c-L" --cpu-prof-interval 50 "$here/profile-case.mjs" "$c" 100000 10 2>/dev/null)
  rs=$(node --cpu-prof --cpu-prof-dir="$out/$c-S" --cpu-prof-interval 50 "$here/profile-case.mjs" "$c" 100 10 2>/dev/null)
  echo "$c $rl $rs" >> "$out/runs.txt"
done
node "$here/build-perfn.mjs" "$out"
node "$here/categorize.mjs" "$out/perfn.json"
