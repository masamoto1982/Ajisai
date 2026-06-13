# energyProxyScore — definition, weights, and honesty policy

Status: quality-discipline document (non-canonical for language semantics).
Authority for Ajisai semantics: `SPECIFICATION.html` only.

## What this is — and what it is not

`energyProxyScore` is a single deterministic integer that aggregates the
Virtual Tensor Unit observation counters (`docs/dev/virtual-tensor-unit-design.md`)
into one structural-cost figure. It exists so that a change which keeps a
program's output identical but moves more data becomes a CI-visible
regression instead of an unnoticed drift.

**It is a proxy. It is not joules.**

- It does **not** measure, estimate, or predict energy consumption,
  wall-clock time, or power draw.
- It counts *structural work the runtime was observed to do* — elements
  carried across the flat/nested tensor boundary, output elements allocated,
  per-operation dispatch — and nothing else.
- Per the standing policy in `virtual-tensor-unit-design.md`, neither the
  counters nor this score use result-asserting names like `energy_saved`.
  The score is `energyProxyScore` (a proxy), never `energyUsed`.

Connecting the proxy to real joules requires physical measurement, which the
project has not yet done. Until then, README / SPECIFICATION must not claim
Ajisai "is low-power"; the honest claim is only that Ajisai *observes and
enforces structural indicators* that plausibly correlate with energy.

## Determinism

`energy_proxy_score(metrics)` is a pure function of `RuntimeMetrics`. The
same program on the same input always produces the same counters and hence
the same score; computing the score never affects execution. This is
asserted by `score_is_deterministic_across_runs` in
`energy_proxy_regression_tests.rs`.

## Formula (proxyVersion = 1)

```
cost      = 4 * tensorFlattenedElements
          + 4 * tensorRebuiltElements
          + 2 * allocatedElements
          + 16 * tensorFlattenCount
          + 16 * tensorRebuildCount
          + 8 * broadcastCount
          + 8 * unaryFlatCount

deduction = 4 * simdKernelUseCount
          + 8 * bulkKernelUseCount
          + 4 * projectedBroadcastCount

score     = saturating_sub(cost, deduction)   // floored at 0
```

### Weight rationale

| Term | Weight | Why |
|---|---|---|
| `tensorFlattenedElements` | 4 / element | Element-granular data movement across the flat boundary; the dominant real cost driver, charged per element. |
| `tensorRebuiltElements` | 4 / element | Symmetric to flattening: rebuilding nested form from flat. |
| `allocatedElements` | 2 / element | Output materialization. Charged, but lighter than a boundary round-trip. |
| `tensorFlattenCount` | 16 / op | Fixed per-operation dispatch/setup overhead, independent of size. |
| `tensorRebuildCount` | 16 / op | Symmetric per-op rebuild overhead. |
| `broadcastCount` | 8 / op | Per binary-broadcast dispatch. |
| `unaryFlatCount` | 8 / op | Per unary flat-tensor dispatch. |
| `simdKernelUseCount` | −4 / use | Realized SIMD lane: same work, less per-element overhead → deducted. |
| `bulkKernelUseCount` | −8 / use | Realized bulk HOF kernel iterating `&[Fraction]` without per-element Value materialization → larger deduction. |
| `projectedBroadcastCount` | −4 / use | Index-projected broadcast avoids materializing the broadcast operand → deducted. |

Deductions use saturating subtraction, so the score never underflows below
0. An all-efficient path (e.g. same-shape 1-D SIMD arithmetic, which records
no boundary movement) therefore scores 0 — the honest "no structural cost
observed" result.

### Counters deliberately excluded from the score

- **Sparse counters** (`sparseCandidate*`, `sparseSkippableZeroElements`) are
  *candidates*, not realized skips. They earn no deduction — the score never
  credits work the runtime did not actually avoid. They feed `suggestions`
  instead. (`sparse_candidates_earn_no_deduction` pins this.)
- **Cache / plan / hedge counters** (`compiled_plan_*`, `hedged_*`,
  `resolve_cache_*`) are execution-strategy bookkeeping, not data-movement
  cost, and are out of scope for this proxy.
- `sameShapeElementwiseCount`, `candidateBlockCount`, `fusionCandidateCount`,
  `rejectedBlockCount` are classification observations (reported in
  `suggestions`), not cost.

## `suggestions`

Mechanical, counter-derived observations emitted alongside the score (in the
CLI `--json` under `runtimeMetrics.vtu.suggestions`). Each rule reads only the
counters and states the structural pattern plus the program-level change that
removes it. Wording stays structural ("data movement", "round-trips") and
never asserts an energy outcome. Current rules:

1. `fusionCandidateCount > 0` — adjacent fusable elementwise stages; merging
   avoids intermediate flatten/rebuild round-trips.
2. `tensorRebuildCount ≥ 2` and total round-trips ≥ 4 — values cross the
   flat/nested boundary repeatedly; chaining tensor words keeps data flat.
3. `sparseSkippableZeroElements ≥ ½ · sparseCandidateElements` — half or more
   candidate lanes are zero; restructuring to avoid moving zero lanes reduces
   movement.
4. `rejectedBlockCount > 0` — quantized blocks rejected as VTU candidates;
   build with the `trace-quant` feature to see why.

## proxyVersion and re-baselining

`ENERGY_PROXY_VERSION` (currently **1**) tags every score. **Any** change to a
weight, to the formula, to which counters participate, or to a deduction
**must** increment it and update this document in the same change. Scores
produced under different `proxyVersion` values are not comparable.

Incrementing `proxyVersion` re-baselines the entire regression catalog in
`energy_proxy_regression_tests.rs`: re-run
`cargo test energy_proxy_discovery -- --nocapture` and record the new
`baseline_score` for each case.

## Regression discipline

`energy_proxy_regression_tests.rs` runs a fixed catalog of programs and
asserts, for each: (a) the output stack is unchanged, and (b) the live score
does not exceed the recorded `baseline_score`. A catalog program with a
non-zero baseline must exist (`tensor_pipeline_is_observed`) so a globally
zeroed counter pipeline cannot silently pass every `<=` check.

- Score drops after an intentional optimization → tighten the baseline in the
  same PR.
- Score rises → it is a regression by default. Either fix it, or, if the
  increase is a justified trade-off, raise the baseline **explicitly** and
  explain why in the PR. Never bump a baseline casually to make CI green.
