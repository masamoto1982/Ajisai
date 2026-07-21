use crate::error::{AjisaiError, Result};
use crate::interpreter::interpreter_core::RuntimeMetrics;
use crate::interpreter::logic_kleene::{self, Ternary};
use crate::interpreter::tensor_ops::{
    apply_binary_broadcast_with_metrics, apply_unary_flat_with_metrics,
};
use crate::interpreter::value_extraction_helpers::extract_count_from_value;
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::Stack;
use crate::types::Value;

/// Whether an operand forces the scalar three-valued (K3) path rather
/// than element-wise tensor broadcast: an operational NIL, the logical
/// `Unknown` (U), or a definite Boolean truth value. A definite Boolean
/// must route through K3 so that `AND`/`OR` of truth values yield a Boolean
/// result (not a 0/1 number). When no operand is truth-valued, `AND`/`OR`
/// keep their element-wise numeric broadcast semantics over numeric vectors.
fn forces_k3_path(value: &Value) -> bool {
    value.is_nil() || value.is_unknown() || value.as_truth().is_some()
}

#[derive(Clone, Copy)]
enum K3BinaryOp {
    Meet,
    Join,
}

impl K3BinaryOp {
    fn combine(self, a: Ternary, b: Ternary) -> Ternary {
        match self {
            K3BinaryOp::Meet => logic_kleene::meet_k3(a, b),
            K3BinaryOp::Join => logic_kleene::join_k3(a, b),
        }
    }

    fn identity(self) -> Ternary {
        match self {
            K3BinaryOp::Meet => Ternary::True,
            K3BinaryOp::Join => Ternary::False,
        }
    }

    fn absorbing(self) -> Ternary {
        match self {
            K3BinaryOp::Meet => Ternary::False,
            K3BinaryOp::Join => Ternary::True,
        }
    }
}

fn compute_k3_binary_value(op: K3BinaryOp, a: &Value, b: &Value) -> Value {
    logic_kleene::into_value_with_diagnosis(
        op.combine(Ternary::classify(a), Ternary::classify(b)),
        &[a, b],
    )
}

fn fold_k3_values(op: K3BinaryOp, items: &[Value]) -> Value {
    let mut acc = op.identity();
    for value in items {
        acc = op.combine(acc, Ternary::classify(value));
        if acc == op.absorbing() {
            break;
        }
    }
    let refs: Vec<&Value> = items.iter().collect();
    logic_kleene::into_value_with_diagnosis(acc, &refs)
}

fn compute_inverted_fraction(f: &Fraction) -> Fraction {
    if f.is_zero() {
        Fraction::from(1)
    } else {
        Fraction::from(0)
    }
}

fn compute_inverted_value(val: &Value, metrics: Option<&mut RuntimeMetrics>) -> Result<Value> {
    // The logical Unknown (¬U = U) and operational NIL (¬NIL = NIL) cases
    // route through the canonical K3 NOT table (SPEC §7.5). Checked before
    // the scalar path because U is represented as a NIL node; a plain
    // numeric/boolean scalar keeps its existing 0↔1 inversion below.
    if val.is_unknown() || val.is_nil() {
        // ¬U = U: carry the operand's comparison diagnosis (agreedPrefix)
        // over to the result U (SPEC §4.5.0 / §7.4.1).
        return Ok(logic_kleene::into_value_with_diagnosis(
            logic_kleene::involution_k3(Ternary::classify(val)),
            &[val],
        ));
    }
    // A definite Boolean inverts to the opposite Boolean (¬T=F, ¬F=T), staying
    // a truth value rather than collapsing to a 0/1 number.
    if let Some(b) = val.as_truth() {
        return Ok(Value::from_bool(!b));
    }
    if let Some(f) = val.as_scalar() {
        return Ok(Value::from_fraction(compute_inverted_fraction(f)));
    }
    apply_unary_flat_with_metrics(val, compute_inverted_fraction, metrics)
}

pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val = if is_keep_mode {
                interp
                    .stack
                    .last()
                    .cloned()
                    .ok_or(AjisaiError::StackUnderflow)?
            } else {
                interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
            };

            let result = match compute_inverted_value(&val, Some(&mut interp.runtime_metrics)) {
                Ok(v) => v,
                Err(e) => {
                    if !is_keep_mode {
                        interp.stack.push(val);
                    }
                    return Err(e);
                }
            };

            interp.stack.push(result);
            Ok(())
        }
        OperationTargetMode::Stack => {
            let source: Vec<Value> = interp.stack.to_vec();
            let mut results: Vec<Value> = Vec::with_capacity(source.len());
            for value in &source {
                results.push(compute_inverted_value(
                    value,
                    Some(&mut interp.runtime_metrics),
                )?);
            }

            if is_keep_mode {
                interp.stack.extend(results);
            } else {
                interp.stack = Stack::from_values(results);
            }
            Ok(())
        }
    }
}

pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let (a_val, b_val) = if is_keep_mode {
                let stack_len = interp.stack.len();
                (
                    interp.stack[stack_len - 2].clone(),
                    interp.stack[stack_len - 1].clone(),
                )
            } else {
                let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                (a_val, b_val)
            };

            // K3 (SPEC §7.5) when either operand is an operational NIL or
            // the logical Unknown (U); otherwise keep element-wise broadcast.
            if forces_k3_path(&a_val) || forces_k3_path(&b_val) {
                interp
                    .stack
                    .push(compute_k3_binary_value(K3BinaryOp::Meet, &a_val, &b_val));
                return Ok(());
            }

            let result = apply_binary_broadcast_with_metrics(
                &a_val,
                &b_val,
                |a, b| {
                    let a_truthy = !a.is_zero();
                    let b_truthy = !b.is_zero();
                    Ok(Fraction::from(if a_truthy && b_truthy { 1 } else { 0 }))
                },
                Some(&mut interp.runtime_metrics),
            )?;
            interp.stack.push(result);
            Ok(())
        }

        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = extract_count_from_value(&count_val)?;

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
                interp.stack.as_slice()[stack_len - count..]
                    .iter()
                    .cloned()
                    .collect()
            } else {
                interp.stack.drain(interp.stack.len() - count..).collect()
            };

            // STAK-mode K3 fold (SPEC §7.5): `AND = meet_K3` with F as
            // the absorbing element and NIL-over-U priority centralized in
            // `logic_kleene`.
            interp.stack.push(fold_k3_values(K3BinaryOp::Meet, &items));
            Ok(())
        }
    }
}

pub fn op_or(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let (a_val, b_val) = if is_keep_mode {
                let stack_len = interp.stack.len();
                (
                    interp.stack[stack_len - 2].clone(),
                    interp.stack[stack_len - 1].clone(),
                )
            } else {
                let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
                (a_val, b_val)
            };

            // K3 (SPEC §7.5) when either operand is an operational NIL or
            // the logical Unknown (U); otherwise keep element-wise broadcast.
            if forces_k3_path(&a_val) || forces_k3_path(&b_val) {
                interp
                    .stack
                    .push(compute_k3_binary_value(K3BinaryOp::Join, &a_val, &b_val));
                return Ok(());
            }

            let result = apply_binary_broadcast_with_metrics(
                &a_val,
                &b_val,
                |a, b| {
                    let a_truthy = !a.is_zero();
                    let b_truthy = !b.is_zero();
                    Ok(Fraction::from(if a_truthy || b_truthy { 1 } else { 0 }))
                },
                Some(&mut interp.runtime_metrics),
            )?;
            interp.stack.push(result);
            Ok(())
        }

        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = extract_count_from_value(&count_val)?;

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
                interp.stack.as_slice()[stack_len - count..]
                    .iter()
                    .cloned()
                    .collect()
            } else {
                interp.stack.drain(interp.stack.len() - count..).collect()
            };

            // STAK-mode K3 fold (SPEC §7.5): `OR = join_K3` with T as
            // the absorbing element and NIL-over-U priority centralized in
            // `logic_kleene`.
            interp.stack.push(fold_k3_values(K3BinaryOp::Join, &items));
            Ok(())
        }
    }
}
