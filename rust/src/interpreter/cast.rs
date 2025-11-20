// rust/src/interpreter/cast.rs
//
// 【責務】
// 型変換ワード群を実装する。
// STR: 任意の型 → String
// NUM: String → Number
// BOOL: String → Boolean
// NIL: String → Nil
// VEC: String → Vector
//
// 【設計原則】
// - 同型変換はエラー（型安全性の維持）
// - 明示的な変換のみ（暗黙的型変換を排除）
// - 型推論を排除（パース失敗は即エラー）

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{wrap_in_square_vector, extract_single_element};
use crate::types::{Value, ValueType, BracketType};
use crate::types::fraction::Fraction;

/// STR - 任意の型を文字列に変換
///
/// 【責務】
/// - Number/Boolean/Nil/Symbol/Vector → String
/// - String → String はエラー（同型変換）
///
/// 【使用法】
/// ```ajisai
/// [ 42 ] STR → [ '42' ]
/// [ TRUE ] STR → [ 'TRUE' ]
/// [ nil ] STR → [ 'nil' ]
/// [ [ 1 2 3 ] ] STR → [ '1 2 3' ]
/// ```
///
/// 【エラー】
/// - String → String（同型変換）
pub fn op_str(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("STR only supports StackTop mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let inner_val = extract_single_element(&val)?;

    match &inner_val.val_type {
        ValueType::String(_) => {
            interp.stack.push(val);
            Err(AjisaiError::from("STR: same-type conversion (String → String) is not allowed"))
        }
        _ => {
            let string_repr = value_to_string_repr(inner_val);
            interp.stack.push(wrap_in_square_vector(
                Value { val_type: ValueType::String(string_repr) }
            ));
            Ok(())
        }
    }
}

/// NUM - 文字列を数値に変換
///
/// 【責務】
/// - String → Number（パース失敗でエラー）
/// - 他の型はエラー
///
/// 【使用法】
/// ```ajisai
/// [ '42' ] NUM → [ 42 ]
/// [ '1/3' ] NUM → [ 1/3 ]
/// [ '3.14' ] NUM → [ 157/50 ]
/// [ 'hello' ] NUM → ERROR
/// ```
///
/// 【エラー】
/// - String以外の型
/// - 数値としてパース不可能な文字列
pub fn op_num(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("NUM only supports StackTop mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let inner_val = extract_single_element(&val)?;

    match &inner_val.val_type {
        ValueType::String(s) => {
            let fraction = Fraction::from_str(s)
                .map_err(|_| AjisaiError::from(format!("NUM: cannot parse '{}' as a number", s)))?;
            interp.stack.push(wrap_in_square_vector(
                Value { val_type: ValueType::Number(fraction) }
            ));
            Ok(())
        }
        _ => {
            interp.stack.push(val);
            Err(AjisaiError::from("NUM: requires String type"))
        }
    }
}

/// BOOL - 文字列を真偽値に変換
///
/// 【責務】
/// - String → Boolean（"TRUE" または "FALSE" のみ、大小文字無視）
/// - 他の型はエラー
///
/// 【使用法】
/// ```ajisai
/// [ 'TRUE' ] BOOL → [ TRUE ]
/// [ 'false' ] BOOL → [ FALSE ]
/// [ 'hello' ] BOOL → ERROR
/// ```
///
/// 【エラー】
/// - String以外の型
/// - "TRUE"/"FALSE"以外の文字列
pub fn op_bool(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("BOOL only supports StackTop mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let inner_val = extract_single_element(&val)?;

    match &inner_val.val_type {
        ValueType::String(s) => {
            let upper = s.to_uppercase();
            let bool_val = if upper == "TRUE" {
                true
            } else if upper == "FALSE" {
                false
            } else {
                return Err(AjisaiError::from(format!(
                    "BOOL: cannot parse '{}' as boolean (expected 'TRUE' or 'FALSE')", s
                )));
            };
            interp.stack.push(wrap_in_square_vector(
                Value { val_type: ValueType::Boolean(bool_val) }
            ));
            Ok(())
        }
        _ => {
            interp.stack.push(val);
            Err(AjisaiError::from("BOOL: requires String type"))
        }
    }
}

/// NIL - 文字列をNilに変換
///
/// 【責務】
/// - String → Nil（"nil" のみ、大小文字無視）
/// - 他の型はエラー
///
/// 【使用法】
/// ```ajisai
/// [ 'nil' ] NIL → [ nil ]
/// [ 'NIL' ] NIL → [ nil ]
/// [ 'hello' ] NIL → ERROR
/// ```
///
/// 【エラー】
/// - String以外の型
/// - "nil"以外の文字列
pub fn op_nil(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("NIL only supports StackTop mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let inner_val = extract_single_element(&val)?;

    match &inner_val.val_type {
        ValueType::String(s) => {
            let upper = s.to_uppercase();
            if upper == "NIL" {
                interp.stack.push(wrap_in_square_vector(
                    Value { val_type: ValueType::Nil }
                ));
                Ok(())
            } else {
                Err(AjisaiError::from(format!(
                    "NIL: cannot parse '{}' as nil (expected 'nil')", s
                )))
            }
        }
        _ => {
            interp.stack.push(val);
            Err(AjisaiError::from("NIL: requires String type"))
        }
    }
}

/// VEC - 文字列をベクトルに変換
///
/// 【責務】
/// - String → Vector（スペースで分割、要素は全て文字列）
/// - 他の型はエラー
///
/// 【使用法】
/// ```ajisai
/// [ '1 2 3' ] VEC → [ '1' '2' '3' ]
/// [ 'hello world' ] VEC → [ 'hello' 'world' ]
/// [ '' ] VEC → [ ]（空ベクトル）
/// ```
///
/// 【エラー】
/// - String以外の型
pub fn op_vec(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("VEC only supports StackTop mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let inner_val = extract_single_element(&val)?;

    match &inner_val.val_type {
        ValueType::String(s) => {
            let parts: Vec<&str> = s.split_whitespace().collect();
            let elements: Vec<Value> = parts.iter()
                .map(|part| Value { val_type: ValueType::String(part.to_string()) })
                .collect();

            interp.stack.push(Value {
                val_type: ValueType::Vector(elements, BracketType::Square)
            });
            Ok(())
        }
        _ => {
            interp.stack.push(val);
            Err(AjisaiError::from("VEC: requires String type"))
        }
    }
}

/// 値を文字列表現に変換する（内部ヘルパー）
///
/// 【責務】
/// - 各型を人間が読める文字列表現に変換
/// - ベクトルは要素をスペース区切りで結合
/// - ネストされたベクトルは括弧なしで平坦化
///
/// 【引数】
/// - value: 変換する値
///
/// 【戻り値】
/// - 文字列表現
fn value_to_string_repr(value: &Value) -> String {
    match &value.val_type {
        ValueType::Number(n) => {
            if n.denominator == num_bigint::BigInt::from(1) {
                format!("{}", n.numerator)
            } else {
                format!("{}/{}", n.numerator, n.denominator)
            }
        }
        ValueType::String(s) => s.clone(),
        ValueType::Boolean(b) => {
            if *b { "TRUE".to_string() } else { "FALSE".to_string() }
        }
        ValueType::Symbol(s) => s.clone(),
        ValueType::Nil => "nil".to_string(),
        ValueType::SingletonVector(boxed_val, _) => {
            value_to_string_repr(boxed_val)
        }
        ValueType::Vector(vec, _) => {
            vec.iter()
                .map(|v| value_to_string_repr(v))
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;
    use num_traits::One;

    #[test]
    fn test_value_to_string_repr() {
        // Number
        let num = Value {
            val_type: ValueType::Number(Fraction::new(BigInt::from(42), BigInt::one()))
        };
        assert_eq!(value_to_string_repr(&num), "42");

        // Boolean
        let bool_val = Value { val_type: ValueType::Boolean(true) };
        assert_eq!(value_to_string_repr(&bool_val), "TRUE");

        // Nil
        let nil = Value { val_type: ValueType::Nil };
        assert_eq!(value_to_string_repr(&nil), "nil");

        // Vector
        let vec = Value {
            val_type: ValueType::Vector(
                vec![
                    Value { val_type: ValueType::Number(Fraction::new(BigInt::from(1), BigInt::one())) },
                    Value { val_type: ValueType::Number(Fraction::new(BigInt::from(2), BigInt::one())) },
                    Value { val_type: ValueType::Number(Fraction::new(BigInt::from(3), BigInt::one())) },
                ],
                BracketType::Square
            )
        };
        assert_eq!(value_to_string_repr(&vec), "1 2 3");
    }

    #[test]
    fn test_str_conversion() {
        let mut interp = Interpreter::new();

        // Number → String
        interp.stack.push(wrap_in_square_vector(
            Value { val_type: ValueType::Number(Fraction::new(BigInt::from(42), BigInt::one())) }
        ));
        op_str(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v, _) = &val.val_type {
                if let ValueType::String(s) = &v[0].val_type {
                    assert_eq!(s, "42");
                }
            }
        }
    }

    #[test]
    fn test_num_conversion() {
        let mut interp = Interpreter::new();

        // String → Number
        interp.stack.push(wrap_in_square_vector(
            Value { val_type: ValueType::String("42".to_string()) }
        ));
        op_num(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v, _) = &val.val_type {
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator, BigInt::from(42));
                }
            }
        }
    }

    #[test]
    fn test_bool_conversion() {
        let mut interp = Interpreter::new();

        // String → Boolean
        interp.stack.push(wrap_in_square_vector(
            Value { val_type: ValueType::String("TRUE".to_string()) }
        ));
        op_bool(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v, _) = &val.val_type {
                if let ValueType::Boolean(b) = &v[0].val_type {
                    assert_eq!(*b, true);
                }
            }
        }
    }

    #[test]
    fn test_vec_conversion() {
        let mut interp = Interpreter::new();

        // String → Vector
        interp.stack.push(wrap_in_square_vector(
            Value { val_type: ValueType::String("hello world".to_string()) }
        ));
        op_vec(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v, _) = &val.val_type {
                assert_eq!(v.len(), 2);
                if let ValueType::String(s1) = &v[0].val_type {
                    assert_eq!(s1, "hello");
                }
                if let ValueType::String(s2) = &v[1].val_type {
                    assert_eq!(s2, "world");
                }
            }
        }
    }
}
