// rust/src/interpreter/arithmetic.rs
//
// Unified Fraction Architecture: all values are fractions. No type checking.

use crate::interpreter::{Interpreter, OperationTargetMode, ConsumptionMode};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{get_integer_from_value, get_operands, push_result};
use crate::types::{Value, ValueData};
use crate::types::fraction::Fraction;

fn extract_single_scalar(val: &Value) -> Option<&Fraction> {
    match &val.data {
        ValueData::Scalar(f) => Some(f),
        ValueData::Vector(children) if children.len() == 1 => {
            extract_single_scalar(&children[0])
        }
        _ => None
    }
}

fn is_single_scalar(val: &Value) -> bool {
    extract_single_scalar(val).is_some()
}

fn broadcast_binary_op<F>(a: &Value, b: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    match (&a.data, &b.data) {
        (ValueData::Nil, ValueData::Nil) => Ok(Value::nil()),

        (ValueData::Nil, _) | (_, ValueData::Nil) => {
            Err(AjisaiError::from("Cannot operate with NIL"))
        }

        (ValueData::CodeBlock(_), _) | (_, ValueData::CodeBlock(_)) => {
            Err(AjisaiError::from("Cannot perform arithmetic on code blocks"))
        }

        (ValueData::Scalar(fa), ValueData::Scalar(fb)) => {
            Ok(Value::from_fraction(op(fa, fb)?))
        }

        (ValueData::Scalar(fa), ValueData::Vector(vb)) => {
            let result: Result<Vec<Value>> = vb.iter()
                .map(|bi| broadcast_binary_op(&Value::from_fraction(fa.clone()), bi, op))
                .collect();
            Ok(Value::from_children(result?))
        }

        (ValueData::Vector(va), ValueData::Scalar(fb)) => {
            let result: Result<Vec<Value>> = va.iter()
                .map(|ai| broadcast_binary_op(ai, &Value::from_fraction(fb.clone()), op))
                .collect();
            Ok(Value::from_children(result?))
        }

        (ValueData::Vector(va), ValueData::Vector(vb)) => {
            if va.len() != vb.len() {
                return Err(AjisaiError::VectorLengthMismatch { len1: va.len(), len2: vb.len() });
            }
            let result: Result<Vec<Value>> = va.iter().zip(vb.iter())
                .map(|(ai, bi)| broadcast_binary_op(ai, bi, op))
                .collect();
            Ok(Value::from_children(result?))
        }

    }
}

fn binary_arithmetic_op<F>(interp: &mut Interpreter, op: F, op_name: &str) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            // get_operands を使用してオペランドを取得
            let operands = get_operands(interp, 2)?;
            let a_val = &operands[0];
            let b_val = &operands[1];

            let result = match broadcast_binary_op(a_val, b_val, op) {
                Ok(r) => r,
                Err(e) => {
                    // エラー時、Consume モードの場合は値を復元
                    if !is_keep_mode {
                        for val in operands {
                            interp.stack.push(val);
                        }
                    }
                    return Err(e);
                }
            };

            if !interp.disable_no_change_check && (result == *a_val || result == *b_val) {
                // エラー時、Consume モードの場合は値を復元
                if !is_keep_mode {
                    for val in operands {
                        interp.stack.push(val);
                    }
                }
                return Err(AjisaiError::NoChange { word: op_name.into() });
            }

            push_result(interp, result);
        },

        OperationTargetMode::Stack => {
            // Stack モードでは、カウント値は常に消費する
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if count == 0 {
                interp.stack.push(count_val);
                return Err(AjisaiError::NoChange { word: op_name.into() });
            }

            if count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::NoChange { word: op_name.into() });
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            // Keep モードの場合は peek、Consume モードの場合は drain
            let items: Vec<Value> = if is_keep_mode {
                let stack_len = interp.stack.len();
                interp.stack[stack_len - count..].iter().cloned().collect()
            } else {
                interp.stack.drain(interp.stack.len() - count..).collect()
            };

            if items.iter().any(|v| !is_single_scalar(v)) {
                if !is_keep_mode {
                    interp.stack.extend(items);
                }
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK mode requires single-element values"));
            }

            let first_scalar = extract_single_scalar(&items[0]).unwrap().clone();
            let mut acc = first_scalar.clone();
            let original_first = acc.clone();

            for item in items.iter().skip(1) {
                if let Some(f) = extract_single_scalar(item) {
                    acc = op(&acc, f)?;
                }
            }

            if !interp.disable_no_change_check && acc == original_first {
                if !is_keep_mode {
                    interp.stack.extend(items);
                }
                interp.stack.push(count_val);
                return Err(AjisaiError::NoChange { word: op_name.into() });
            }

            push_result(interp, Value::from_fraction(acc));
        }
    }
    Ok(())
}

