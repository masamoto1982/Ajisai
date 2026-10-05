# Fusion statistics

How often a MAP / FILTER / FOLD / SCAN block takes the fused route
(`rust/src/interpreter/fused_block.rs`) in the programs the repository
contains, and what stops the ones that do not. Nothing here changes the source
tree or the committed bundle.

Re-run: `bash scripts/bench/fusion-stats/run-all.sh /tmp/fusion-stats` (tables in `/tmp/fusion-stats/tables.md`).

| File | Role |
|---|---|
| `collect-corpus.mjs` | Step 1: programs from tests, docs, MCP corpora, bench cases and `business.mjs` → `corpus.json` (kind: test / example / bench / business) |
| `probe-wasm.mjs` | Copies the bundle and adds an import `probe.ev` called on entry/exit of chosen functions (indices renumbered) |
| `trace.mjs` | Step 2, runtime: every higher-order call — Word, top-level position, enclosing call, whether the lowering succeeded (`FusedBlock::run` entered), whether the walk was fused (its result), target/seed shape, elements |
| `static.mjs` | Step 2, static: every block and what `fused_block_lower.rs` would say about each op — all the causes, not only the first |
| `stats.mjs`, `report.mjs` | Step 3/4: link calls to blocks, aggregate, write `tables.md` |
| `selftest.mjs` | Checks the probe points against programs whose route is known (indices/offsets are specific to the bundle at 7edd4c0) |
