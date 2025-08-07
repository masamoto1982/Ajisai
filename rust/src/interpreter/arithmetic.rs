use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, Fraction};

// ヘルパー関数：値の型を文字列で取得
fn value_type_name(val_type: &ValueType) -> &'static str {
    match val_type {
        ValueType::Number(_) => "number",
        ValueType::String(_) => "string",
        ValueType::Boolean(_) => "boolean",
        ValueType::Symbol(_) => "symbol",
        ValueType::Vector(_) => "vector",
        ValueType::Nil => "nil",
    }
}

// ベクトルに単項関数を適用
fn apply_unary_to_vector<F>(vec: &[Value], f: F) -> Vec<Value>
where
    F: Fn(&Value) -> Value,
{
    vec.iter().map(f).collect()
}

// 2つのベクトルに二項演算を適用
fn apply_binary_to_vectors<F>(v1: &[Value], v2: &[Value], f: F) -> Result<Vec<Value>>
where
    F: Fn(&Value, &Value) -> Result<Value>,
{
    if v1.len() != v2.len() {
        return Err(AjisaiError::VectorLengthMismatch {
            len1: v1.len(),
            len2: v2.len(),
        });
    }
    
    v1.iter().zip(v2.iter())
        .map(|(a, b)| f(a, b))
        .collect::<Result<Vec<Value>>>()
}

// 共通の二項算術演算処理
fn binary_arithmetic_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> Fraction + Copy,
{
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b = interp.stack.pop().unwrap();
    let a = interp.stack.pop().unwrap();
    
    let result = match (&a.val_type, &b.val_type) {
        // 数値同士
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Number(op(n1, n2)) }
        },
        
        // ベクトルと数値（暗黙の反復）
        (ValueType::Vector(v), ValueType::Number(n)) => {
            let result = apply_unary_to_vector(v, |elem| {
                if let ValueType::Number(elem_n) = &elem.val_type {
                    Value { val_type: ValueType::Number(op(elem_n, n)) }
                } else {
                    elem.clone()
                }
            });
            Value { val_type: ValueType::Vector(result) }
        },
        
        // 数値とベクトル（暗黙の反復）
        (ValueType::Number(n), ValueType::Vector(v)) => {
            let result = apply_unary_to_vector(v, |elem| {
                if let ValueType::Number(elem_n) = &elem.val_type {
                    Value { val_type: ValueType::Number(op(n, elem_n)) }
                } else {
                    elem.clone()
                }
            });
            Value { val_type: ValueType::Vector(result) }
        },
        
        // ベクトル同士（要素ごとの演算）
        (ValueType::Vector(v1), ValueType::Vector(v2)) => {
            let result = apply_binary_to_vectors(v1, v2, |a, b| {
                match (&a.val_type, &b.val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        Ok(Value { val_type: ValueType::Number(op(n1, n2)) })
                    },
                    _ => Ok(a.clone())
                }
            })?;
            Value { val_type: ValueType::Vector(result) }
        },
        
        _ => return Err(AjisaiError::type_error(
            "number or vector",
            &format!("{} and {}", value_type_name(&a.val_type), value_type_name(&b.val_type))
        )),
    };
    
    interp.stack.push(result);
    Ok(())
}

// 共通の二項比較演算処理
fn binary_comparison_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> bool + Copy,
{
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b = interp.stack.pop().unwrap();
    let a = interp.stack.pop().unwrap();
    
    let result = match (&a.val_type, &b.val_type) {
        // 数値同士
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Boolean(op(n1, n2)) }
        },
        
        // ベクトルと数値（暗黙の反復）
        (ValueType::Vector(v), ValueType::Number(n)) => {
            let result = apply_unary_to_vector(v, |elem| {
                if let ValueType::Number(elem_n) = &elem.val_type {
                    Value { val_type: ValueType::Boolean(op(elem_n, n)) }
                } else {
                    Value { val_type: ValueType::Boolean(false) }
                }
            });
            Value { val_type: ValueType::Vector(result) }
        },
        
        // 数値とベクトル（暗黙の反復）
        (ValueType::Number(n), ValueType::Vector(v)) => {
            let result = apply_unary_to_vector(v, |elem| {
                if let ValueType::Number(elem_n) = &elem.val_type {
                    Value { val_type: ValueType::Boolean(op(n, elem_n)) }
                } else {
                    Value { val_type: ValueType::Boolean(false) }
                }
            });
            Value { val_type: ValueType::Vector(result) }
        },
        
        // ベクトル同士（要素ごとの演算）
        (ValueType::Vector(v1), ValueType::Vector(v2)) => {
            let result = apply_binary_to_vectors(v1, v2, |a, b| {
                match (&a.val_type, &b.val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        Ok(Value { val_type: ValueType::Boolean(op(n1, n2)) })
                    },
                    _ => Ok(Value { val_type: ValueType::Boolean(false) })
                }
            })?;
            Value { val_type: ValueType::Vector(result) }
        },
        
        _ => return Err(AjisaiError::type_error(
            "number or vector",
            &format!("{} and {}", value_type_name(&a.val_type), value_type_name(&b.val_type))
        )),
    };
    
    interp.stack.push(result);
    Ok(())
}

