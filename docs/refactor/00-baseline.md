# Task 0 Baseline (Ajisai Refactor Instructions)

Status: generated for Task 0 of the refactor instructions
Branch: `claude/ajisai-task-instructions-U7HHN`
Date: 2026-04-21

This document captures the factual state of the repository at the start of the refactor.
It is descriptive, not prescriptive. SPECIFICATION.md remains the canonical authority.

---

## 1. Top-level `rust/src/` layout

Output equivalent to `tree -L 2 rust/src` (alphabetised for reproducibility).

```
rust/src/
├── arithmetic-operation-tests.rs
├── builtins/
│   ├── builtin-word-definitions.rs
│   ├── builtin-word-details.rs
│   ├── detail-lookup-arithmetic-logic.rs
│   ├── detail-lookup-cond.rs
│   ├── detail-lookup-control-higher-order.rs
│   ├── detail-lookup-io-module.rs
│   ├── detail-lookup-modifier.rs
│   ├── detail-lookup-string-cast.rs
│   ├── detail-lookup-vector-ops.rs
│   └── mod.rs
├── dimension-limit-tests.rs
├── elastic/
│   ├── cache_manager.rs
│   ├── elastic-engine-tests.rs
│   ├── evaluation_unit.rs
│   ├── execution_mode.rs
│   ├── fallback_bridge.rs
│   ├── hedged_executor.rs
│   ├── hedged_policy.rs
│   ├── hedged_result.rs
│   ├── hedged_snapshot.rs
│   ├── hedged_trace.rs
│   ├── mod.rs
│   ├── purity_table.rs
│   └── tracer.rs
├── error.rs
├── interpreter/
│   ├── arithmetic.rs
│   ├── audio/            (module; see §2 item "modules / builtins registration")
│   ├── cast*.rs
│   ├── child-runtime.rs
│   ├── comparison.rs
│   ├── compiled-plan.rs
│   ├── control.rs
│   ├── control-cond.rs
│   ├── datetime.rs
│   ├── epoch.rs
│   ├── execute-builtin.rs
│   ├── execute-def.rs
│   ├── execute-del.rs
│   ├── execute-lookup.rs
│   ├── execution-loop.rs
│   ├── execution_plan_set.rs
│   ├── hash.rs
│   ├── higher-order-operations.rs
│   ├── higher-order-fold-operations.rs
│   ├── interpreter-core.rs
│   ├── interval_ops.rs
│   ├── io.rs
│   ├── json.rs
│   ├── logic.rs
│   ├── mod.rs
│   ├── modules.rs
│   ├── naming-convention-checker.rs
│   ├── optimization-hooks.rs
│   ├── quantized-block.rs
│   ├── random.rs
│   ├── redundancy-budget.rs
│   ├── redundancy-layer.rs
│   ├── resolve-cache.rs
│   ├── resolve-word.rs
│   ├── shadow-validation.rs
│   ├── simd-vector-operations.rs
│   ├── sort.rs
│   ├── tensor-shape-commands.rs
│   ├── tensor-shape-operations.rs
│   ├── value-extraction-helpers.rs
│   ├── vector-execution-operations.rs
│   └── vector_ops/
│       ├── mod.rs
│       ├── position.rs
│       ├── quantity.rs
│       ├── structure.rs
│       ├── targeting.rs
│       ├── tests.rs
│       └── tests_modes.rs
├── json-io-tests.rs
├── lib.rs
├── tensor-operation-tests.rs
├── tokenizer-regression-tests-2.rs
├── tokenizer-regression-tests.rs
├── tokenizer.rs
├── types/
│   ├── arena.rs
│   ├── display.rs
│   ├── flow-token.rs
│   ├── fraction.rs
│   ├── fraction-arithmetic.rs
│   ├── interval.rs
│   ├── mod.rs
│   └── value-operations.rs
├── wasm-interpreter-bindings.rs
├── wasm-interpreter-execution.rs
├── wasm-interpreter-state.rs
└── wasm-value-conversion.rs
```

