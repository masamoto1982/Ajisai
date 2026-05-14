use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::interval_ops::value_to_interval;
use crate::interpreter::tensor_ops::FlatTensor;
use crate::interpreter::value_extraction_helpers::{
    extract_integer_from_value, nil_passthrough_binary,
};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::continued_fraction::{ExactReal, DEFAULT_COMPARISON_BUDGET};
use crate::types::fraction::Fraction;
use crate::types::{DisplayHint, Value, ValueData};

/// One of the four ordering comparisons. Carries the dispatch
/// decision through the SCALAR-comparison helper so the helper can
/// keep the Fraction fast path for both-Rational operands while
/// routing any non-Rational ExactReal pair through
/// `ExactReal::cmp_with_budget` (SPEC §7.4.1).
#[derive(Debug, Clone, Copy)]
enum OrderingKind {
    Lt,
    Le,
    Gt,
    Ge,
}

impl OrderingKind {
    fn apply_to_fraction(self, a: &Fraction, b: &Fraction) -> bool {
        match self {
            OrderingKind::Lt => a.lt(b),
            OrderingKind::Le => a.le(b),
            OrderingKind::Gt => a.gt(b),
            OrderingKind::Ge => a.ge(b),
        }
    }

    /// Apply the relation to a budgeted `ExactReal` three-way
    /// comparison result. `None` propagates the budget-exhaustion
    /// case unchanged so callers project it to the §7.4.1
    /// Undecidable NIL.
    fn apply_to_ordering(self, ord: Option<std::cmp::Ordering>) -> Option<bool> {
        use std::cmp::Ordering;
        ord.map(|o| match self {
            OrderingKind::Lt => o == Ordering::Less,
            OrderingKind::Le => o != Ordering::Greater,
            OrderingKind::Gt => o == Ordering::Greater,
            OrderingKind::Ge => o != Ordering::Less,
        })
    }
}

fn push_boolean_result(interp: &mut Interpreter, result: bool) {
    interp.stack.push(Value::from_bool(result));
    let stack_len = interp.stack.len();
    interp.semantic_registry.normalize_to_stack_len(stack_len);
    interp
        .semantic_registry
        .update_hint_at(stack_len - 1, DisplayHint::Boolean);
}

/// Push the SPEC §7.4.1 undecidable-NIL: reason = `Undecidable`,
/// origin = `ComparisonBudget`. The Bubble Rule (SPEC §11.2) places
/// this on the stack instead of raising an error, so subsequent
/// NIL-passthrough words can continue the pipeline.
fn push_undecidable_nil(interp: &mut Interpreter) {
    interp.stack.push(Value::nil_with_reason(NilReason::Undecidable));
    let stack_len = interp.stack.len();
    interp.semantic_registry.normalize_to_stack_len(stack_len);
    interp
        .semantic_registry
        .update_hint_at(stack_len - 1, DisplayHint::Nil);
}

/// Compare two scalar values under an ordering kind. Returns `Ok(Some(bool))`
/// when the comparison decides, `Ok(None)` when the comparison budget
/// exhausts (SPEC §7.4.1) — the caller projects `None` to an Undecidable
/// NIL. Returns `Err(_)` for structurally-non-comparable operands.
///
/// Both-Rational operands take the Fraction fast path (always
/// decidable per SPEC §7.4.1: "the budget value itself is not part
/// of observable semantics; it must be high enough that distinct
/// rationals always decide"). Any non-Rational ExactReal operand
/// routes through `ExactReal::cmp_with_budget` under the
/// `DEFAULT_COMPARISON_BUDGET`; budget exhaustion surfaces as
/// `Ok(None)` here.
fn compare_scalar_pair(
    a_val: &Value,
    b_val: &Value,
    kind: OrderingKind,
) -> Result<Option<bool>> {
    let a = extract_exact_real_for_comparison(a_val)?;
    let b = extract_exact_real_for_comparison(b_val)?;
    Ok(match (a.as_rational(), b.as_rational()) {
        (Some(af), Some(bf)) => Some(kind.apply_to_fraction(af, bf)),
        _ => kind.apply_to_ordering(a.cmp_with_budget(&b, DEFAULT_COMPARISON_BUDGET)),
    })
}

