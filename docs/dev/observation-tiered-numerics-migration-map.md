# Migration Map: Observation-Unified Tiered Numerics (Phase 0 Inventory)

Status: working memo, non-canonical. Companion to
`observation-tiered-numerics-rework-instructions.md`.
Baseline commit: the parent of the first Phase 1 commit on
`claude/observation-tiered-numerics-xuu05c`.

## 1. Reference inventory

Every production reference to the CF-era surface, grouped by the phase
that migrates it. Test files are listed only when they pin behavior that
must move (not merely compile).

### `ExactReal` construction and arithmetic (Phase 3 rewires)

| Site | Uses | Migration |
| --- | --- | --- |
| `interpreter/interval_ops.rs` (`op_sqrt`) | `ExactReal::from_sqrt_rational` | build Tier 1 value; same NIL-on-negative rule |
| `interpreter/arithmetic.rs` | `schema.exact_real` → `add/sub/mul/div`, `Value::from_exact_real`, broadcast lanes | route through `Algebraic` field ops |
| `interpreter/math_ops.rs` (`NEG`, `ABS`, `SIGN`, `MIN`, `MAX`) | `exact_real_of`, `er.neg()`, `three_way_compare` | `Algebraic::neg` + decidable `cmp` |
| `interpreter/tensor_cmds.rs` (`FLOOR`/`CEIL`/`ROUND`/`MOD` exact paths) | `er.floor/ceil/round`, `value_as_exact_real`, `to_fraction` | `Algebraic` decidable equivalents |
| `interpreter/parallel.rs` | `Option<ExactReal>` lanes (prod + tests) | lane type becomes the new exact type |
| `interpreter/modules/semantic_sync.rs` | fingerprints `ExactReal::{Rational,AlgebraicSqrt,Gosper}` variants directly | fingerprint the Tier 1 normal form instead (internal only, not observable) |

### Comparison / UNKNOWN production (Phase 3 replaces)

| Site | Uses | Migration |
| --- | --- | --- |
| `interpreter/comparison.rs` | `cmp_with_budget_tracked`, `cmp_streamed_with_budget_tracked`, `CmpOutcome`, `DEFAULT_COMPARISON_BUDGET`, `push_unknown` | Tier ≤ 1: budget-free total `cmp`; `Undecided` path reserved for Tier 2 |
| `interpreter/sort.rs`, `math_ops.rs` | `three_way_compare` → `OrderOutcome::Undecided` | `Undecided` unreachable from current vocabulary |
| `interpreter/logic_kleene.rs`, `logic.rs`, `nil_diagnostics.rs`, `debug_diagnosis.rs`, `cli/{report,explain,clarify}.rs` | `agreed_prefix` diagnosis plumbing | key kept, meaning redefined (D3); production source becomes Tier 2 only |

### Display / serialization (Phase 3-4 rederive, byte-compatible)

| Site | Uses | Migration |
| --- | --- | --- |
| `types/display.rs` | `partial_quotients`, `partial_quotients_bounded(32)` | CF terms derived from Tier 1 by floor+reciprocal (or √ period); same nested form, same `CF_DISPLAY_BUDGET` |
| `types/value_protocol.rs` | `best_rational_approximation(1e9)`, CF-role display string | same observable protocol; approximation from Tier 1 |
| `wasm_interpreter_bindings/wasm_value_conversion.rs` | `approximate: true` marker for `ExactScalar` | unchanged (keyed on value kind, not representation) |
| `cli/report.rs` | same protocol node + `agreedPrefix` | unchanged shape |
| `interpreter/vector_exec.rs`, `types/arena.rs`, `cast/cast_value_helpers.rs`, `value_extraction_helpers.rs`, `higher_order/memo.rs`, `simd_ops.rs`, `interval_ops.rs (>CF)` | kind-dispatch on `ValueData::ExactScalar` | mechanical rename of the payload type |

**WASM/TS protocol finding (D4): no CF internals leak.** The wire format
exposes only `numerator/denominator` (best rational approximation), the
`approximate` marker, and the ContinuedFraction-role display string.
Radicands, partial-quotient lists, and Gosper structure never cross the
boundary. No protocol change is needed; `src/wasm-interpreter-types.ts`
stays as-is.

### Retired with Phase 4

- `types/continued_fraction.rs` Gosper machinery: `Gosper` enum, Möbius /
  bihomographic steppers, `CfIter` ingestion, `NicfStream`,
  `cmp_with_budget*`, `cmp_streamed_with_budget_tracked`, `rcf_order`,
  `eq/ne/lt/le/gt/ge_with_budget`, `GOSPER_INGEST_SAFETY`,
  `enclosing_interval`, `cmp_via_interval_filter`.
- Kept (moved to a display-derivation module): rational CF expansion,
  √ surd stepper / `sqrt_cf_period`, `partial_quotients(_bounded)`,
  `from_partial_quotients`, nested-form rendering.
- `types/multiquadratic.rs` is promoted into the Tier 1 core (Phase 2);
  the `ExactReal`-walking ingestion (`collect_radicands`, `eval`) retires
  with the Gosper tree.
- `rust/examples/sqrt_cf_bench.rs`, `rust/tests/nicf_feasibility.rs`,
  `rust/tests/nicf_gosper_feasibility.rs` exercise the retired machinery
  and retire/shrink with it.
