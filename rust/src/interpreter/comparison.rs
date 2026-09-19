use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::lane_lift::lift_lanes;
use crate::interpreter::record_lift;
use crate::interpreter::value_extraction_helpers::nil_passthrough_binary;
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::Recoverability;
use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value, ValueData};

use super::comparison_scalar::{compare_scalar_pair, scalar_pair_eq, OrderingKind, ScalarCmp};

fn push_boolean_result(interp: &mut Interpreter, result: bool) {
    interp.stack.push(Value::from_bool(result));
    let stack_len = interp.stack.len();
    interp
        .stack
        .set_role_at(stack_len - 1, Interpretation::TruthValue);
}

/// The logical Unknown (U): a NIL read in truth position (LANG.VALUES.TRUTH),
/// carrying the reason a comparison could not decide. Mirrors
/// `interpreter::logic::as_unknown` — U's `hint` is `TruthValue` directly
/// (not just the stack role) so `Value::truth_value()` reports `"unknown"`
/// from the value alone, and `NIL?`/`NIL-REASON` still see the
/// absence it is (SPEC: being read in truth position adds an observation, it
/// takes none away).
fn undecidable_truth_value() -> Value {
    let mut v = Value::nil_with_reason(NilReason::Undecidable, Recoverability::Retryable);
    v.hint = Interpretation::TruthValue;
    v
}

fn push_undecidable_result(interp: &mut Interpreter) {
    interp.stack.push(undecidable_truth_value());
    let stack_len = interp.stack.len();
    interp
        .stack
        .set_role_at(stack_len - 1, Interpretation::TruthValue);
}

struct ScalarFastOperand {
    fraction: Fraction,
}

fn scalar_fast_operand(value: &Value) -> Option<ScalarFastOperand> {
    match &value.data {
        ValueData::Scalar(f) => Some(ScalarFastOperand {
            fraction: f.clone(),
        }),
        _ => None,
    }
}

/// The top two stack operands, if the fast path applies to both: enabled,
/// two-deep, and both plain Scalars. Shared by the ordering and equality
/// fast paths so their eligibility check has one definition.
fn scalar_fastpath_pair(interp: &Interpreter) -> Option<(ScalarFastOperand, ScalarFastOperand)> {
    if !interp.scalar_fastpath_enabled || interp.stack.len() < 2 {
        return None;
    }
    let stack_len = interp.stack.len();
    let a = scalar_fast_operand(&interp.stack[stack_len - 2])?;
    let b = scalar_fast_operand(&interp.stack[stack_len - 1])?;
    Some((a, b))
}

fn record_fastpath_hit(interp: &mut Interpreter) {
    let count = &mut interp.runtime_metrics.scalar_fastpath_count;
    *count = count.saturating_add(1);
}

fn push_ordering_scalar_fastpath(interp: &mut Interpreter, kind: OrderingKind) -> bool {
    let Some((a, b)) = scalar_fastpath_pair(interp) else {
        return false;
    };
    let decided = kind.apply_to_fraction(&a.fraction, &b.fraction);
    if interp.consumption_mode == ConsumptionMode::Consume {
        interp.stack.pop();
        interp.stack.pop();
    }
    push_boolean_result(interp, decided);
    record_fastpath_hit(interp);
    true
}

fn push_equality_scalar_fastpath(interp: &mut Interpreter, invert: bool) -> bool {
    let Some((a, b)) = scalar_fastpath_pair(interp) else {
        return false;
    };
    let eq = a.fraction == b.fraction;
    if interp.consumption_mode == ConsumptionMode::Consume {
        interp.stack.pop();
        interp.stack.pop();
    }
    push_boolean_result(interp, if invert { !eq } else { eq });
    record_fastpath_hit(interp);
    true
}