/// Extract an `ExactReal` view of a value's scalar content for
/// comparison. Scalar (`Fraction`-backed) values lift to
/// `ExactReal::Rational`; singleton Vector / Tensor values also
/// project to their sole scalar. Non-scalar shapes and non-numeric
/// kinds error. When a future migration replaces
/// `ValueData::Scalar(Fraction)` with an `ExactReal`-backed
/// representation, this helper is the single point that needs to
/// surface the new variant, and `compare_scalar_pair` / `pairwise_eq`
/// will route it through the budgeted CF path automatically.
fn extract_exact_real_for_comparison(val: &Value) -> Result<ExactReal> {
    let f = extract_scalar_for_comparison(val)?;
    Ok(ExactReal::from_fraction(f))
}

fn extract_scalar_for_comparison(val: &Value) -> Result<Fraction> {
    match &val.data {
        ValueData::Scalar(f) => Ok(f.clone()),
        ValueData::Vector(_) | ValueData::Record { .. } => {
            let tensor = FlatTensor::from_value(val)?;
            if tensor.data.len() != 1 {
                return Err(AjisaiError::create_structure_error(
                    "scalar value",
                    "non-scalar value",
                ));
            }
            Ok(tensor.data[0].clone())
        }
        ValueData::Tensor { data, .. } => {
            if data.len() != 1 {
                return Err(AjisaiError::create_structure_error(
                    "scalar value",
                    "non-scalar value",
                ));
            }
            data.get_small_fraction(0).ok_or_else(|| {
                AjisaiError::create_structure_error("scalar value", "non-scalar value")
            })
        }
        ValueData::Nil => Err(AjisaiError::create_structure_error(
            "scalar value",
            "non-scalar value",
        )),
        ValueData::CodeBlock(_) | ValueData::ProcessHandle(_) | ValueData::SupervisorHandle(_) => {
            Err(AjisaiError::create_structure_error(
                "scalar value",
                "non-scalar value",
            ))
        }
    }
}

/// Check whether every adjacent pair in `items` satisfies `kind`.
/// Returns `Ok(Some(bool))` when the property is decidable for every
/// pair, `Ok(None)` when some pair triggers SPEC §7.4.1's comparison
/// budget short-circuit. SPEC §7.4 requires the entire STAK-mode
/// result to be NIL on the first NIL-producing pair regardless of
/// later pairs.
fn check_all_adjacent_pairs(items: &[Value], kind: OrderingKind) -> Result<Option<bool>> {
    for pair in items.windows(2) {
        match compare_scalar_pair(&pair[0], &pair[1], kind)? {
            Some(true) => continue,
            Some(false) => return Ok(Some(false)),
            None => return Ok(None),
        }
    }
    Ok(Some(true))
}

/// Same three-valued discipline as `check_all_adjacent_pairs` for
/// the EQ relation: `Some(true)` iff every adjacent pair decides
/// equal, `Some(false)` on the first decidedly-unequal pair, `None`
/// on the first §7.4.1 budget-exhausted pair (short-circuit per
/// SPEC §7.4 STAK-mode short-circuit rule). `invert` flips the
/// per-pair predicate to drive `NEQ`'s "all adjacent pairs unequal"
/// semantics.
fn check_all_adjacent_eq(items: &[Value], invert: bool) -> Option<bool> {
    for pair in items.windows(2) {
        match pairwise_eq(&pair[0], &pair[1]) {
            Some(eq) => {
                let pair_ok = if invert { !eq } else { eq };
                if !pair_ok {
                    return Some(false);
                }
            }
            None => return None,
        }
    }
    Some(true)
}

