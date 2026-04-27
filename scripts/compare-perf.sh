#!/usr/bin/env bash
set -euo pipefail

BASELINE="${1:-bench-baselines/main.jsonl}"
CURRENT="${2:-rust/target/perf-report.jsonl}"
MODE="${3:-advisory}"
THRESHOLD="${PERF_DEGRADE_THRESHOLD_PCT:-20}"

if [[ ! -f "$BASELINE" ]]; then
  echo "[compare-perf] baseline not found: $BASELINE" >&2
  exit 0
fi

if [[ ! -f "$CURRENT" ]]; then
  echo "[compare-perf] current report not found: $CURRENT" >&2
  exit 0
fi

python3 - "$BASELINE" "$CURRENT" "$MODE" "$THRESHOLD" <<'PY'
import json
import sys
from pathlib import Path

baseline_path = Path(sys.argv[1])
current_path = Path(sys.argv[2])
mode = sys.argv[3].strip().lower()
threshold_pct = float(sys.argv[4])

def load(path: Path):
    rows = {}
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        obj = json.loads(line)
        label = obj.get("label")
        if not label:
            continue
        rows[label] = obj
    return rows

base = load(baseline_path)
curr = load(current_path)

if not base or not curr:
    print("[compare-perf] no comparable rows found")
    raise SystemExit(0)

regressions = []
for label, old in base.items():
    if label not in curr:
        continue
    new = curr[label]
    old_ms = float(old.get("elapsed_ms", 0.0))
    new_ms = float(new.get("elapsed_ms", 0.0))
    if old_ms <= 0:
        continue
    delta_pct = ((new_ms - old_ms) / old_ms) * 100.0
    if delta_pct >= threshold_pct:
        regressions.append((label, old_ms, new_ms, delta_pct))

if regressions:
    print(f"[compare-perf] detected >= {threshold_pct:.1f}% regressions:")
    for label, old_ms, new_ms, delta_pct in regressions:
        print(f"  - {label}: {old_ms:.2f}ms -> {new_ms:.2f}ms ({delta_pct:+.2f}%)")
    if mode == "strict":
        raise SystemExit(1)
    print("[compare-perf] advisory mode: not failing build")
else:
    print("[compare-perf] no regressions over threshold")
PY