/// Apply an ordering Word across the shapes LANG.COLLECTIONS.LIFT allows.
///
/// The alignment is `lane_lift`'s, shared with `booleanLogic`, so a
/// comparison and the `AND` that combines two of its results agree on what
/// pairs: equal lengths pair lane by lane, and a one-lane operand is reused
/// across the other's length. That last case is what makes a mask writable
/// against a bare threshold — `[ 1 2 3 ] [ 2 ] GT` — and it was the one shape
/// this family refused while `[ 1 2 3 ] [ 2 ] MUL` had always accepted it.
///
/// The comparison family used to do the reverse of this: it projected a
/// singleton Vector to its element (`[ 3 ] 4 LT` was `TRUE`) and refused the
/// element-wise application the clause requires (`[ 3 4 ] 4 LT` was an ERROR).
///
/// A NIL operand lane answers NIL, which is the scalar law's own outcome for
/// `NIL 3 LT` — the clause says each lane preserves the scalar law's NIL
/// distinction.
fn lift_comparison(a_val: &Value, b_val: &Value, kind: OrderingKind) -> Result<Value> {
    lift_lanes([a_val, b_val], &|[a, b]| compare_lane(a, b, kind))
}

/// One lane of an ordering comparison: both operands are past the alignment,
/// so neither is a Vector here.
fn compare_lane(a_val: &Value, b_val: &Value, kind: OrderingKind) -> Result<Value> {
    if a_val.is_nil() || b_val.is_nil() {
        return Ok(Value::nil_with_reason_unknown(
            a_val
                .nil_reason()
                .or_else(|| b_val.nil_reason())
                .copied()
                .unwrap_or(NilReason::Literal),
        ));
    }
    // `unsupportedComparison`: LT/LTE/GT/GTE, the only callers of
    // `compare_lane`, declare it uniformly. EQ/NEQ never reach here —
    // `pairwise_eq` is total and raises nothing.
    match compare_scalar_pair(a_val, b_val, kind).map_err(|e| match e {
        AjisaiError::StructureError { expected, .. } if expected == "scalar value" => {
            AjisaiError::declared("unsupportedComparison", "expected comparable operands")
        }
        other => other,
    })? {
        ScalarCmp::Decided(b) => Ok(Value::from_bool(b)),
        ScalarCmp::Undecided => Ok(undecidable_truth_value()),
    }
}

fn apply_binary_comparison(interp: &mut Interpreter, kind: OrderingKind) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let (a_val, b_val) = if is_keep_mode {
        let stack_len = interp.stack.len();
        let a_val = interp.stack[stack_len - 2].clone();
        let b_val = interp.stack[stack_len - 1].clone();
        (a_val, b_val)
    } else {
        let b_val = interp.stack.pop().unwrap();
        let a_val = interp.stack.pop().unwrap();
        (a_val, b_val)
    };

    match lift_comparison(&a_val, &b_val, kind) {
        Ok(result) => {
            interp.stack.push(result);
            Ok(())
        }
        Err(e) => {
            if !is_keep_mode {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
            }
            Err(e)
        }
    }
}

fn apply_ordering_schema(interp: &mut Interpreter, kind: OrderingKind) -> Result<()> {
    if nil_passthrough_binary(interp) {
        return Ok(());
    }
    if push_ordering_scalar_fastpath(interp, kind) {
        return Ok(());
    }
    apply_binary_comparison(interp, kind)
}

pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    if record_lift::lift_binary(interp, &op_lt)? {
        return Ok(());
    }
    apply_ordering_schema(interp, OrderingKind::Lt)
}

pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    if record_lift::lift_binary(interp, &op_le)? {
        return Ok(());
    }
    apply_ordering_schema(interp, OrderingKind::Le)
}

pub fn op_gt(interp: &mut Interpreter) -> Result<()> {
    if record_lift::lift_binary(interp, &op_gt)? {
        return Ok(());
    }
    apply_ordering_schema(interp, OrderingKind::Gt)
}

pub fn op_gte(interp: &mut Interpreter) -> Result<()> {
    if record_lift::lift_binary(interp, &op_gte)? {
        return Ok(());
    }
    apply_ordering_schema(interp, OrderingKind::Ge)
}

pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    apply_equality(interp, false)
}

pub fn op_neq(interp: &mut Interpreter) -> Result<()> {
    apply_equality(interp, true)
}

