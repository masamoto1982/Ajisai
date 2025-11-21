// rust/src/interpreter/cast.rs
//
// 【責務】
// 型変換ワード群を実装する。
// STR: 任意の型 → String
// NUM: String/Boolean → Number
// BOOL: String → Boolean
// NIL: String → Nil
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

/// NUM - 文字列または真偽値を数値に変換
///
/// 【責務】
/// - String → Number（パース失敗でエラー）
/// - Boolean → Number（TRUE → 1、FALSE → 0）
/// - Number → エラー（同型変換）
/// - Nil → エラー
/// - 他の型はエラー
///
/// 【使用法】
/// ```ajisai
/// [ '42' ] NUM → [ 42 ]
/// [ '1/3' ] NUM → [ 1/3 ]
/// [ '3.14' ] NUM → [ 157/50 ]
/// [ TRUE ] NUM → [ 1 ]
/// [ FALSE ] NUM → [ 0 ]
/// [ 'hello' ] NUM → ERROR
/// [ 42 ] NUM → ERROR（同型変換）
/// [ nil ] NUM → ERROR
/// ```
///
/// 【エラー】
/// - 数値としてパース不可能な文字列
/// - Number型（同型変換）
/// - Nil型
/// - その他の型
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
        ValueType::Boolean(b) => {
            use num_bigint::BigInt;
            use num_traits::One;
            let num = if *b { BigInt::one() } else { BigInt::from(0) };
            interp.stack.push(wrap_in_square_vector(
                Value { val_type: ValueType::Number(Fraction::new(num, BigInt::one())) }
            ));
            Ok(())
        }
        ValueType::Number(_) => {
            interp.stack.push(val);
            Err(AjisaiError::from("NUM: same-type conversion (Number → Number) is not allowed"))
        }
        ValueType::Nil => {
            interp.stack.push(val);
            Err(AjisaiError::from("NUM: cannot convert Nil to Number"))
        }
        _ => {
            interp.stack.push(val);
            Err(AjisaiError::from("NUM: requires String or Boolean type"))
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

        // Boolean → Number (TRUE → 1)
        interp.stack.clear();
        interp.stack.push(wrap_in_square_vector(
            Value { val_type: ValueType::Boolean(true) }
        ));
        op_num(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v, _) = &val.val_type {
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator, BigInt::from(1));
                    assert_eq!(n.denominator, BigInt::from(1));
                }
            }
        }

        // Boolean → Number (FALSE → 0)
        interp.stack.clear();
        interp.stack.push(wrap_in_square_vector(
            Value { val_type: ValueType::Boolean(false) }
        ));
        op_num(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v, _) = &val.val_type {
                if let ValueType::Number(n) = &v[0].val_type {
                    assert_eq!(n.numerator, BigInt::from(0));
                    assert_eq!(n.denominator, BigInt::from(1));
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

}
