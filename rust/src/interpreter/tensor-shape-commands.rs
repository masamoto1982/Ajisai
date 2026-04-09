use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::create_number_value;
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::Value;

use super::tensor_ops::{
    apply_binary_broadcast, apply_unary_flat, build_nested_value, FlatTensor,
};

fn apply_tensor_metadata(
    interp: &mut Interpreter,
    word: &str,
    mapper: fn(&Value) -> Value,
) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: word.into(),
            mode: "Stack".into(),
        });
    }

    let is_keep_mode: bool = interp.consumption_mode == ConsumptionMode::Keep;
    let value: Value = if is_keep_mode {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let result: Value = mapper(&value);
    interp.stack.push(result);
    Ok(())
}

fn compute_shape_of_value(value: &Value) -> Value {
    if value.is_nil() {
        return Value::nil();
    }

    if !value.is_vector() {
        return Value::from_vector(vec![]);
    }

    let shape_values: Vec<Value> = value
        .shape()
        .iter()
        .map(|&n| Value::from_number(Fraction::from(n as i64)))
        .collect();
    Value::from_vector(shape_values)
}

fn compute_rank_of_value(value: &Value) -> Value {
    if value.is_nil() {
        return Value::nil();
    }

    let rank: i64 = if value.is_vector() {
        value.shape().len() as i64
    } else {
        0
    };
    create_number_value(Fraction::from(rank))
}

pub fn op_shape(interp: &mut Interpreter) -> Result<()> {
    apply_tensor_metadata(interp, "SHAPE", compute_shape_of_value)
}

pub fn op_rank(interp: &mut Interpreter) -> Result<()> {
    apply_tensor_metadata(interp, "RANK", compute_rank_of_value)
}

pub fn op_reshape(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: "RESHAPE".into(),
            mode: "Stack".into(),
        });
    }

    let shape_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let data_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if !shape_val.is_vector() && !shape_val.is_nil() {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from("RESHAPE requires shape as vector"));
    }

    let dim_count: usize = shape_val.len();

    let mut new_shape: Vec<usize> = Vec::with_capacity(dim_count);
    for i in 0..dim_count {
        let dim = match shape_val
            .get_child(i)
            .unwrap()
            .as_scalar()
            .and_then(|f| f.as_usize())
        {
            Some(d) => d,
            None => {
                interp.stack.push(data_val);
                interp.stack.push(shape_val);
                return Err(AjisaiError::from(
                    "Shape dimensions must be positive integers",
                ));
            }
        };
        new_shape.push(dim);
    }

    if data_val.is_nil() {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from("RESHAPE requires data as vector"));
    }

    let input_tensor: FlatTensor = match FlatTensor::from_value(&data_val) {
        Ok(t) => t,
        Err(err) => {
            interp.stack.push(data_val);
            interp.stack.push(shape_val);
            return Err(err);
        }
    };

    let required_size: usize = if new_shape.is_empty() {
        1
    } else {
        new_shape.iter().product()
    };
    if input_tensor.data.len() != required_size {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from(format!(
            "RESHAPE failed: data length {} doesn't match shape {:?} (requires {})",
            input_tensor.data.len(),
            new_shape,
            required_size
        )));
    }

    let result_tensor: FlatTensor = FlatTensor::from_shape_and_data(new_shape, input_tensor.data)?;

    if interp.consumption_mode == ConsumptionMode::Keep {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
    }

    interp.stack.push(result_tensor.to_value());
    Ok(())
}

pub fn op_transpose(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: "TRANSPOSE".into(),
            mode: "Stack".into(),
        });
    }

    let val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if val.is_nil() {
        interp.stack.push(Value::nil());
        return Ok(());
    }

    let tensor: FlatTensor = match FlatTensor::from_value(&val) {
        Ok(t) => t,
        Err(err) => {
            interp.stack.push(val);
            return Err(err);
        }
    };

    if tensor.shape.len() != 2 {
        interp.stack.push(val);
        return Err(AjisaiError::from("TRANSPOSE requires 2D vector"));
    }

    let rows: usize = tensor.shape[0];
    let cols: usize = tensor.shape[1];

    let mut transposed: Vec<Fraction> = Vec::with_capacity(tensor.data.len());
    for j in 0..cols {
        for i in 0..rows {
            transposed.push(tensor.data[i * cols + j].clone());
        }
    }

    let result_tensor: FlatTensor = FlatTensor::from_shape_and_data(vec![cols, rows], transposed)?;

    if interp.consumption_mode == ConsumptionMode::Keep {
        interp.stack.push(val);
    }

    interp.stack.push(result_tensor.to_value());
    Ok(())
}