/// Pairwise equality. Every pair decides: the structural Vector / Tensor paths
/// are total, as is scalar comparison.
///
/// Equality is *structural over disjoint domains* (LANG.VALUES.DISJOINT): two
/// values are "never equal merely because their encodings resemble one
/// another". Two consequences are load-bearing here.
///
/// A singleton Vector is not its element. `[ 3 ] 3 EQ` used to decide TRUE via
/// a projection path, which made the Vector and Scalar domains overlap for
/// this one Word while `EQ`'s own contract promises a decision over tagged
/// values. It now decides FALSE, like every other cross-domain pair.
///
/// Two NILs are the same value exactly when their reasons agree
/// (LANG.VALUES.NIL: "the reason is the entire observable content of a NIL").
/// The `ValueData` comparison below cannot see a reason — `ValueData::Nil`
/// carries none — so NIL pairs are decided before it, on the reason itself.
fn pairwise_eq(a_val: &Value, b_val: &Value) -> ScalarCmp {
    if a_val.is_nil() || b_val.is_nil() {
        return ScalarCmp::Decided(
            a_val.is_nil() && b_val.is_nil() && a_val.nil_reason() == b_val.nil_reason(),
        );
    }
    // A Tier 2 operand makes `ValueData` equality answer from allocation
    // identity, so it may not settle anything (`Value::carries_computable`).
    let tier2 = a_val.carries_computable() || b_val.carries_computable();
    if !tier2 && a_val.data == b_val.data {
        return ScalarCmp::Decided(true);
    }
    match (&a_val.data, &b_val.data) {
        (ValueData::Scalar(_), ValueData::Scalar(_))
        | (ValueData::ExactScalar(_), ValueData::ExactScalar(_))
        | (ValueData::ExactScalar(_), ValueData::Scalar(_))
        | (ValueData::Scalar(_), ValueData::ExactScalar(_)) => scalar_pair_eq(a_val, b_val),
        (ValueData::Vector(x), ValueData::Vector(y)) if tier2 => vector_pair_eq(x, y),
        // Two Records are one value when their key sequences and their value
        // sequences are (LANG.RECORDS.STRUCTURE); a Tier 2 value in either
        // sequence makes the answer as undecidable as it is for Vectors.
        (ValueData::Record(x), ValueData::Record(y)) if tier2 => {
            match vector_pair_eq(x.keys(), y.keys()) {
                ScalarCmp::Decided(false) => ScalarCmp::Decided(false),
                keys => match (keys, vector_pair_eq(x.values(), y.values())) {
                    (_, ScalarCmp::Decided(false)) => ScalarCmp::Decided(false),
                    (ScalarCmp::Decided(true), ScalarCmp::Decided(true)) => {
                        ScalarCmp::Decided(true)
                    }
                    _ => ScalarCmp::Undecided,
                },
            }
        }
        // Disjoint domains are unequal whatever they carry
        // (LANG.VALUES.DISJOINT), so Tier 2 does not make them undecidable.
        _ => ScalarCmp::Decided(false),
    }
}

/// Element-wise equality of two Tier 2-carrying Vectors, combined as the
/// Kleene conjunction the truth domain already uses: one unequal element (or
/// a length difference) settles FALSE, and only an otherwise-equal pair with
/// an undecided element is UNKNOWN.
fn vector_pair_eq(x: &[Value], y: &[Value]) -> ScalarCmp {
    if x.len() != y.len() {
        return ScalarCmp::Decided(false);
    }
    let mut answer = ScalarCmp::Decided(true);
    for (p, q) in x.iter().zip(y.iter()) {
        match pairwise_eq(p, q) {
            ScalarCmp::Decided(false) => return ScalarCmp::Decided(false),
            ScalarCmp::Decided(true) => {}
            ScalarCmp::Undecided => answer = ScalarCmp::Undecided,
        }
    }
    answer
}

fn apply_equality(interp: &mut Interpreter, invert: bool) -> Result<()> {
    if nil_passthrough_binary(interp) {
        return Ok(());
    }

    if push_equality_scalar_fastpath(interp, invert) {
        return Ok(());
    }

    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let (a_val, b_val) = if is_keep_mode {
        let stack_len = interp.stack.len();
        let a_val = interp.stack[stack_len - 2].clone();
        let b_val = interp.stack[stack_len - 1].clone();
        (a_val, b_val)
    } else {
        let b_val = interp.stack.pop().unwrap();
        let a_val = interp.stack.pop().unwrap();
        (a_val, b_val)
    };

    match pairwise_eq(&a_val, &b_val) {
        ScalarCmp::Decided(eq) => push_boolean_result(interp, if invert { !eq } else { eq }),
        ScalarCmp::Undecided => push_undecidable_result(interp),
    }
    Ok(())
}
