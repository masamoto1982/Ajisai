# Unfused-route profile

Where the time goes on routes the fused walk declines. Every script reads the
committed bundle in `src/wasm/generated/` unless told otherwise; none of them
changes the source tree.

| Step | Re-run (from the repo root) |
|---|---|
| 1. Factor microbenchmarks (A–GP × 1e3/1e4/1e5, `AjisaiInterpreter.execute` and `bench_execute`, result checked against a fused reference) | `node scripts/bench/unfused/breakdown.mjs --json /tmp/step1.json` |
| 2. Per-element CPU profile (large n minus small n, so the per-run constant cancels); all cases at once: `bash scripts/bench/unfused/profile-all.sh /tmp/ajisai-prof` | `for n in 100000 100; do node --cpu-prof --cpu-prof-dir=/tmp/prof-B-$n --cpu-prof-interval 50 scripts/bench/unfused/profile-case.mjs B $n 10; done` then `node scripts/bench/unfused/diff-profile.mjs /tmp/prof-B-100000/*.cpuprofile <runs> 100000 /tmp/prof-B-100/*.cpuprofile <runs> 100` (`<runs>` is what profile-case printed) |
| 2b. Name the anonymous `wasm-function[N]` | `node scripts/bench/unfused/wasm-inspect.mjs src/wasm/generated/ajisai_core_bg.wasm 66 328 809` (source locations from panic `Location`s, allocator chain, callees); `disasm.mjs` prints one body |
| 2c. File functions under factors | `node scripts/bench/unfused/categorize.mjs perfn.json` (map in `categories.mjs`; indices are for the bundle at 7edd4c0) |
| 3. Allocations and calls per element | `node scripts/bench/unfused/instrument-wasm.mjs src/wasm/generated /tmp/counted && node scripts/bench/unfused/count-calls.mjs /tmp/counted --n 10000` |

`cases.mjs` holds the cases. `instrument-wasm.mjs` writes a call-counting copy
of the module (one exported i32 global per function, bumped on entry); its
timings are meaningless and it is never committed.