fn apply_binary_comparison(
    interp: &mut Interpreter,
    kind: OrderingKind,
    _op_name: &str,
) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
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

            match compare_scalar_pair(&a_val, &b_val, kind) {
                Ok(Some(b)) => push_boolean_result(interp, b),
                Ok(None) => push_undecidable_nil(interp),
                Err(e) => {
                    if !is_keep_mode {
                        interp.stack.push(a_val);
                        interp.stack.push(b_val);
                    }
                    return Err(e);
                }
            }
            Ok(())
        }

        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = extract_integer_from_value(&count_val)? as usize;

            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Ok(());
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = if is_keep_mode {
                let stack_len = interp.stack.len();
                interp.stack[stack_len - count..].iter().cloned().collect()
            } else {
                interp.stack.drain(interp.stack.len() - count..).collect()
            };

            if items.iter().any(|v| v.is_nil()) {
                interp.stack.push(Value::nil());
                return Ok(());
            }

            match check_all_adjacent_pairs(&items, kind) {
                Ok(Some(decided)) => push_boolean_result(interp, decided),
                Ok(None) => push_undecidable_nil(interp),
                Err(e) => {
                    if !is_keep_mode {
                        interp.stack.extend(items);
                    }
                    interp.stack.push(count_val);
                    return Err(e);
                }
            }
            Ok(())
        }
    }
}

/// Three-valued interval comparison: `Some(true)` and `Some(false)` are
/// decidable; `None` means the two intervals overlap in a way that depends on
/// the unresolved precision of their endpoints. `definitely_true` and
/// `definitely_false` encode the relation under test in terms of interval
/// endpoints; they are independent so that callers can express LT/LTE/GT/GTE
/// without re-deriving each truth table.
fn interval_relation<F1, F2>(
    interp: &mut Interpreter,
    definitely_true: F1,
    definitely_false: F2,
) -> Option<Result<()>>
where
    F1: Fn(&crate::types::interval::Interval, &crate::types::interval::Interval) -> bool,
    F2: Fn(&crate::types::interval::Interval, &crate::types::interval::Interval) -> bool,
{
    if interp.stack.len() < 2 {
        return None;
    }
    let len = interp.stack.len();
    let a = interp.stack[len - 2].clone();
    let b = interp.stack[len - 1].clone();
    let (ai, bi) = match (value_to_interval(&a), value_to_interval(&b)) {
        (Some(ai), Some(bi)) => (ai, bi),
        _ => return None,
    };
    let decided = if definitely_true(&ai, &bi) {
        Some(true)
    } else if definitely_false(&ai, &bi) {
        Some(false)
    } else {
        None
    };
    Some(match decided {
        Some(v) => {
            if interp.consumption_mode != ConsumptionMode::Keep {
                interp.stack.pop();
                interp.stack.pop();
            }
            push_boolean_result(interp, v);
            Ok(())
        }
        None => Err(AjisaiError::from(
            "interval comparison is undecidable with current precision",
        )),
    })
}

pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop
        && nil_passthrough_binary(interp)
    {
        return Ok(());
    }
    if interp.operation_target_mode == OperationTargetMode::StackTop {
        if let Some(res) = interval_relation(interp, |ai, bi| ai.hi.lt(&bi.lo), |ai, bi| ai.lo.ge(&bi.hi)) {
            return res;
        }
    }
    apply_binary_comparison(interp, OrderingKind::Lt, "<")
}

pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop
        && nil_passthrough_binary(interp)
    {
        return Ok(());
    }
    if interp.operation_target_mode == OperationTargetMode::StackTop {
        if let Some(res) = interval_relation(interp, |ai, bi| ai.hi.le(&bi.lo), |ai, bi| ai.lo.gt(&bi.hi)) {
            return res;
        }
    }
    apply_binary_comparison(interp, OrderingKind::Le, "<=")
}

pub fn op_gt(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop
        && nil_passthrough_binary(interp)
    {
        return Ok(());
    }
    if interp.operation_target_mode == OperationTargetMode::StackTop {
        if let Some(res) = interval_relation(interp, |ai, bi| ai.lo.gt(&bi.hi), |ai, bi| ai.hi.le(&bi.lo)) {
            return res;
        }
    }
    apply_binary_comparison(interp, OrderingKind::Gt, ">")
}

