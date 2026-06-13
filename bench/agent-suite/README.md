# Agent benchmark suite

A self-hostable reproduction of the "hand an agent a spec, watch it write →
run → fix on the CLI" benchmark used by external AI-language comparisons,
specialized to Ajisai. It exists to produce *honest* head-to-head material —
including the cases where Ajisai loses (see `protocol.md` §5).

## Layout

```
bench/agent-suite/
  README.md            this file
  protocol.md          how to run a measurement (conditions, metrics, rules)
  lib/verify-lib.sh    shared, task-agnostic verification engine
  tasks/
    verify.sh          entry point: verify.sh <task> <solution.ajisai>
    <task>.md          language-independent spec + solution contract + cases
    <task>.cases.tsv   machine-readable acceptance cases for that task
  results/
    TEMPLATE.md        copy per (date, model, condition) and fill in
```

## Tasks

Two ported tasks (for comparability with external write-ups) and four
Ajisai-favorable tasks (the verification × low-movement quadrant Ajisai aims
to own):

- `json-parser` — compose the JSON module to parse/query/transform/serialize.
- `bank-account` — functional balance with overdraft rejection via NIL.
- `exact-rational-calculator` — exact rational arithmetic, no float drift.
- `three-valued-logic` — the UNKNOWN truth value and Kleene connectives.
- `nil-fallback-pipeline` — NIL bubbling and `^` fallback.
- `energy-refactor` — preserve output while lowering `energyProxyScore`.

## Requirements

`bash`, `python3` (for JSON extraction in the verifier), and the Rust
toolchain (to build the `ajisai` CLI). All three are present on the CI image
and typical dev machines; the suite itself is not run in CI (it is driven by
independent agent sessions per `protocol.md`).

## Running a verification

```sh
# build once (verify.sh will also do this if needed):
cargo build --bin ajisai --manifest-path rust/Cargo.toml

# verify a candidate solution against a task's cases:
bench/agent-suite/tasks/verify.sh exact-rational-calculator my-solution.ajisai
```

Exit code 0 means every acceptance case passed; non-zero means at least one
failed (or a usage/setup error). `AJISAI_BIN=/path/to/ajisai` overrides which
binary is used.

## How verification works

Each task ships a `.cases.tsv` (tab-separated: `id`, `invocation`,
`expect_kind`, `expect_value`). For every case the engine concatenates the
candidate solution with the case's invocation, runs it through
`ajisai run --json`, extracts one observable (final stack display, PRINT
output, status, diagnosis `why`, or `energyProxyScore`), and compares it to
the expected value. The verdict is purely mechanical — no human judgement of
correctness (`protocol.md` §0).

Reference/answer solutions are intentionally **not** committed here; shipping
them would void the benchmark. The harness is validated by confirming each
`verify.sh` returns 0 on a known-correct solution and non-zero on a wrong one
(done off-tree by the maintainer).