pub fn op_add(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| Ok(a.add(b)), "+")
}

pub fn op_sub(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| Ok(a.sub(b)), "-")
}

pub fn op_mul(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.operation_target_mode == OperationTargetMode::StackTop {
        let operands = get_operands(interp, 2)?;
        let a_val = &operands[0];
        let b_val = &operands[1];

        if a_val.is_nil() || b_val.is_nil() {
            if !is_keep_mode {
                for val in operands {
                    interp.stack.push(val);
                }
            }
            return Err(AjisaiError::from("Cannot operate with NIL"));
        }

        let result = apply_optimized_mul(a_val, b_val);

        match result {
            Ok(r) => {
                if !interp.disable_no_change_check && (r == *a_val || r == *b_val) {
                    if !is_keep_mode {
                        for val in operands {
                            interp.stack.push(val);
                        }
                    }
                    return Err(AjisaiError::NoChange { word: "*".into() });
                }
                push_result(interp, r);
                return Ok(());
            }
            Err(e) => {
                if !is_keep_mode {
                    for val in operands {
                        interp.stack.push(val);
                    }
                }
                return Err(e);
            }
        }
    }

    binary_arithmetic_op(interp, |a, b| Ok(a.mul(b)), "*")
}

fn apply_optimized_mul(a: &Value, b: &Value) -> Result<Value> {
    match (&a.data, &b.data) {
        (ValueData::Scalar(fa), ValueData::Scalar(fb)) => {
            Ok(Value::from_fraction(fa.mul(fb)))
        }
        (ValueData::Scalar(scalar), ValueData::Vector(vec)) => {
            if scalar.is_integer() {
                let result: Vec<Value> = vec.iter()
                    .map(|v| apply_scalar_mul_to_value(scalar, v))
                    .collect();
                Ok(Value::from_children(result))
            } else {
                broadcast_binary_op(a, b, |x, y| Ok(x.mul(y)))
            }
        }
        (ValueData::Vector(vec), ValueData::Scalar(scalar)) => {
            if scalar.is_integer() {
                let result: Vec<Value> = vec.iter()
                    .map(|v| apply_scalar_mul_to_value(scalar, v))
                    .collect();
                Ok(Value::from_children(result))
            } else {
                broadcast_binary_op(a, b, |x, y| Ok(x.mul(y)))
            }
        }
        (ValueData::Vector(va), ValueData::Vector(vb)) => {
            if va.len() != vb.len() {
                return Err(AjisaiError::VectorLengthMismatch { len1: va.len(), len2: vb.len() });
            }
            let result: Result<Vec<Value>> = va.iter().zip(vb.iter())
                .map(|(ai, bi)| apply_optimized_mul(ai, bi))
                .collect();
            Ok(Value::from_children(result?))
        }
        _ => Err(AjisaiError::from("Cannot multiply NIL")),
    }
}

fn apply_scalar_mul_to_value(scalar: &Fraction, val: &Value) -> Value {
    match &val.data {
        ValueData::Scalar(f) => {
            if scalar.is_integer() {
                Value::from_fraction(f.mul_by_integer(scalar))
            } else {
                Value::from_fraction(f.mul(scalar))
            }
        }
        ValueData::Vector(children) => {
            let new_children: Vec<Value> = children.iter()
                .map(|c| apply_scalar_mul_to_value(scalar, c))
                .collect();
            Value::from_children(new_children)
        }
        ValueData::Nil => val.clone(),
        ValueData::CodeBlock(_) => val.clone(),
    }
}