fn apply_unary_math<F>(interp: &mut Interpreter, op: F, op_name: &str) -> Result<()>
where
    F: Fn(&Fraction) -> Fraction,
{
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: op_name.to_string(),
            mode: "Stack".into(),
        });
    }

    let is_keep_mode: bool = interp.consumption_mode == ConsumptionMode::Keep;

    let val: Value = if is_keep_mode {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    if val.is_nil() {
        if !is_keep_mode {
            interp.stack.push(val);
        }
        return Err(AjisaiError::from(format!(
            "{} requires number or vector",
            op_name
        )));
    }

    if val.is_scalar() {
        if let Some(f) = val.as_scalar() {
            let result: Fraction = op(f);
            interp.stack.push(create_number_value(result));
            return Ok(());
        }
    }

    if val.is_vector() {
        match apply_unary_flat(&val, op) {
            Ok(result) => {
                interp.stack.push(result);
                return Ok(());
            }
            Err(_) => {
                if !is_keep_mode {
                    interp.stack.push(val);
                }
                return Err(AjisaiError::from(format!(
                    "{} requires number or vector",
                    op_name
                )));
            }
        }
    }

    if !is_keep_mode {
        interp.stack.push(val);
    }
    Err(AjisaiError::from(format!(
        "{} requires number or vector",
        op_name
    )))
}

pub fn op_floor(interp: &mut Interpreter) -> Result<()> {
    apply_unary_math(interp, |f| f.floor(), "FLOOR")
}

pub fn op_ceil(interp: &mut Interpreter) -> Result<()> {
    apply_unary_math(interp, |f| f.ceil(), "CEIL")
}

pub fn op_round(interp: &mut Interpreter) -> Result<()> {
    apply_unary_math(interp, |f| f.round(), "ROUND")
}

pub fn op_mod(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: "MOD".into(),
            mode: "Stack".into(),
        });
    }

    let is_keep_mode: bool = interp.consumption_mode == ConsumptionMode::Keep;

    let b_val: Value = if is_keep_mode {
        interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    let a_val = if is_keep_mode {
        let stack_len = interp.stack.len();
        if stack_len < 2 {
            return Err(AjisaiError::StackUnderflow);
        }
        interp.stack[stack_len - 2].clone()
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    if a_val.is_nil() || b_val.is_nil() {
        if !is_keep_mode {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
        }
        return Err(AjisaiError::from("MOD requires vectors or numbers"));
    }

    let result = apply_binary_broadcast(&a_val, &b_val, |x, y| {
        if y.is_zero() {
            Err(AjisaiError::from("Modulo by zero"))
        } else {
            Ok(x.modulo(y))
        }
    });

    match result {
        Ok(r) => {
            interp.stack.push(r);
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

pub fn op_fill(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: "FILL".into(),
            mode: "Stack".into(),
        });
    }

    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if args_val.is_nil() {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("FILL requires [shape... value] vector"));
    }

    let n = args_val.len();

    if n < 2 {
        interp.stack.push(args_val);
        return Err(AjisaiError::from(
            "FILL requires [shape... value] (at least 2 elements)",
        ));
    }

    let fill_value = match args_val.get_child(n - 1).and_then(|v| v.as_scalar()) {
        Some(f) => f.clone(),
        None => {
            interp.stack.push(args_val);
            return Err(AjisaiError::from("FILL value must be a scalar"));
        }
    };

    let shape_len = n - 1;

    let mut shape = Vec::with_capacity(shape_len);
    for i in 0..shape_len {
        let dim = match args_val
            .get_child(i)
            .unwrap()
            .as_scalar()
            .and_then(|f| f.as_usize())
        {
            Some(d) if d > 0 => d,
            Some(_) | None => {
                interp.stack.push(args_val);
                return Err(AjisaiError::from(
                    "RESHAPE: expected positive integer dimensions, got invalid dimension",
                ));
            }
        };
        shape.push(dim);
    }

    let total_size: usize = shape.iter().product();
    let data: Vec<Fraction> = (0..total_size).map(|_| fill_value.clone()).collect();

    let result = build_nested_value(&data, &shape);

    if interp.consumption_mode == ConsumptionMode::Keep {
        interp.stack.push(args_val);
    }

    interp.stack.push(result);
    Ok(())
}
