# D1 scalar-scalar fast path

This note records the implementation of the D1 handoff in
`handoff-d1-scalar-fastpath.md`. D1 is a value-model optimization: it does not
change Ajisai's arithmetic or comparison semantics, only the route taken for the
smallest StackTop scalar cases.

## Scope

The fast path is guarded by `Interpreter::set_scalar_fastpath_enabled(bool)` and
the `AJISAI_NO_SCALAR_FASTPATH` environment switch. It increments
`RuntimeMetrics::scalar_fastpath_count` when it completes an operation.

The implementation deliberately uses this safe subset:

- `OperationTargetMode::StackTop`
- `ConsumptionMode::Consume` and `ConsumptionMode::Keep`
- both operands are bare `Scalar(Fraction)`, or both are singleton numeric
  wrappers with the same effective shape (`Tensor` or non-Text `Vector`)
- arithmetic `+ - * /`
- comparison `< <= > >= = !=`

Everything else falls through to the existing NIL, interval, ExactReal, sparse,
SIMD, broadcast, and Stack-mode paths.

## Observable-value preservation

The fast path reconstructs the same observable result shape as the normal path:

- bare scalar + bare scalar returns a bare scalar
- singleton tensor/vector + singleton tensor/vector returns a singleton tensor
  with the same effective shape the normal broadcast path produces
- mixed scalar/tensor/vector wrappers fall back

Results are still pushed through the same result helpers (`push_result` for
numeric values and the comparison boolean helper for truth values), so semantic
hints remain the same as the baseline route.

For `KEEP`, the fast path mirrors the normal mode contract: the two operands stay
on the stack and the computed result is appended. For `Consume`, the operands are
removed before the result is pushed.

Division by zero is handled in the fast path by pushing the same reasoned NIL
bubble as the existing arithmetic schema. NIL operands are handled before the
fast path by the existing binary NIL passthrough rule.

## Verification

`scalar_fastpath_tests.rs` runs ON/OFF differential tests for stack data,
rendered output, and per-value hints across:

- bare scalar arithmetic
- singleton tensor arithmetic
- singleton vector arithmetic
- bare scalar comparisons
- singleton tensor comparisons
- singleton vector comparisons
- tensor wrapping preservation
- KEEP mode operand preservation
- unsupported/mixed/Text-hinted/NIL fallback cases
- division-by-zero bubble preservation

This keeps D1 measurable while preserving the existing paths as the reference
for all shapes outside the intentionally narrow fast subset.
