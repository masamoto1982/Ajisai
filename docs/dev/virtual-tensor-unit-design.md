# Virtual Tensor Unit (VTU) — Design Note

Status: **Non-canonical** (developer note).
Authority: this document does not define Ajisai semantics. The canonical
source is `SPECIFICATION.md`. If anything here conflicts with the
specification, the specification wins.

## What VTU is

A **Virtual Tensor Unit (VTU)** is an *internal classification* applied to
pure, shape-aware Ajisai computations. It marks blocks that would, in
principle, be schedulable onto a vectorized backend (CPU loop, Wasm SIMD,
NPU, GPU, ...) without changing what the program returns.

VTU is **not** a physical accelerator. It is not StableHLO, not MLIR, not
XLA, not WebGPU, not a TPU backend. The current implementation only
*observes* and *labels* — it never reroutes execution.

## What VTU does today

- Classifies each `QuantizedBlock` with a `VtuHint` describing
  suitability, candidate backends, and approximate data-movement cost.
- Increments observational counters in `RuntimeMetrics` for: tensor
  flatten / rebuild, broadcast paths, unary flat ops, allocated output
  elements, SIMD fast-path uses, candidate vs. rejected blocks, and
  fusion candidates.
- Surfaces these counters through `Interpreter::runtime_metrics()` for
  tests and tooling.

## What VTU does *not* do today

- It does not change the result of any Ajisai program.
- It does not reorder or reschedule operations.
- It does not affect `Fraction` exactness.
- It does not enter `GuardSignature`. VTU is explanation, not contract;
  including it in the guard would cause spurious cache invalidation on
  what is purely classifying information.
- It does not measure real energy. The counters are *proxies* for energy
  / data-movement cost, not joules or watt-hours.

## Existing components, mapped

```
FlatTensor        -> shape-aware dense representation
QuantizedBlock    -> kernel candidate detection
KernelKind        -> virtual kernel classification
PurityInfo        -> safety for cache / reorder / fusion
RuntimeMetrics    -> energy-aware proxy metrics
Wasm SIMD         -> one concrete backend-like fast path
```

## Classification policy

`infer_vtu_hint(kernel_kind, purity)` produces a `VtuHint` with these
defaults:

| Kernel               | Purity         | Suitability       | Notes                                  |
|----------------------|----------------|-------------------|----------------------------------------|
| `MapUnaryPure`       | Pure           | StrongCandidate   | Embarrassingly parallel.               |
| `PredicateUnaryPure` | Pure           | StrongCandidate   | Pure boolean elementwise.              |
| `FoldBinaryPure`     | Pure           | WeakCandidate     | Reductions need an Approx boundary.    |
| `ScanBinaryPure`     | Pure           | WeakCandidate     | Accumulator-carrying, not parallel.    |
| `GenericCompiled`    | Pure           | WeakCandidate     | Pure but unspecialized.                |
| `GenericCompiled`    | Unknown        | NotSuitable       | Conservative.                          |
| `GenericCompiled`    | SideEffecting  | NotSuitable       |                                        |
| `NonQuantizable`     | any            | NotSuitable       |                                        |
| any                  | SideEffecting  | NotSuitable       | Hard rule.                             |

The default for `VtuHint` is `NotSuitable`. We never *upgrade* a hint
opportunistically.

## Counter semantics

| Counter                                  | Bumped when                                    |
|------------------------------------------|------------------------------------------------|
| `vtu_tensor_flatten_count`               | Each successful `FlatTensor::from_value`.      |
| `vtu_tensor_flattened_elements`          | Sum of element counts over those flattens.     |
| `vtu_tensor_rebuild_count`               | Each rebuild back into nested `Value`.         |
| `vtu_tensor_rebuilt_elements`            | Sum of rebuilt element counts.                 |
| `vtu_broadcast_count`                    | Each binary broadcast that begins executing.   |
| `vtu_unary_flat_count`                   | Each unary flat op that begins executing.      |
| `vtu_allocated_elements`                 | Output buffer size of tensor ops.              |
| `vtu_same_shape_elementwise_count`       | Same-shape fast path inside binary broadcast.  |
| `vtu_projected_broadcast_count`          | Index-projection path inside binary broadcast. |
| `vtu_simd_kernel_use_count`              | A SIMD fast path in `arithmetic.rs` succeeds.  |
| `vtu_candidate_block_count`              | A built block's `VtuHint` is Strong or Weak.   |
| `vtu_rejected_block_count`               | A built block's `VtuHint` is NotSuitable.      |
| `vtu_fusion_candidate_count`             | A built block reports `eligible_for_fusion` or `can_fuse`. |

Counters are bumped at *block build time* and at *operation start time*.
Cache hits do not double-count classifications.

These are observational only: they describe what shape-aware work the
runtime did, not what it cost in energy. The names deliberately avoid
verbs like `energy_saved`.

## Phase II — Dense Vector representation (in progress)

Phase II is now under way. See `docs/dev/vtu-phase-ii-handover.md` for the
multi-PR plan. The first concrete change is a parallel `Tensor` variant
on `ValueData`:

- `Vector(Rc<Vec<Value>>)` remains the **nested** form — used for mixed-type
  vectors and any tree-shaped data.
- `Tensor { data: Rc<Vec<Fraction>>, shape: Rc<Vec<usize>> }` is the new
  **dense** form — used when every leaf is a Fraction and the shape is
  rectangular.

Observable semantics are identical between the two; classification is a
construction-time decision. PR #1 (this work) only adds the variant and
makes every consumer match-exhaustive against it. No producer is wired yet,
so the existing test suite is unchanged. PR #2 will switch the literal
parser and `apply_*_with_metrics` outputs over to `Tensor`, at which point
`vtu_tensor_rebuild_count` should drop sharply.

## Future scope (not implemented)

- `ajisai trace --vtu` / `ajisai trace --energy` displays.
- `ajisai explain --vtu <word>` per-word classification dump.
- Plan-level fusion execution (Phase II PR #4).
- In-place mutation of dense `Tensor.data` (Phase II PR #5).
- Pure-plan result memoisation keyed on Fraction digests (Phase II PR #6).
- Integer-Fraction SIMD extension for `MAP` / `FILTER` kernels (Phase II PR #7).
- An explicit `EXACT` / `APPROX` boundary (e.g. `TO-F32`, `TO-BF16`)
  before any approximate backend may run.
- StableHLO / MLIR-style textual dump.
- Real WebGPU / NPU / TPU backends.

## Design message

> Ajisai does not assume a physical TPU. It models pure, shape-aware
> computations as VTU plans that *could* be dispatched onto CPU, SIMD,
> Wasm, NPU, GPU, or — eventually — TPU.
>
> Today it does not redirect execution. It only makes wasted work,
> wasted rebuilds, and wasted data movement visible.
>
> Ajisai is a debuggable, exact-by-default, virtual-accelerator-aware
> language.