- `rust/src/elastic/` has **no** CF references; nothing to follow up.

## 2. Preserved behavior (captured 2026-07-17, all-green baseline)

Golden observations from `ajisai run` (debug build, step budget default):

| Program | Observed |
| --- | --- |
| `'math' IMPORT 2 SQRT` | `( 1 ( 2 ( 2 … ( 2 ...) … )` — 32 terms, `...)` marker |
| `'math' IMPORT 3 SQRT` | `( 1 ( 1 ( 2 ( 1 ( 2 … ...)` — 32 terms |
| `'math' IMPORT 1 2 DIV SQRT` | `( 0 ( 1 ( 2 ( 2 … ...)` |
| `'math' IMPORT 2 SQRT NEG` | `( -2 ( 1 ( 1 ( 2 ( 2 … ...)` |
| `'math' IMPORT 2 SQRT 2 SQRT ADD` | `( 2 ( 1 ( 4 ( 1 ( 4 … ...)` |
| `'math' IMPORT 9 4 DIV SQRT` | `3/2` (perfect square projects to Tier 0) |
| `'math' IMPORT 0 1 SUB SQRT` | `NIL` (negative radicand) |
| `'math' IMPORT 2 SQRT SQRT` | **error** `SQRT: expected Number or Interval` |
| `'math' IMPORT 2 SQRT 2 LT` | `TRUE` |
| `'math' IMPORT 2 SQRT 2 SQRT EQ` | `TRUE` |
| `'math' IMPORT 8 SQRT 2 SQRT 2 MUL EQ` | `TRUE` (multiquadratic path) |
| `'math' IMPORT 2 SQRT 2 SQRT MUL` | `2/1` |
| `1 0 DIV` | `NIL` (divisionByZero) |
| `'math' IMPORT 2 SQRT 3 SQRT 1 COMPARE-WITHIN` | `-1/1` (O(1) surd shortcut) |
| `'math' IMPORT 2 SQRT 2 SQRT 1 COMPARE-WITHIN` | `0/1` |
| `'math' IMPORT 8 SQRT 2 SQRT 2 MUL 5 COMPARE-WITHIN` | `UNKNOWN` |
| `'math' IMPORT 2 SQRT 1 ADD 2 SQRT 1 ADD 3 COMPARE-WITHIN` | `UNKNOWN` |
| `'math' IMPORT 1 2 SQRT ADD 2 SQRT 1 SUB MUL` | `( ...)` (Gosper cannot pin a0) |

Notes:

- **Nested `SQRT` is outside the admitted domain today** (`as_scalar()`
  rejects `ExactScalar`, and `value_to_interval` does too, so `2 SQRT SQRT`
  is malformed use → error). D1 resolution: the multiquadratic normal form
  covers the entire reachable domain; keep the same boundary (error) after
  the switch. No general-algebraic fallback needed.
- The two `UNKNOWN` rows and the `( ...)` row are the only observations
  the rework intentionally changes (instructions §5-2, §8): Gosper-built
  values gain exact normal forms, so `COMPARE-WITHIN` decides and
  `(1+√2)(√2−1)` displays as `1/1`. Everything else in the table is a
  golden that must not move.
- Default relations (`EQ`/`LT`/…) already never yield `UNKNOWN` over the
  current vocabulary (the §4.2.7 multiquadratic pre-pass is total there);
  the streamed `COMPARE-WITHIN` window is today's only `UNKNOWN` source.

## 3. Quality-gate baseline (all green before Phase 1)

`cargo test --lib` 1566 passed / `cargo test --tests` all green /
`npm run check`, `npm run test` (192), `check:semantic-firewall`,
`provenance:check`, `check:file-size` (0 violations),
`check:formalization-coverage` (215/215) all green.

## 4. Completion record (Phases 1-6)

- The tiered core lives in `rust/src/types/exact/`: `observation.rs`
  (Observation/Refine/Water/RatInterval), `basis.rs` + `algebraic*.rs`
  (Tier 1), `computable.rs` (Tier 2 receptacle), `value.rs` +
  `value_approx.rs` (the `ExactReal` enum behind `ValueData::ExactScalar`).
  `continued_fraction.rs` and `multiquadratic.rs` are deleted (−6,779
  lines); CF terms survive as display-side derivation only.
- D1 resolved with the default: the multiquadratic normal form covers the
  whole reachable domain; nested `SQRT` keeps its historical error
  boundary. D2: `SQRT-EPS`/`POW` observations unchanged. D3: `agreedPrefix`
  kept, redefined as refinement steps without separation (Tier 2 only).
  D4: no protocol change was needed (no CF internals ever crossed the
  boundary).
- Every golden in §2 was re-verified after the switch; the two rows marked
  as intended changes now read `0/1` (COMPARE-WITHIN decides) and `1/1`
  (demotion), as specified.
- Performance: wall-clock comparison of release CLIs (pre-rework 8f8f389
  vs post-Phase 5) on a 5,000-lane rational vector chain and a chained
  algebraic-comparison workload showed parity or better on both; the
  rational path is byte-identical code, and the algebraic path replaces
  Gosper streaming with normal-form arithmetic.
- Tier 2 isolation is pinned by `interpreter/tier2_isolation_tests.rs`
  (vocabulary sweep reaches neither Tier 2 nor UNKNOWN; the Starved → U
  projection stays wired for future words).