Notes:
- File names use hyphens on disk; `mod` declarations use underscores with `#[path = "..."]`.
- Integration tests live in `rust/tests/`.
- This crate is a WASM-targeted `cdylib + rlib` (see §3).

---

## 2. Required artifact presence / location

For each item requested by Task 0, the canonical file(s) are listed below.
Paths are written one-per-line under a `Paths:` block so the verification loop
```
while read path; do test -e "$path" || echo MISSING: $path; done
```
can consume them directly after trimming the leading bullet marker.

### 2.1 Fraction / exact numeric implementation
Present.
Paths:
- rust/src/types/fraction.rs
- rust/src/types/fraction-arithmetic.rs

Additional related:
- rust/src/types/interval.rs
- rust/src/types/value-operations.rs

### 2.2 Vector execution engine
Present.
Paths:
- rust/src/interpreter/vector-execution-operations.rs
- rust/src/interpreter/vector_ops/mod.rs
- rust/src/interpreter/vector_ops/position.rs
- rust/src/interpreter/vector_ops/quantity.rs
- rust/src/interpreter/vector_ops/structure.rs
- rust/src/interpreter/vector_ops/targeting.rs
- rust/src/interpreter/simd-vector-operations.rs

### 2.3 Target mode / consume mode implementation
Present. Both concepts are enums in the interpreter core
(`OperationTargetMode { StackTop, Stack }`,
`ConsumptionMode { Consume, Keep }` — SPECIFICATION.md §6.1/§6.2).
Paths:
- rust/src/interpreter/interpreter-core.rs
- rust/src/interpreter/vector_ops/targeting.rs
- rust/src/interpreter/execute-builtin.rs

Note: these enums describe **stack operand selection and retention**. They are
the spec's §6 "Target / Consume modifiers", not a database transaction concept.
The refactor plan's use of the terms "target mode / consume mode" as
"observation phase / commit phase" is a different concept under the same name;
this potential collision is flagged for Task 1 invariants.

### 2.4 CodeBlock implementation
Present.
Paths:
- rust/src/types/mod.rs

The `CodeBlock(Vec<Token>)` variant is declared in `ValueData` at
rust/src/types/mod.rs:53. Compilation and execution of code blocks are handled
by `compiled-plan.rs` and `quantized-block.rs` (§2.5).

### 2.5 compiled-plan / quantized-block / child runtime
Present.
Paths:
- rust/src/interpreter/compiled-plan.rs
- rust/src/interpreter/compiled-plan-tests.rs
- rust/src/interpreter/quantized-block.rs
- rust/src/interpreter/quantized-block-tests.rs
- rust/src/interpreter/child-runtime.rs
- rust/src/interpreter/child-runtime-tests.rs
- rust/src/interpreter/execution_plan_set.rs

### 2.6 Error type definition
Present.
Paths:
- rust/src/error.rs

`pub enum AjisaiError` at rust/src/error.rs:6. Variants match
SPECIFICATION.md §11 exactly (both 11.1 user-level and 11.2 internal flow-level).

### 2.7 Builtins / modules registration mechanism
Present.
Paths:
- rust/src/builtins/mod.rs
- rust/src/builtins/builtin-word-definitions.rs
- rust/src/builtins/builtin-word-details.rs
- rust/src/interpreter/modules.rs

`fn register_builtins` at rust/src/builtins/mod.rs:29. Module registration
(`MUSIC`, `JSON`, `IO` — matching SPECIFICATION.md §9.1) is driven from
`MODULE_SPECS` at rust/src/interpreter/modules.rs:224.

### 2.8 Interpreter main loop
Present.
Paths:
- rust/src/interpreter/execution-loop.rs
- rust/src/interpreter/interpreter-core.rs
- rust/src/interpreter/execute-builtin.rs
- rust/src/interpreter/resolve-word.rs

