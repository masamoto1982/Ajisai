#!/usr/bin/env bash
# Usage: bash scripts/update-bench-baseline.sh
set -euo pipefail
cd "$(dirname "$0")/.."
cargo bench 2>&1 | tee benchmark-baseline.txt
echo "Baseline updated: benchmark-baseline.txt"
