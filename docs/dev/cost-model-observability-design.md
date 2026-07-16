# Cost model: observability and specification (design note)

Status: **Non-canonical** (developer note).
Authority: this note defines no semantics. The canonical source is
`SPECIFICATION.html`; the Cost Model is normative there at §4.8. This note
records how cost was made observable and why the observable surface is a
diagnostics channel rather than the value model.

## Goal

Follow-up to the Cost Model documentation (Reference page + §4.8): make the
cost model *observable at runtime* and *normative in the specification*, per
the external review's request to give users a predictable performance model.

## The invariant that shapes the design

§4.2.2 and §4.3.1 make a value's internal representation **non-observable**:
it is not part of value identity, order, display, or serialization. A word
that returned "is this value a small rational or a big one?" as a comparable
value would break that invariant. So cost is exposed the same way VTU already
exposes data-movement cost — as **observational counters** that never enter
the value model, equality, or any guard/cache signature (see
`docs/dev/virtual-tensor-unit-design.md`). §4.8 fixes this as a normative
rule: cost counters are diagnostics; reading them changes no result, and no
Coreword reads them.

## What was implemented

### 1. Comparison-budget counters (Rust)

The existing `RuntimeMetrics` already covered "which values were fast"
(`scalar_fastpath_count`) and "how much data moved" (the `vtu_*` counters).
The missing cost-model question was #4 — *when is the comparison budget
consumed?* Over the admitted domain \(D\) the six relations decide exactly
via the multiquadratic normal form and spend nothing; `COMPARE-WITHIN` is the
one Coreword that streams partial quotients under an explicit budget. So the
new counters live entirely in `op_compare_within`
(`rust/src/interpreter/comparison.rs`):

- `compare_within_count` — invocations that reached the comparison step.
- `compare_within_lazy_count` — the streamed (non-rational) subset that can
  actually spend budget.
- `compare_within_unknown_count` — invocations that exhausted the budget → U.
- `compare_within_budget_terms_consumed` — total NICF terms consumed, summed
  from `agreedPrefix` on the U results.

Tests (`arithmetic_operation_tests.rs::compare_within_metrics_tests`) assert
that a bare relation over sqrt operands spends nothing, a rational
`COMPARE-WITHIN` counts but spends nothing, and an equal-lazy
`COMPARE-WITHIN` records `lazy`, `unknown`, and consumed terms.

### 2. Observable surfaces

- **CLI `--json`** (`cli/report.rs::runtime_metrics_json`): added
  `scalarFastpathCount` and a `comparison` group. Additive; schema stays v1.
- **WASM** (`wasm_interpreter_bindings/wasm_runtime_metrics.rs`):
  `collect_runtime_metrics()` returns the cost-relevant counters as a JS
  object for the Playground. Placed in its own module to keep
  `wasm_interpreter_state.rs` under its file-size budget.

### 3. Specification (§4.8 Cost model — performance contract)

A normative subsection of the Value Model that: states the
representation-non-observability relationship (the cost model constrains
performance, never meaning); tabulates the cost classes (small-rational fast
lane, big-integer rational, lazy irrational, dense vs nested vector); lists
the promotion triggers; restates the comparison-budget guarantee (exact over
\(D\), budget only via out-of-\(D\) lazy values and `COMPARE-WITHIN`, unit =
one NICF term); and fixes the one new normative rule — cost counters are
observational-only and outside the conformance surface.

## Follow-up (not in this change)

A Playground **Cost panel** that renders `collect_runtime_metrics()` after a
run. It needs a WASM rebuild to regenerate the TypeScript bindings
(`.d.ts`) plus browser verification, so it is scoped separately from this
Rust + spec change. The WASM API and CLI `--json` already make the counters
observable at the contract level; the panel is the presentation layer on top.