---

## 3. `Cargo.toml` dependencies (baseline)

From `rust/Cargo.toml`:

Package:
- name: `ajisai-core`
- version: `0.1.0`
- edition: `2021`
- crate-type: `cdylib`, `rlib`

Runtime dependencies:
- wasm-bindgen = "0.2" (features: serde-serialize)
- js-sys = "0.3"
- serde = "1.0" (features: derive)
- serde-wasm-bindgen = "0.6"
- num-bigint = "0.4" (features: serde)
- num-traits = "0.2"
- num-integer = "0.1"
- serde_json = "1.0"
- wasm-bindgen-futures = "0.4"
- chrono = "0.4" (features: wasmbind)
- getrandom = "0.2" (features: js)
- lazy_static = "1.4"
- smallvec = "1"
- web-sys = "0.3" (features: console, Window, CustomEvent, EventTarget, Event)

Dev-dependencies:
- tokio = "1.0" (features: macros, rt-multi-thread)
- criterion = "0.5" (features: html_reports)

Feature flags: `trace-compile`, `trace-epoch`, `trace-quant`.

No database-backend crates are currently present (no rusqlite, no tokio-postgres, etc.).

---

## 4. `cargo test` baseline

Command: `cargo test` (run from `rust/`).

Result: **PASS**.
- lib tests (`target/debug/deps/ajisai_core-*`): 611 passed, 0 failed, 0 ignored.
- integration `fractional-dataflow-behavior-tests`: 57 passed, 0 failed, 0 ignored.
- doc-tests: 0 tests.
- **Total: 668 tests passed, 0 failed.**

Compilation warnings: 9 (7 duplicates) — all `dead_code` / unused-import style,
none blocking. Baseline captured; no regression permitted in subsequent tasks.

---

## 5. `SPECIFICATION.md` chapter headings

Level-2 headings, in file order:

1. Language Identity
2. Specification Authority
3. Syntax
4. Value Model
5. Stack
6. Modifiers
7. Built-in Words
8. User Words
9. Module System
10. Child Runtime
11. Error Model
12. Semantic Plane
13. Fractional-Dataflow Internal Invariants
14. AI-first Implementation Rules
15. Conformance Checklist

Version marker in the file header: **2026-04-13** (Status: Canonical).

---

## 6. Observations relevant to later tasks

These are factual observations drawn from the reading above. They are not
decisions; decisions belong to subsequent tasks once the user has directed how
to reconcile the refactor plan with SPECIFICATION.md.

1. `AjisaiError` (rust/src/error.rs) tracks SPEC §11 exactly. A parallel
   `PortError` (Task 7) would introduce a second error enum that the spec does
   not currently sanction. SPEC §15.1 forbids "introducing a second design
   authority".
2. Only three modules (`MUSIC`, `JSON`, `IO`) are registered, matching SPEC §9.1.
   Adding a `PORT` module is a spec-level change.
3. The `OperationTargetMode` / `ConsumptionMode` enums are stack-access
   modifiers (SPEC §6). The refactor plan reuses the words "target" and
   "consume" for DB transaction phases. This is a naming collision.
4. Runtime is `cdylib + rlib` and depends on `wasm-bindgen`. Any
   `rusqlite`-backed backend added later (Task 10) will need `cfg` gating to
   avoid breaking WASM builds; this is not currently in scope for Task 0.
5. No documentation under `docs/refactor/` existed before this file;
   `docs/refactor/` was created empty for this task and contains only this
   document.

---

## 7. Verification

All paths cited above exist on disk as of commit time. Each `Paths:` block
contains one path per line prefixed by `- `; stripping the leading `- ` yields
input for `while read path; do test -e "$path" || echo MISSING: $path; done`.
No `NOT FOUND` entries are present in this document because every item Task 0
asked about was located.