pub fn op_div(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    if interp.operation_target_mode == OperationTargetMode::StackTop {
        let operands = get_operands(interp, 2)?;
        let a_val = &operands[0];
        let b_val = &operands[1];

        if a_val.is_nil() || b_val.is_nil() {
            if !is_keep_mode {
                for val in operands {
                    interp.stack.push(val);
                }
            }
            return Err(AjisaiError::from("Cannot operate with NIL"));
        }

        let result = apply_optimized_div(a_val, b_val);

        match result {
            Ok(r) => {
                if !interp.disable_no_change_check && (r == *a_val || r == *b_val) {
                    if !is_keep_mode {
                        for val in operands {
                            interp.stack.push(val);
                        }
                    }
                    return Err(AjisaiError::NoChange { word: "/".into() });
                }
                push_result(interp, r);
                return Ok(());
            }
            Err(e) => {
                if !is_keep_mode {
                    for val in operands {
                        interp.stack.push(val);
                    }
                }
                return Err(e);
            }
        }
    }

    binary_arithmetic_op(interp, |a, b| {
        if b.is_zero() {
            Err(AjisaiError::DivisionByZero)
        } else {
            Ok(a.div(b))
        }
    }, "/")
}

fn apply_optimized_div(a: &Value, b: &Value) -> Result<Value> {
    match (&a.data, &b.data) {
        (ValueData::Scalar(fa), ValueData::Scalar(fb)) => {
            if fb.is_zero() {
                return Err(AjisaiError::DivisionByZero);
            }
            Ok(Value::from_fraction(fa.div(fb)))
        }
        (ValueData::Vector(vec), ValueData::Scalar(scalar)) => {
            if scalar.is_zero() {
                return Err(AjisaiError::DivisionByZero);
            }
            if scalar.is_integer() {
                let result: Result<Vec<Value>> = vec.iter()
                    .map(|v| apply_scalar_div_to_value(v, scalar))
                    .collect();
                Ok(Value::from_children(result?))
            } else {
                broadcast_binary_op(a, b, |x, y| {
                    if y.is_zero() { Err(AjisaiError::DivisionByZero) } else { Ok(x.div(y)) }
                })
            }
        }
        (ValueData::Scalar(scalar), ValueData::Vector(vec)) => {
            let result: Result<Vec<Value>> = vec.iter()
                .map(|v| apply_div_scalar_by_value(scalar, v))
                .collect();
            Ok(Value::from_children(result?))
        }
        (ValueData::Vector(va), ValueData::Vector(vb)) => {
            if va.len() != vb.len() {
                return Err(AjisaiError::VectorLengthMismatch { len1: va.len(), len2: vb.len() });
            }
            let result: Result<Vec<Value>> = va.iter().zip(vb.iter())
                .map(|(ai, bi)| apply_optimized_div(ai, bi))
                .collect();
            Ok(Value::from_children(result?))
        }
        _ => Err(AjisaiError::from("Cannot divide NIL")),
    }
}

fn apply_scalar_div_to_value(val: &Value, scalar: &Fraction) -> Result<Value> {
    match &val.data {
        ValueData::Scalar(f) => {
            if scalar.is_integer() {
                Ok(Value::from_fraction(f.div_by_integer(scalar)))
            } else {
                Ok(Value::from_fraction(f.div(scalar)))
            }
        }
        ValueData::Vector(children) => {
            let new_children: Result<Vec<Value>> = children.iter()
                .map(|c| apply_scalar_div_to_value(c, scalar))
                .collect();
            Ok(Value::from_children(new_children?))
        }
        ValueData::Nil => Ok(val.clone()),
        ValueData::CodeBlock(_) => Ok(val.clone()),
    }
}

fn apply_div_scalar_by_value(scalar: &Fraction, val: &Value) -> Result<Value> {
    match &val.data {
        ValueData::Scalar(f) => {
            if f.is_zero() {
                return Err(AjisaiError::DivisionByZero);
            }
            Ok(Value::from_fraction(scalar.div(f)))
        }
        ValueData::Vector(children) => {
            let new_children: Result<Vec<Value>> = children.iter()
                .map(|c| apply_div_scalar_by_value(scalar, c))
                .collect();
            Ok(Value::from_children(new_children?))
        }
        ValueData::Nil => Ok(val.clone()),
        ValueData::CodeBlock(_) => Ok(val.clone()),
    }
}
