use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::wrap_number;
use crate::interpreter::{Interpreter, OperationTargetMode};
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
                let shape = value.shape();
                let data = value.flatten_fractions();
                let strides = compute_strides(&shape);
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
        let strides = compute_strides(&shape);
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
    let rank = a.len().max(b.len());
    let mut out = vec![1; rank];

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

pub fn op_shape(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: "SHAPE".into(),
            mode: "Stack".into(),
        });
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // NILの場合: Map型 → NIL伝播
    if val.is_nil() {
        interp.stack.push(Value::nil());
        return Ok(());
    }

    // ベクタの場合
    if val.is_vector() {
        let shape_vec = val.shape();

        let shape_values: Vec<Value> = shape_vec
            .iter()
            .map(|&n| Value::from_number(Fraction::from(n as i64)))
            .collect();

        // 保持モードの場合は元の値を戻す
        if interp.consumption_mode == crate::interpreter::ConsumptionMode::Keep {
            interp.stack.push(val);
        }

        interp.stack.push(Value::from_vector(shape_values));
        return Ok(());
    }

    // スカラーの場合: 形状は空（0次元）
    if interp.consumption_mode == crate::interpreter::ConsumptionMode::Keep {
        interp.stack.push(val);
    }
    interp.stack.push(Value::from_vector(vec![]));
    Ok(())
}

pub fn op_rank(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: "RANK".into(),
            mode: "Stack".into(),
        });
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // NILの場合: Map型 → NIL伝播
    if val.is_nil() {
        interp.stack.push(Value::nil());
        return Ok(());
    }

    // ベクタの場合
    if val.is_vector() {
        let shape = val.shape();
        let r = shape.len();
        let rank_frac = Fraction::from(r as i64);

        // 保持モードの場合は元の値を戻す
        if interp.consumption_mode == crate::interpreter::ConsumptionMode::Keep {
            interp.stack.push(val);
        }

        interp.stack.push(wrap_number(rank_frac));
        return Ok(());
    }

    // スカラーの場合: ランクは0
    let rank_frac = Fraction::from(0i64);
    if interp.consumption_mode == crate::interpreter::ConsumptionMode::Keep {
        interp.stack.push(val);
    }
    interp.stack.push(wrap_number(rank_frac));
    Ok(())
}

pub fn op_reshape(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: "RESHAPE".into(),
            mode: "Stack".into(),
        });
    }

    let shape_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let data_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if !shape_val.is_vector() && !shape_val.is_nil() {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from("RESHAPE requires shape as vector"));
    }

    let dim_count = shape_val.len();
    if dim_count > MAX_VISIBLE_DIMENSIONS {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
        return Err(AjisaiError::from(format!(
            "Nesting depth limit exceeded: Ajisai supports up to 10 dimensions. Nesting depth {} exceeds the limit.",
            dim_count
        )));
    }

    let mut new_shape = Vec::with_capacity(dim_count);
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

    let input_tensor = match FlatTensor::from_value(&data_val) {
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

    let result_tensor = FlatTensor::from_shape_and_data(new_shape, input_tensor.data.clone())?;

    if interp.consumption_mode == crate::interpreter::ConsumptionMode::Keep {
        interp.stack.push(data_val);
        interp.stack.push(shape_val);
    }

    interp.stack.push(result_tensor.to_value());
    Ok(())
}

pub(crate) fn build_nested_value(data: &[Fraction], shape: &[usize]) -> Value {
    if shape.is_empty() {
        // スカラー
        if data.len() == 1 {
            return Value {
                data: ValueData::Scalar(data[0].clone()),
            };
        }
        // データが複数ある場合はベクタ
        let children: Vec<Value> = data
            .iter()
            .map(|f| Value::from_fraction(f.clone()))
            .collect();
        return Value::from_children(children);
    }

    if shape.len() == 1 {
        // 1次元ベクタ
        let children: Vec<Value> = data
            .iter()
            .map(|f| Value::from_fraction(f.clone()))
            .collect();
        return Value {
            data: ValueData::Vector(Rc::new(children)),
        };
    }

    // 多次元: 最外層の次元でチャンクに分割し、再帰的に構築
    let outer_size = shape[0];
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

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if val.is_nil() {
        interp.stack.push(Value::nil());
        return Ok(());
    }

    let tensor = match FlatTensor::from_value(&val) {
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

    let rows = tensor.shape[0];
    let cols = tensor.shape[1];

    let mut transposed = Vec::with_capacity(tensor.data.len());
    for j in 0..cols {
        for i in 0..rows {
            transposed.push(tensor.data[i * cols + j].clone());
        }
    }

    let result_tensor = FlatTensor::from_shape_and_data(vec![cols, rows], transposed)?;

    if interp.consumption_mode == crate::interpreter::ConsumptionMode::Keep {
        interp.stack.push(val);
    }

    interp.stack.push(result_tensor.to_value());
    Ok(())
}

fn unary_math_op<F>(interp: &mut Interpreter, op: F, op_name: &str) -> Result<()>
where
    F: Fn(&Fraction) -> Fraction,
{
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: op_name.to_string(),
            mode: "Stack".into(),
        });
    }

    let is_keep_mode = interp.consumption_mode == crate::interpreter::ConsumptionMode::Keep;

    let val = if is_keep_mode {
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
            let result = op(f);
            interp.stack.push(wrap_number(result));
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
    unary_math_op(interp, |f| f.floor(), "FLOOR")
}

pub fn op_ceil(interp: &mut Interpreter) -> Result<()> {
    unary_math_op(interp, |f| f.ceil(), "CEIL")
}

pub fn op_round(interp: &mut Interpreter) -> Result<()> {
    unary_math_op(interp, |f| f.round(), "ROUND")
}

pub fn op_mod(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode == OperationTargetMode::Stack {
        return Err(AjisaiError::ModeUnsupported {
            word: "MOD".into(),
            mode: "Stack".into(),
        });
    }

    let is_keep_mode = interp.consumption_mode == crate::interpreter::ConsumptionMode::Keep;

    let b_val = if is_keep_mode {
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
        if y.numerator.is_zero() {
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
            _ => {
                interp.stack.push(args_val);
                return Err(AjisaiError::from(
                    "Shape dimensions must be positive integers",
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
