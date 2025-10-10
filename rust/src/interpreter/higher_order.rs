// rust/src/interpreter/higher_order.rs

use crate::interpreter::{Interpreter, OperationTarget, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, BracketType};
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};

// === ヘルパー関数 ===

fn get_word_name_from_value(value: &Value) -> Result<String> {
    match &value.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            if let ValueType::String(s) = &v[0].val_type {
                Ok(s.to_uppercase())
            } else {
                Err(AjisaiError::type_error("string for word name", "other type"))
            }
        },
        _ => Err(AjisaiError::type_error("single-element vector with string", "other type")),
    }
}

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

// === 高階関数の実装 ===

pub fn op_map(interp: &mut Interpreter) -> Result<()> {
    let word_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let word_name = get_word_name_from_value(&word_val)?;

    if !interp.dictionary.contains_key(&word_name) {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(elements, bracket_type) = target_val.val_type {
                let mut results = Vec::new();
                for elem in elements {
                    // 各要素をスタックにプッシュ（Vector<T>ではなくTそのもの）
                    interp.stack.push(elem);
                    interp.execute_word_sync(&word_name)?;
                    results.push(interp.stack.pop().ok_or_else(|| AjisaiError::from("MAP word must return a value"))?);
                }
                interp.stack.push(Value { val_type: ValueType::Vector(results, bracket_type) });
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        },
        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if interp.stack.len() < count {
                return Err(AjisaiError::StackUnderflow);
            }

            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let mut results = Vec::new();

            for item in targets {
                interp.stack.push(item);
                interp.execute_word_sync(&word_name)?;
                results.push(interp.stack.pop().ok_or_else(|| AjisaiError::from("MAP word must return a value"))?);
            }
            interp.stack.extend(results);
        }
    }
    Ok(())
}

pub fn op_filter(interp: &mut Interpreter) -> Result<()> {
    let word_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let word_name = get_word_name_from_value(&word_val)?;
    
    if !interp.dictionary.contains_key(&word_name) {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    match interp.operation_target {
        OperationTarget::StackTop => {
            let target_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            if let ValueType::Vector(elements, bracket_type) = target_val.val_type {
                let mut results = Vec::new();
                for elem in elements {
                    interp.stack.push(elem.clone()); // フィルター条件判定後も元の要素が必要なためclone
                    interp.execute_word_sync(&word_name)?;
                    let condition_result = interp.stack.pop().ok_or_else(|| AjisaiError::from("FILTER word must return a boolean value"))?;
                    
                    if let ValueType::Vector(v, _) = condition_result.val_type {
                        if v.len() == 1 {
                            if let ValueType::Boolean(b) = v[0].val_type {
                                if b {
                                    results.push(elem);
                                }
                            }
                        }
                    }
                }
                interp.stack.push(Value { val_type: ValueType::Vector(results, bracket_type) });
            } else {
                return Err(AjisaiError::type_error("vector", "other type"));
            }
        },
        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            if interp.stack.len() < count {
                return Err(AjisaiError::StackUnderflow);
            }
            
            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let mut results = Vec::new();

            for item in targets {
                interp.stack.push(item.clone());
                interp.execute_word_sync(&word_name)?;
                let condition_result = interp.stack.pop().ok_or_else(|| AjisaiError::from("FILTER word must return a boolean value"))?;

                if let ValueType::Vector(v, _) = condition_result.val_type {
                    if v.len() == 1 {
                        if let ValueType::Boolean(b) = v[0].val_type {
                            if b {
                                results.push(item);
                            }
                        }
                    }
                }
            }
            interp.stack.extend(results);
        }
    }
    Ok(())
}
