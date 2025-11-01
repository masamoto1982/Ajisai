// rust/src/interpreter/bloom.rs
// BLOOM - 蕾が花開くように、保護されたデータがコードとして実行される

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{Value, ValueType, GuardClause, GuardBranch};

/// BLOOM組み込みワード：Vectorから値を解放して実行
pub fn op_bloom(interp: &mut Interpreter) -> Result<()> {
    let vec_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    
    match vec_val.val_type {
        ValueType::Vector(values, _) => {
            // ガード節を含むかチェック
            if contains_guard_separator(&values) {
                // ガード節として実行
                let guard = parse_guard_clause_from_values(&values)?;
                interp.execute_guard_clause(&guard)?;
            } else {
                // 通常のコードとして各要素を順に処理
                for value in values {
                    process_value(interp, value)?;
                }
            }
            Ok(())
        }
        _ => {
            // Vector以外はそのまま戻す
            interp.stack.push(vec_val);
            Ok(())
        }
    }
}

/// 値を処理する（BLOOMの核心）
pub fn process_value(interp: &mut Interpreter, value: Value) -> Result<()> {
    match value.val_type {
        // シンボルは「ワードとして実行」
        ValueType::Symbol(name) => {
            interp.execute_word_sync(&name.to_uppercase())?;
        }
        
        // ネストしたVectorは「データとしてスタックに積む」
        // （まだ保護膜の中）
        ValueType::Vector(_, _) => {
            interp.stack.push(value);
        }
        
        // その他のリテラルも「データとしてスタックに積む」
        ValueType::Number(_) | ValueType::String(_) | 
        ValueType::Boolean(_) | ValueType::Nil => {
            interp.stack.push(value);
        }
        
        // GuardSeparatorとLineBreakは無視
        // （これらはガード節のパース時に使用される）
        ValueType::GuardSeparator | ValueType::LineBreak => {}
    }
    Ok(())
}

/// Valueの列にガード区切りが含まれているかチェック
fn contains_guard_separator(values: &[Value]) -> bool {
    values.iter().any(|v| matches!(v.val_type, ValueType::GuardSeparator))
}

/// Valueの列からガード節をパース
fn parse_guard_clause_from_values(values: &[Value]) -> Result<GuardClause> {
    let mut branches = Vec::new();
    let mut current_section = Vec::new();
    let mut in_condition = true;
    let mut temp_condition: Option<Vec<Value>> = None;
    
    for value in values {
        match &value.val_type {
            ValueType::GuardSeparator => {
                if in_condition {
                    // 条件部分終了
                    if current_section.is_empty() {
                        return Err(AjisaiError::from("Empty condition in guard clause"));
                    }
                    temp_condition = Some(current_section.clone());
                    current_section.clear();
                    in_condition = false;
                } else {
                    // アクション部分終了
                    if let Some(condition) = temp_condition.take() {
                        branches.push(GuardBranch {
                            condition,
                            action: current_section.clone(),
                        });
                        current_section.clear();
                        in_condition = true;
                    } else {
                        return Err(AjisaiError::from("Guard clause syntax error: action without condition"));
                    }
                }
            }
            ValueType::LineBreak => {
                // 改行は無視（単一行ガード節のサポート）
            }
            _ => {
                current_section.push(value.clone());
            }
        }
    }
    
    // 最後のセクションがデフォルト行
    if current_section.is_empty() {
        return Err(AjisaiError::from("Guard clause must have a default branch"));
    }
    
    // temp_conditionに値が残っている場合はエラー
    // （条件があるのにアクションがない）
    if temp_condition.is_some() {
        return Err(AjisaiError::from("Guard clause: condition without action"));
    }
    
    Ok(GuardClause {
        branches,
        default: current_section,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::fraction::Fraction;
    use num_bigint::BigInt;
    use num_traits::One;

    #[test]
    fn test_contains_guard_separator() {
        let values = vec![
            Value { val_type: ValueType::Number(Fraction::new(BigInt::one(), BigInt::one())) },
            Value { val_type: ValueType::GuardSeparator },
        ];
        assert!(contains_guard_separator(&values));
        
        let values_no_guard = vec![
            Value { val_type: ValueType::Number(Fraction::new(BigInt::one(), BigInt::one())) },
        ];
        assert!(!contains_guard_separator(&values_no_guard));
    }

    #[test]
    fn test_parse_simple_guard() {
        // [ TRUE ] : [ 1 ] : [ 2 ]
        let values = vec![
            Value { val_type: ValueType::Boolean(true) },
            Value { val_type: ValueType::GuardSeparator },
            Value { val_type: ValueType::Number(Fraction::new(BigInt::one(), BigInt::one())) },
            Value { val_type: ValueType::GuardSeparator },
            Value { val_type: ValueType::Number(Fraction::new(BigInt::from(2), BigInt::one())) },
        ];
        
        let guard = parse_guard_clause_from_values(&values).unwrap();
        assert_eq!(guard.branches.len(), 1);
        assert_eq!(guard.default.len(), 1);
    }

    #[test]
    fn test_parse_guard_no_default_error() {
        // [ TRUE ] : [ 1 ] :
        let values = vec![
            Value { val_type: ValueType::Boolean(true) },
            Value { val_type: ValueType::GuardSeparator },
            Value { val_type: ValueType::Number(Fraction::new(BigInt::one(), BigInt::one())) },
            Value { val_type: ValueType::GuardSeparator },
        ];
        
        let result = parse_guard_clause_from_values(&values);
        assert!(result.is_err());
    }
}