// 算術演算子の実装
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
    // 除算は特別処理（ゼロ除算チェック）
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b = interp.stack.pop().unwrap();
    let a = interp.stack.pop().unwrap();
    
    // ゼロ除算チェック
    match &b.val_type {
        ValueType::Number(n) if n.numerator == 0 => return Err(AjisaiError::DivisionByZero),
        ValueType::Vector(v) => {
            for elem in v {
                if let ValueType::Number(n) = &elem.val_type {
                    if n.numerator == 0 {
                        return Err(AjisaiError::DivisionByZero);
                    }
                }
            }
        },
        _ => {}
    }
    
    let result = match (&a.val_type, &b.val_type) {
        (ValueType::Number(n1), ValueType::Number(n2)) => {
            Value { val_type: ValueType::Number(n1.div(n2)) }
        },
        
        (ValueType::Vector(v), ValueType::Number(n)) => {
            let result = apply_unary_to_vector(v, |elem| {
                if let ValueType::Number(elem_n) = &elem.val_type {
                    Value { val_type: ValueType::Number(elem_n.div(n)) }
                } else {
                    elem.clone()
                }
            });
            Value { val_type: ValueType::Vector(result) }
        },
        
        (ValueType::Number(n), ValueType::Vector(v)) => {
            let result = apply_unary_to_vector(v, |elem| {
                if let ValueType::Number(elem_n) = &elem.val_type {
                    Value { val_type: ValueType::Number(n.div(elem_n)) }
                } else {
                    elem.clone()
                }
            });
            Value { val_type: ValueType::Vector(result) }
        },
        
        (ValueType::Vector(v1), ValueType::Vector(v2)) => {
            let result = apply_binary_to_vectors(v1, v2, |a, b| {
                match (&a.val_type, &b.val_type) {
                    (ValueType::Number(n1), ValueType::Number(n2)) => {
                        Ok(Value { val_type: ValueType::Number(n1.div(n2)) })
                    },
                    _ => Ok(a.clone())
                }
            })?;
            Value { val_type: ValueType::Vector(result) }
        },
        
        _ => return Err(AjisaiError::type_error(
            "number or vector",
            &format!("{} and {}", value_type_name(&a.val_type), value_type_name(&b.val_type))
        )),
    };
    
    interp.stack.push(result);
    Ok(())
}

// 比較演算子の実装
pub fn op_gt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.gt(b))
}

pub fn op_ge(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.ge(b))
}

pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.lt(b))
}

pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.le(b))
}

pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    // 等価比較は特別（型を超えた比較が可能）
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b = interp.stack.pop().unwrap();
    let a = interp.stack.pop().unwrap();
    
    interp.stack.push(Value { val_type: ValueType::Boolean(a == b) });
    Ok(())
}

// 論理演算（暗黙の反復対応・三値論理）
pub fn op_not(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    let result = match val.val_type {
        ValueType::Boolean(b) => Value { val_type: ValueType::Boolean(!b) },
        ValueType::Nil => Value { val_type: ValueType::Nil },
        ValueType::Vector(v) => {
            let result = apply_unary_to_vector(&v, |elem| {
                match &elem.val_type {
                    ValueType::Boolean(b) => Value { val_type: ValueType::Boolean(!b) },
                    ValueType::Nil => Value { val_type: ValueType::Nil },
                    _ => elem.clone(),
                }
            });
            Value { val_type: ValueType::Vector(result) }
        },
        _ => return Err(AjisaiError::type_error(
            "boolean, nil, or vector",
            value_type_name(&val.val_type)
        )),
    };
    
    interp.stack.push(result);
    Ok(())
}

pub fn op_and(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b_val = interp.stack.pop().unwrap();
    let a_val = interp.stack.pop().unwrap();
    
    let result = match (a_val.val_type, b_val.val_type) {
        (ValueType::Boolean(a), ValueType::Boolean(b)) => {
            Value { val_type: ValueType::Boolean(a && b) }
        },
        (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) => {
            Value { val_type: ValueType::Boolean(false) }
        },
        (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) | (ValueType::Nil, ValueType::Nil) => {
            Value { val_type: ValueType::Nil }
        },
        _ => return Err(AjisaiError::type_error(
            "boolean or nil",
            "other types"
        )),
    };
    
    interp.stack.push(result);
    Ok(())
}

pub fn op_or(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let b_val = interp.stack.pop().unwrap();
    let a_val = interp.stack.pop().unwrap();
    
    let result = match (a_val.val_type, b_val.val_type) {
        (ValueType::Boolean(a), ValueType::Boolean(b)) => {
            Value { val_type: ValueType::Boolean(a || b) }
        },
        (ValueType::Boolean(true), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(true)) => {
            Value { val_type: ValueType::Boolean(true) }
        },
        (ValueType::Boolean(false), ValueType::Nil) | (ValueType::Nil, ValueType::Boolean(false)) | (ValueType::Nil, ValueType::Nil) => {
            Value { val_type: ValueType::Nil }
        },
        _ => return Err(AjisaiError::type_error(
            "boolean or nil",
            "other types"
        )),
    };
    
    interp.stack.push(result);
    Ok(())
}

// Nil関連
pub fn op_nil_check(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    interp.stack.push(Value { 
        val_type: ValueType::Boolean(matches!(val.val_type, ValueType::Nil)) 
    });
    Ok(())
}

pub fn op_not_nil_check(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    interp.stack.push(Value { 
        val_type: ValueType::Boolean(!matches!(val.val_type, ValueType::Nil)) 
    });
    Ok(())
}

pub fn op_default(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }
    
    let default_val = interp.stack.pop().unwrap();
    let val = interp.stack.pop().unwrap();
    
    if matches!(val.val_type, ValueType::Nil) {
        interp.stack.push(default_val);
    } else {
        interp.stack.push(val);
    }
    Ok(())
}
