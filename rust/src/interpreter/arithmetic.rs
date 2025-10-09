// rust/src/interpreter/arithmetic.rs

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction, BracketType};
use num_traits::{Zero, One, ToPrimitive};

impl Interpreter {
    // スタックトップから数値の引数を取得する
    // 引数がなければデフォルト値(2)を返す
    fn get_optional_count(&mut self, default: usize) -> Result<usize> {
        if let Some(top) = self.stack.last() {
            if let ValueType::Vector(v, _) = &top.val_type {
                if v.len() == 1 {
                    if let ValueType::Number(n) = &v[0].val_type {
                        if n.denominator == One::one() {
                            let count = n.numerator.to_usize().ok_or_else(|| AjisaiError::from("Count too large"))?;
                            self.stack.pop(); // countを消費
                            return Ok(count);
                        }
                    }
                }
            }
        }
        Ok(default)
    }
}


fn binary_arithmetic_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> Fraction,
{
    let n = interp.get_optional_count(2)?;
    if interp.stack.len() < n { return Err(AjisaiError::StackUnderflow); }

    let mut vectors: Vec<Vec<Value>> = Vec::with_capacity(n);
    for _ in 0..n {
        let val = interp.stack.pop().unwrap();
        if let ValueType::Vector(v, _) = val.val_type {
            vectors.push(v);
        } else {
            return Err(AjisaiError::type_error("vector", "other type"));
        }
    }
    vectors.reverse(); // 評価順序をスタックの順序に合わせる

    if vectors.is_empty() { return Ok(()); }

    // 全てのベクトルの長さをチェック
    let first_len = vectors[0].len();
    if !vectors.iter().all(|v| v.len() == first_len) {
        return Err(AjisaiError::from("All vectors in arithmetic operation must have the same length"));
    }
    
    let mut result_vec = Vec::with_capacity(first_len);

    for i in 0..first_len {
        let initial_val = vectors[0][i].clone();
        let mut acc = if let ValueType::Number(f) = initial_val.val_type {
            f
        } else {
            return Err(AjisaiError::type_error("number", "other type"));
        };

        for j in 1..n {
            let next_val = &vectors[j][i];
            let next_frac = if let ValueType::Number(f) = &next_val.val_type {
                f
            } else {
                return Err(AjisaiError::type_error("number", "other type"));
            };
            acc = op(&acc, next_frac);
        }
        result_vec.push(Value { val_type: ValueType::Number(acc) });
    }

    interp.stack.push(Value { val_type: ValueType::Vector(result_vec, BracketType::Square) });
    Ok(())
}

pub fn op_add(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| a.add(b))
}

pub fn op_sub(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| a.sub(b))
}

pub fn op_mul(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| a.mul(b))
}

pub fn op_div(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| {
        if b.numerator.is_zero() {
            // このエラーハンドリングは理想的ではないが、シグネチャの制約上ここでpanicする
            panic!("Division by zero");
        }
        a.div(b)
    })
}

// 比較演算子は変更なし (2つのベクトルのみを比較)
// ... (op_lt, op_le, op_gt, op_ge, op_eq) ...

// 論理演算子は変更なし
// ... (op_not, op_and, op_or) ...

// (変更のない関数は省略)
fn extract_single_element_value(vector_val: &Value) -> Result<&Value> {
    match &vector_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => Ok(&v[0]),
        ValueType::Vector(_, _) => Err(AjisaiError::from("Multi-element vector not supported in this context")),
        _ => Err(AjisaiError::type_error("single-element vector", "other type")),
    }
}

fn wrap_result_value(value: Value) -> Value {
    Value {
        val_type: ValueType::Vector(vec![value], BracketType::Square)
    }
}

fn binary_comparison_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> bool,
{
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b_vec = interp.stack.pop().unwrap();
    let a_vec = interp.stack.pop().unwrap();
    
    let a_val = extract_single_element_value(&a_vec)?;
    let b_val = extract_single_element_value(&b_vec)?;
    
    let result = match (&a_val.val_type, &b_val.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Boolean(op(n1, n2)) }
        },
        _ => return Err(AjisaiError::type_error("number", "other type")),
    };
    
    interp.stack.push(wrap_result_value(result));
    Ok(())
}

pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.lt(b))
}

pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.le(b))
}

pub fn op_gt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.gt(b))
}

pub fn op_ge(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.ge(b))
}

pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b_vec = interp.stack.pop().unwrap();
    let a_vec = interp.stack.pop().unwrap();
    
    let result = Value { val_type: ValueType::Boolean(a_vec == b_vec) };
    interp.stack.push(wrap_result_value(result));
    Ok(())
}

pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    let val_vec = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let val = extract_single_element_value(&val_vec)?;
    
    let result = match &val.val_type {
        ValueType::Boolean(b) => Value { val_type: ValueType::Boolean(!b) },
        ValueType::Nil => Value { val_type: ValueType::Nil },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other type")),
    };
    
    interp.stack.push(wrap_result_value(result));
    Ok(())
}

pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 { return Err(AjisaiError::StackUnderflow); }
    let b_vec = interp.stack.pop().unwrap();
    let a_vec = interp.stack.pop().unwrap();
    let a_val = extract_single_element_value(&a_vec)?;
    let b_val = extract_single_element_value(&b_vec)?;
    
    let result = match (&a_val.val_type, &b_val.val_type) {
        (ValueType::Boolean(a), ValueType::Boolean(b)) => Value { val_type: ValueType::Boolean(*a && *b) },
        (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) => Value { val_type: ValueType::Boolean(false) },
        (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) | (ValueType::Nil, ValueType::Nil) => Value { val_type: ValueType::Nil },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other types")),
    };
    interp.stack.push(wrap_result_value(result));
    Ok(())
}

pub fn op_or(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 { return Err(AjisaiError::StackUnderflow); }
    let b_vec = interp.stack.pop().unwrap();
    let a_vec = interp.stack.pop().unwrap();
    let a_val = extract_single_element_value(&a_vec)?;
    let b_val = extract_single_element_value(&b_vec)?;

    let result = match (&a_val.val_type, &b_val.val_type) {
        (ValueType::Boolean(a), ValueType::Boolean(b)) => Value { val_type: ValueType::Boolean(*a || *b) },
        (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) => Value { val_type: ValueType::Boolean(true) },
        (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) | (ValueType::Nil, ValueType::Nil) => Value { val_type: ValueType::Nil },
        _ => return Err(AjisaiError::type_error("boolean or nil", "other types")),
    };
    interp.stack.push(wrap_result_value(result));
    Ok(())
}
