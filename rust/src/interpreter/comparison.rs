use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::interval_ops::value_to_interval;
use crate::interpreter::tensor_ops::FlatTensor;
use crate::interpreter::value_extraction_helpers::{
    extract_integer_from_value, nil_passthrough_binary,
};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::{DisplayHint, Value, ValueData};

/// One of the four ordering comparisons. Carries the dispatch
/// decision through the SCALAR-comparison helper so the helper can
/// keep the Fraction fast path for the common case while leaving a
/// hook for Phase 7's ExactReal `cmp_with_budget` path.
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
/// Phase 6 implementation keeps the existing Fraction fast path
/// because `ValueData::Scalar` is still `Fraction`-backed; the dispatch
/// shape is in place so Phase 7 can route the non-Rational ExactReal
/// case through `ExactReal::cmp_with_budget` without touching the
/// caller-side code paths or the STAK-mode short-circuit.
fn compare_scalar_pair(
    a_val: &Value,
    b_val: &Value,
    kind: OrderingKind,
) -> Result<Option<bool>> {
    let af = extract_scalar_for_comparison(a_val)?;
    let bf = extract_scalar_for_comparison(b_val)?;
    Ok(Some(kind.apply_to_fraction(&af, &bf)))
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

fn check_all_adjacent_equal(items: &[Value]) -> bool {
    items.windows(2).all(|pair| pair[0].data == pair[1].data)
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

fn pairwise_eq(a_val: &Value, b_val: &Value) -> bool {
    if a_val.data == b_val.data {
        return true;
    }
    if let (Some(ai), Some(bi)) = (value_to_interval(a_val), value_to_interval(b_val)) {
        if ai.is_exact() && bi.is_exact() {
            return ai.lo == bi.lo;
        }
        return false;
    }
    match (&a_val.data, &b_val.data) {
        (ValueData::Scalar(_), ValueData::Vector(children)) if children.len() == 1 => {
            a_val.data == children[0].data
        }
        (ValueData::Vector(children), ValueData::Scalar(_)) if children.len() == 1 => {
            children[0].data == b_val.data
        }
        (ValueData::Scalar(_), ValueData::Tensor { .. }) if b_val.len() == 1 => b_val
            .child(0)
            .map(|c| a_val.data == c.data)
            .unwrap_or(false),
        (ValueData::Tensor { .. }, ValueData::Scalar(_)) if a_val.len() == 1 => a_val
            .child(0)
            .map(|c| c.data == b_val.data)
            .unwrap_or(false),
        _ => false,
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

            let eq: bool = pairwise_eq(&a_val, &b_val);
            push_boolean_result(interp, if invert { !eq } else { eq });
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

            let property: bool = if invert {
                items.windows(2).all(|pair| !pairwise_eq(&pair[0], &pair[1]))
            } else {
                check_all_adjacent_equal(&items)
            };
            push_boolean_result(interp, property);
            Ok(())
        }
    }
}
