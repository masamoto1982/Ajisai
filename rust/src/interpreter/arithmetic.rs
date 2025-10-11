// rust/src/interpreter/arithmetic.rs

use crate::interpreter::{Interpreter, OperationTarget, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, BracketType};
// `Fraction` のインポートパスを修正
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
// `BigInt::one()` を使用するために `One` トレイトをスコープに入れる
use num_traits::{One, ToPrimitive, Zero};

// === ヘルパー関数 ===

fn get_integer_from_value(value: &Value) -> Result<i64> {
    match &value.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            if let ValueType::Number(n) = &v[0].val_type {
                if n.denominator == BigInt::one() {
                    n.numerator.to_i64().ok_or_else(|| AjisaiError::from("Count is too large"))
                } else {
                    Err(AjisaiError::type_error("integer", "fraction"))
                }
            } else {
                Err(AjisaiError::type_error("integer", "other type"))
            }
        },
        _ => Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    }
}

// 数値を取り出すための内部ヘルパー
fn extract_number(val: &Value) -> Result<&Fraction> {
    match &val.val_type {
        ValueType::Number(n) => Ok(n),
        ValueType::Vector(v, _) if v.len() == 1 => {
            if let ValueType::Number(n) = &v[0].val_type {
                Ok(n)
            } else {
                Err(AjisaiError::type_error("number", "other type in inner vector"))
            }
        },
        _ => Err(AjisaiError::type_error("number or single-element number vector", "other type")),
    }
}


// === 新しい演算ロジック ===

fn binary_arithmetic_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction>,
{
    match interp.operation_target {
        // STACKTOPモード: ベクトル間の要素ごと演算
        OperationTarget::StackTop => {
            let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            let (a_vec, a_bracket) = match a_val.val_type {
                ValueType::Vector(v, b) => (v, b),
                _ => return Err(AjisaiError::type_error("vector", "other type")),
            };
            let (b_vec, _) = match b_val.val_type {
                ValueType::Vector(v, b) => (v, b),
                _ => return Err(AjisaiError::type_error("vector", "other type")),
            };

            let mut result_vec = Vec::new();
            
            // ブロードキャスト判定
            if a_vec.len() > 1 && b_vec.len() == 1 {
                let scalar = &b_vec[0];
                for elem in &a_vec {
                    let res_num = op(extract_number(elem)?, extract_number(scalar)?)?;
                    result_vec.push(Value { val_type: ValueType::Number(res_num) });
                }
            } else if a_vec.len() == 1 && b_vec.len() > 1 {
                let scalar = &a_vec[0];
                for elem in &b_vec {
                    let res_num = op(extract_number(scalar)?, extract_number(elem)?)?;
                    result_vec.push(Value { val_type: ValueType::Number(res_num) });
                }
            } else {
                if a_vec.len() != b_vec.len() {
                    return Err(AjisaiError::VectorLengthMismatch{ len1: a_vec.len(), len2: b_vec.len() });
                }
                for (a, b) in a_vec.iter().zip(b_vec.iter()) {
                    let res_num = op(extract_number(a)?, extract_number(b)?)?;
                    result_vec.push(Value { val_type: ValueType::Number(res_num) });
                }
            }

            interp.stack.push(Value { val_type: ValueType::Vector(result_vec, a_bracket) });
        },

        // STACKモード: N個の要素を畳み込む
        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if interp.stack.len() < count {
                return Err(AjisaiError::StackUnderflow);
            }
            if count == 0 {
                return Ok(()); // 何もしない
            }

            let items: Vec<Value> = interp.stack.drain(interp.stack.len() - count ..).collect();
            
            let mut acc_num = extract_number(&items[0])?.clone();

            for item in items.iter().skip(1) {
                acc_num = op(&acc_num, extract_number(item)?)?;
            }
            
            let result_val = Value { val_type: ValueType::Number(acc_num) };
            interp.stack.push(Value { val_type: ValueType::Vector(vec![result_val], BracketType::Square) });
        }
    }
    Ok(())
}


pub fn op_add(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| Ok(a.add(b)))
}

pub fn op_sub(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| Ok(a.sub(b)))
}

pub fn op_mul(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| Ok(a.mul(b)))
}

pub fn op_div(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| {
        if b.numerator.is_zero() {
            Err(AjisaiError::DivisionByZero)
        } else {
            Ok(a.div(b))
        }
    })
}
