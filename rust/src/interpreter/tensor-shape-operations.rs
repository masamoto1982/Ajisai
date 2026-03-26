use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::create_number_value;
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData, MAX_VISIBLE_DIMENSIONS};
use std::rc::Rc;

use num_traits::Zero;

#[derive(Debug, Clone)]
pub(crate) struct FlatTensor {
    pub(crate) data: Vec<Fraction>,
    pub(crate) shape: Vec<usize>,
    pub(crate) strides: Vec<usize>,
}

impl FlatTensor {
    pub(crate) fn from_value(value: &Value) -> Result<Self> {
        match &value.data {
            ValueData::Nil => Err(AjisaiError::from(
                "Tensor conversion requires non-NIL value",
            )),
            ValueData::Scalar(f) => Ok(Self {
                data: vec![f.clone()],
                shape: Vec::new(),
                strides: Vec::new(),
            }),
            ValueData::Vector(_) | ValueData::Record { .. } => {
                let shape: Vec<usize> = value.shape();
                let total_size: usize = value.count_fractions();
                let mut data: Vec<Fraction> = Vec::with_capacity(total_size);
                value.collect_fractions_flat_into(&mut data);
                let strides: Vec<usize> = compute_strides(&shape);
                Ok(Self {
                    data,
                    shape,
                    strides,
                })
            }
            ValueData::CodeBlock(_) => Err(AjisaiError::from(
                "Tensor conversion requires scalar or vector",
            )),
        }
    }

    pub(crate) fn from_shape_and_data(shape: Vec<usize>, data: Vec<Fraction>) -> Result<Self> {
        let expected: usize = if shape.is_empty() {
            1
        } else {
            shape.iter().product()
        };
        if data.len() != expected {
            return Err(AjisaiError::from(format!(
                "Tensor shape/data mismatch: data_len={}, required={}, shape={:?}",
                data.len(),
                expected,
                shape
            )));
        }
        let strides: Vec<usize> = compute_strides(&shape);
        Ok(Self {
            data,
            shape,
            strides,
        })
    }

    pub(crate) fn to_value(&self) -> Value {
        if self.shape.is_empty() {
            return Value::from_fraction(self.data[0].clone());
        }
        build_nested_value(&self.data, &self.shape)
    }
}

pub(crate) fn compute_strides(shape: &[usize]) -> Vec<usize> {
    if shape.is_empty() {
        return Vec::new();
    }
    let mut strides = vec![1; shape.len()];
    for i in (0..shape.len() - 1).rev() {
        strides[i] = strides[i + 1] * shape[i + 1];
    }
    strides
}

fn unravel_index(mut linear: usize, shape: &[usize], strides: &[usize]) -> Vec<usize> {
    if shape.is_empty() {
        return Vec::new();
    }
    let mut out = vec![0; shape.len()];
    for i in 0..shape.len() {
        out[i] = linear / strides[i];
        linear %= strides[i];
    }
    out
}

fn ravel_index(index: &[usize], strides: &[usize]) -> usize {
    index.iter().zip(strides.iter()).map(|(i, s)| i * s).sum()
}

fn project_broadcast_index(
    output_index: &[usize],
    output_shape: &[usize],
    input_shape: &[usize],
) -> Vec<usize> {
    if input_shape.is_empty() {
        return Vec::new();
    }

    let mut projected = vec![0; input_shape.len()];
    let rank_diff = output_shape.len().saturating_sub(input_shape.len());

    for i in 0..input_shape.len() {
        let out_axis = i + rank_diff;
        let out_val = output_index[out_axis];
        projected[i] = if input_shape[i] == 1 { 0 } else { out_val };
    }

    projected
}

pub(crate) fn broadcast_shape(a: &[usize], b: &[usize]) -> Result<Vec<usize>> {
    let rank: usize = a.len().max(b.len());
    let mut out: Vec<usize> = vec![1; rank];

    for i in 0..rank {
        let a_dim = if i >= rank - a.len() {
            a[i - (rank - a.len())]
        } else {
            1
        };
        let b_dim = if i >= rank - b.len() {
            b[i - (rank - b.len())]
        } else {
            1
        };
        if a_dim == b_dim || a_dim == 1 || b_dim == 1 {
            out[i] = a_dim.max(b_dim);
        } else {
            return Err(AjisaiError::from(format!(
                "Cannot broadcast shapes {:?} and {:?}",
                a, b
            )));
        }
    }

    Ok(out)
}

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
    if dim_count > MAX_VISIBLE_DIMENSIONS {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from(format!(
            "Nesting depth limit exceeded: Ajisai supports up to 10 dimensions. Nesting depth {} exceeds the limit.",
            dim_count
        )));
    }

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

    let result_tensor: FlatTensor = FlatTensor::from_shape_and_data(new_shape, input_tensor.data.clone())?;

    if interp.consumption_mode == crate::interpreter::ConsumptionMode::Keep {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
    }

    interp.stack.push(result_tensor.to_value());
    Ok(())
}