pub fn op_gte(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop
        && nil_passthrough_binary(interp)
    {
        return Ok(());
    }
    if interp.operation_target_mode == OperationTargetMode::StackTop {
        if let Some(res) = interval_relation(interp, |ai, bi| ai.lo.ge(&bi.hi), |ai, bi| ai.hi.lt(&bi.lo)) {
            return res;
        }
    }
    apply_binary_comparison(interp, OrderingKind::Ge, ">=")
}

pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    apply_equality(interp, false)
}

pub fn op_neq(interp: &mut Interpreter) -> Result<()> {
    apply_equality(interp, true)
}

/// Three-valued pairwise equality matching the SPEC §7.4.1
/// discipline: `Some(true)` / `Some(false)` for decidable pairs,
/// `None` when budget exhaustion makes the comparison undecidable.
/// `None` is only reachable for scalar pairs where at least one
/// operand is a non-Rational `ExactReal`; the structural Vector /
/// Tensor / Record paths and the singleton-projection paths always
/// decide.
fn pairwise_eq(a_val: &Value, b_val: &Value) -> Option<bool> {
    if a_val.data == b_val.data {
        return Some(true);
    }
    if let (Some(ai), Some(bi)) = (value_to_interval(a_val), value_to_interval(b_val)) {
        if ai.is_exact() && bi.is_exact() {
            return Some(ai.lo == bi.lo);
        }
        return Some(false);
    }
    match (&a_val.data, &b_val.data) {
        (ValueData::Scalar(_), ValueData::Scalar(_)) => scalar_pair_eq(a_val, b_val),
        (ValueData::Scalar(_), ValueData::Vector(children)) if children.len() == 1 => {
            Some(a_val.data == children[0].data)
        }
        (ValueData::Vector(children), ValueData::Scalar(_)) if children.len() == 1 => {
            Some(children[0].data == b_val.data)
        }
        (ValueData::Scalar(_), ValueData::Tensor { .. }) if b_val.len() == 1 => Some(
            b_val
                .child(0)
                .map(|c| a_val.data == c.data)
                .unwrap_or(false),
        ),
        (ValueData::Tensor { .. }, ValueData::Scalar(_)) if a_val.len() == 1 => Some(
            a_val
                .child(0)
                .map(|c| c.data == b_val.data)
                .unwrap_or(false),
        ),
        _ => Some(false),
    }
}

/// Scalar–scalar equality routed through `ExactReal::eq_with_budget`
/// (SPEC §7.4.1). Both-Rational operands decide via `Fraction`
/// `PartialEq` — value equality on canonical reduced rationals.
/// Anything mixing in a non-Rational `ExactReal` runs the budgeted
/// CF expansion; budget exhaustion returns `None` and the caller
/// projects it to the Undecidable NIL.
fn scalar_pair_eq(a_val: &Value, b_val: &Value) -> Option<bool> {
    let a = extract_exact_real_for_comparison(a_val).ok()?;
    let b = extract_exact_real_for_comparison(b_val).ok()?;
    match (a.as_rational(), b.as_rational()) {
        (Some(af), Some(bf)) => Some(af == bf),
        _ => a.eq_with_budget(&b, DEFAULT_COMPARISON_BUDGET),
    }
}

fn apply_equality(interp: &mut Interpreter, invert: bool) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::StackTop
        && nil_passthrough_binary(interp)
    {
        return Ok(());
    }

    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
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
                Some(eq) => push_boolean_result(interp, if invert { !eq } else { eq }),
                None => push_undecidable_nil(interp),
            }
            Ok(())
        }

        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = extract_integer_from_value(&count_val)? as usize;

            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Ok(());
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = if is_keep_mode {
                let stack_len = interp.stack.len();
                interp.stack[stack_len - count..].iter().cloned().collect()
            } else {
                interp.stack.drain(interp.stack.len() - count..).collect()
            };

            if items.iter().any(|v| v.is_nil()) {
                interp.stack.push(Value::nil());
                return Ok(());
            }

            match check_all_adjacent_eq(&items, invert) {
                Some(decided) => push_boolean_result(interp, decided),
                None => push_undecidable_nil(interp),
            }
            Ok(())
        }
    }
}