pub(crate) fn build_nested_value(data: &[Fraction], shape: &[usize]) -> Value {
    if shape.is_empty() {
        if data.len() == 1 {
            return Value {
                data: ValueData::Scalar(data[0].clone()),
            };
        }
        let children: Vec<Value> = data
            .iter()
            .map(|f| Value::from_fraction(f.clone()))
            .collect();
        return Value::from_children(children);
    }

    if shape.len() == 1 {
        let children: Vec<Value> = data
            .iter()
            .map(|f| Value::from_fraction(f.clone()))
            .collect();
        return Value {
            data: ValueData::Vector(Rc::new(children)),
        };
    }

    let outer_size: usize = shape[0];
    let inner_shape = &shape[1..];
    let inner_size: usize = inner_shape.iter().product();

    let children: Vec<Value> = (0..outer_size)
        .map(|i| {
            let start = i * inner_size;
            let end = start + inner_size;
            build_nested_value(&data[start..end], inner_shape)
        })
        .collect();

    Value {
        data: ValueData::Vector(Rc::new(children)),
    }
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

    if interp.consumption_mode == crate::interpreter::ConsumptionMode::Keep {
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

    let is_keep_mode: bool = interp.consumption_mode == crate::interpreter::ConsumptionMode::Keep;

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

    let is_keep_mode: bool = interp.consumption_mode == crate::interpreter::ConsumptionMode::Keep;

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

    // NILチェック
    if a_val.is_nil() || b_val.is_nil() {
        if !is_keep_mode {
            interp.stack.push(a_val);
            interp.stack.push(b_val);
        }
        return Err(AjisaiError::from("MOD requires vectors or numbers"));
    }

    // ブロードキャスト対応の剰余演算
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

pub(crate) fn apply_binary_broadcast<F>(a: &Value, b: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    if a.is_nil() || b.is_nil() {
        return Err(AjisaiError::from("Cannot broadcast NIL values"));
    }

    let tensor_a = FlatTensor::from_value(a)?;
    let tensor_b = FlatTensor::from_value(b)?;

    let out_shape = broadcast_shape(&tensor_a.shape, &tensor_b.shape)?;
    let out_size: usize = if out_shape.is_empty() {
        1
    } else {
        out_shape.iter().product()
    };
    let out_strides = compute_strides(&out_shape);

    let mut out_data = Vec::with_capacity(out_size);

    for linear in 0..out_size {
        let out_index = unravel_index(linear, &out_shape, &out_strides);

        let a_index = project_broadcast_index(&out_index, &out_shape, &tensor_a.shape);
        let b_index = project_broadcast_index(&out_index, &out_shape, &tensor_b.shape);

        let a_offset = ravel_index(&a_index, &tensor_a.strides);
        let b_offset = ravel_index(&b_index, &tensor_b.strides);

        out_data.push(op(&tensor_a.data[a_offset], &tensor_b.data[b_offset])?);
    }

    let out_tensor = FlatTensor::from_shape_and_data(out_shape, out_data)?;
    Ok(out_tensor.to_value())
}

pub(crate) fn apply_unary_flat<F>(val: &Value, op: F) -> Result<Value>
where
    F: Fn(&Fraction) -> Fraction,
{
    let tensor = FlatTensor::from_value(val)?;
    let result_data: Vec<Fraction> = tensor.data.iter().map(&op).collect();
    let result_tensor = FlatTensor::from_shape_and_data(tensor.shape, result_data)?;
    Ok(result_tensor.to_value())
}

/// In-place variant of `apply_unary_flat`.
///
/// When the linear-consumption optimization hook (`FlowToken::can_update_in_place`)
/// indicates the value is uniquely owned and fully available, this function
/// reuses the data buffer instead of allocating a new Vec<Fraction>.
/// Falls back to `apply_unary_flat` when in-place update is not safe.
pub(crate) fn apply_unary_flat_inplace<F>(val: &Value, op: F, in_place_hint: bool) -> Result<Value>
where
    F: Fn(&Fraction) -> Fraction,
{
    let mut tensor = FlatTensor::from_value(val)?;
    if in_place_hint {
        // Reuse the data buffer: mutate each element in place
        for f in tensor.data.iter_mut() {
            *f = op(f);
        }
        Ok(tensor.to_value())
    } else {
        let result_data: Vec<Fraction> = tensor.data.iter().map(&op).collect();
        let result_tensor = FlatTensor::from_shape_and_data(tensor.shape, result_data)?;
        Ok(result_tensor.to_value())
    }
}

/// In-place variant of `apply_binary_broadcast`.
///
/// When the linear-consumption optimization hook indicates the left operand
/// is uniquely owned and shapes match without broadcasting, this function
/// reuses the left operand's data buffer instead of allocating a new one.
/// Falls back to the standard allocation path otherwise.
pub(crate) fn apply_binary_broadcast_inplace<F>(
    a: &Value,
    b: &Value,
    op: F,
    in_place_hint: bool,
) -> Result<Value>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    if a.is_nil() || b.is_nil() {
        return Err(AjisaiError::from("Cannot broadcast NIL values"));
    }

    let mut tensor_a = FlatTensor::from_value(a)?;
    let tensor_b = FlatTensor::from_value(b)?;

    let out_shape = broadcast_shape(&tensor_a.shape, &tensor_b.shape)?;

    // In-place path: reuse tensor_a's data buffer when shapes match exactly
    // (no broadcasting needed) and the hint allows it
    if in_place_hint && tensor_a.shape == out_shape && tensor_b.shape == out_shape {
        for i in 0..tensor_a.data.len() {
            tensor_a.data[i] = op(&tensor_a.data[i], &tensor_b.data[i])?;
        }
        return Ok(tensor_a.to_value());
    }

    // Standard allocation path
    let out_size: usize = if out_shape.is_empty() {
        1
    } else {
        out_shape.iter().product()
    };
    let out_strides = compute_strides(&out_shape);

    let mut out_data = Vec::with_capacity(out_size);

    for linear in 0..out_size {
        let out_index = unravel_index(linear, &out_shape, &out_strides);

        let a_index = project_broadcast_index(&out_index, &out_shape, &tensor_a.shape);
        let b_index = project_broadcast_index(&out_index, &out_shape, &tensor_b.shape);

        let a_offset = ravel_index(&a_index, &tensor_a.strides);
        let b_offset = ravel_index(&b_index, &tensor_b.strides);

        out_data.push(op(&tensor_a.data[a_offset], &tensor_b.data[b_offset])?);
    }

    let out_tensor = FlatTensor::from_shape_and_data(out_shape, out_data)?;
    Ok(out_tensor.to_value())
}

pub fn op_fill(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: "FILL".into(),
            mode: "Stack".into(),
        });
    }

    // 引数ベクタ [ shape... value ] を取得
    let args_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // NILチェック
    if args_val.is_nil() {
        interp.stack.push(args_val);
        return Err(AjisaiError::from("FILL requires [shape... value] vector"));
    }

    // 要素を取得
    let n = args_val.len();

    // 最低2要素必要
    if n < 2 {
        interp.stack.push(args_val);
        return Err(AjisaiError::from(
            "FILL requires [shape... value] (at least 2 elements)",
        ));
    }

    // 最後の要素が埋める値
    let fill_value = match args_val.get_child(n - 1).and_then(|v| v.as_scalar()) {
        Some(f) => f.clone(),
        None => {
            interp.stack.push(args_val);
            return Err(AjisaiError::from("FILL value must be a scalar"));
        }
    };

    // それより前の要素が形状
    let shape_len = n - 1;
    if shape_len > MAX_VISIBLE_DIMENSIONS {
        interp.stack.push(args_val);
        return Err(AjisaiError::from(format!(
            "Nesting depth limit exceeded: Ajisai supports up to 10 dimensions. Nesting depth {} exceeds the limit.",
            shape_len
        )));
    }

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

    // データを生成
    let total_size: usize = shape.iter().product();
    let data: Vec<Fraction> = (0..total_size).map(|_| fill_value.clone()).collect();

    // 再帰的構造を構築
    let result = build_nested_value(&data, &shape);

    // 保持モードの場合は元の値を戻す
    if interp.consumption_mode == crate::interpreter::ConsumptionMode::Keep {
        interp.stack.push(args_val);
    }

    interp.stack.push(result);
    Ok(())
}
