// rust/src/interpreter/cast.rs
//
// 統一分数アーキテクチャ版の型変換ワード群
//
// 【設計原則】
// すべての値は Vec<Fraction> として表現される。
// DisplayHint は表示目的のみに使用し、演算には使用しない。
// 「型変換」は実質的に DisplayHint の変更と、表示形式の変換である。

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{wrap_value, wrap_number};
use crate::types::{Value, DisplayHint};
use crate::types::fraction::Fraction;

/// 値を文字列として解釈する（内部ヘルパー）
fn value_as_string(val: &Value) -> Option<String> {
    if val.data.is_empty() {
        return None;
    }

    Some(val.data.iter()
        .filter_map(|f| {
            f.to_i64().and_then(|n| {
                if n >= 0 && n <= 0x10FFFF {
                    char::from_u32(n as u32)
                } else {
                    None
                }
            })
        })
        .collect())
}

/// 値が文字列として扱えるかチェック
fn is_string_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::String
}

/// 値が真偽値として扱えるかチェック
fn is_boolean_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::Boolean && val.data.len() == 1
}

/// 値が数値として扱えるかチェック
fn is_number_value(val: &Value) -> bool {
    matches!(val.display_hint, DisplayHint::Number | DisplayHint::Auto) && val.data.len() == 1
}

/// 値がDateTimeとして扱えるかチェック
fn is_datetime_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::DateTime && val.data.len() == 1
}

/// STR - 任意の型を文字列に変換
///
/// 【使用法】
/// ```ajisai
/// [ 42 ] STR → [ '42' ]
/// [ TRUE ] STR → [ 'TRUE' ]
/// [ NIL ] STR → [ 'NIL' ]
/// ```
///
/// 【エラー】
/// - String → String（同型変換）
pub fn op_str(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("STR does not support Stack (..) mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // NILの場合
    if val.is_nil() {
        interp.stack.push(wrap_value(Value::from_string("NIL")));
        return Ok(());
    }

    // 既に文字列形式の場合は冗長な変換エラー
    if is_string_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::from("STR: value is already in string format"));
    }

    // 真偽値の場合
    if is_boolean_value(&val) {
        let string_repr = if !val.data[0].is_zero() { "TRUE" } else { "FALSE" };
        interp.stack.push(wrap_value(Value::from_string(string_repr)));
        return Ok(());
    }

    // DateTimeの場合
    if is_datetime_value(&val) {
        let string_repr = fraction_to_string(&val.data[0]);
        interp.stack.push(wrap_value(Value::from_string(&string_repr)));
        return Ok(());
    }

    // 数値の場合
    if is_number_value(&val) {
        let string_repr = fraction_to_string(&val.data[0]);
        interp.stack.push(wrap_value(Value::from_string(&string_repr)));
        return Ok(());
    }

    // ベクタの場合（複数要素）
    let string_repr = value_to_string_repr(&val);
    interp.stack.push(wrap_value(Value::from_string(&string_repr)));
    Ok(())
}

/// 分数を文字列に変換するヘルパー
fn fraction_to_string(f: &Fraction) -> String {
    use num_bigint::BigInt;
    use num_traits::One;
    if f.denominator == BigInt::one() {
        format!("{}", f.numerator)
    } else {
        format!("{}/{}", f.numerator, f.denominator)
    }
}

/// NUM - 文字列または真偽値を数値に変換
///
/// 【使用法】
/// ```ajisai
/// [ '42' ] NUM → [ 42 ]
/// [ '1/3' ] NUM → [ 1/3 ]
/// [ TRUE ] NUM → [ 1 ]
/// [ FALSE ] NUM → [ 0 ]
/// ```
///
/// 【エラー】
/// - 数値としてパース不可能な文字列
/// - Number型（同型変換）
/// - Nil型
pub fn op_num(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("NUM does not support Stack (..) mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // NILの場合
    if val.is_nil() {
        interp.stack.push(val);
        return Err(AjisaiError::from("NUM: cannot convert Nil to Number"));
    }

    // 文字列の場合
    if is_string_value(&val) {
        let s = value_as_string(&val).unwrap_or_default();
        match Fraction::from_str(&s) {
            Ok(fraction) => {
                interp.stack.push(wrap_number(fraction));
                return Ok(());
            }
            Err(_) => {
                let err_msg = format!("NUM: cannot parse '{}' as a number", s);
                interp.stack.push(val);
                return Err(AjisaiError::from(err_msg));
            }
        }
    }

    // 真偽値の場合
    if is_boolean_value(&val) {
        use num_bigint::BigInt;
        use num_traits::One;
        let num = if !val.data[0].is_zero() { BigInt::one() } else { BigInt::from(0) };
        interp.stack.push(wrap_number(Fraction::new(num, BigInt::one())));
        return Ok(());
    }

    // 既に数値形式の場合は冗長な変換エラー
    if is_number_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::from("NUM: value is already in number format"));
    }

    // その他はエラー
    interp.stack.push(val);
    Err(AjisaiError::from("NUM: requires string or boolean format"))
}

/// BOOL - 文字列または数値を真偽値に変換
///
/// 【使用法】
/// ```ajisai
/// [ 'TRUE' ] BOOL → [ TRUE ]
/// [ '1' ] BOOL → [ TRUE ]
/// [ 1 ] BOOL → [ TRUE ]
/// [ 0 ] BOOL → [ FALSE ]
/// ```
///
/// 【エラー】
/// - 真偽値として認識できない文字列
/// - 1または0以外の数値
/// - Boolean型（同型変換）
/// - Nil型
pub fn op_bool(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("BOOL does not support Stack (..) mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // NILの場合
    if val.is_nil() {
        interp.stack.push(val);
        return Err(AjisaiError::from("BOOL: cannot convert Nil to Boolean"));
    }

    // 文字列の場合
    if is_string_value(&val) {
        let s = value_as_string(&val).unwrap_or_default();
        return convert_string_to_bool(&s, &val, interp);
    }

    // 数値の場合
    if is_number_value(&val) {
        return convert_fraction_to_bool(&val.data[0], &val, interp);
    }

    // 既に真偽値形式の場合は冗長な変換エラー
    if is_boolean_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::from("BOOL: value is already in boolean format"));
    }

    // その他はエラー
    interp.stack.push(val);
    Err(AjisaiError::from("BOOL: requires string or number format"))
}

/// 文字列をBoolに変換するヘルパー
fn convert_string_to_bool(s: &str, original_val: &Value, interp: &mut Interpreter) -> Result<()> {
    let upper = s.to_uppercase();
    let bool_val = if upper == "TRUE" || upper == "1" || s == "真" {
        true
    } else if upper == "FALSE" || upper == "0" || s == "偽" {
        false
    } else {
        interp.stack.push(original_val.clone());
        return Err(AjisaiError::from(format!(
            "BOOL: cannot parse '{}' as boolean (expected 'TRUE'/'FALSE', '1'/'0', '真'/'偽')", s
        )));
    };
    interp.stack.push(wrap_value(Value::from_bool(bool_val)));
    Ok(())
}

/// 分数をBoolに変換するヘルパー
fn convert_fraction_to_bool(n: &Fraction, original_val: &Value, interp: &mut Interpreter) -> Result<()> {
    use num_bigint::BigInt;
    use num_traits::One;

    let one = Fraction::new(BigInt::one(), BigInt::one());
    let zero = Fraction::new(BigInt::from(0), BigInt::one());

    if *n == one {
        interp.stack.push(wrap_value(Value::from_bool(true)));
        Ok(())
    } else if *n == zero {
        interp.stack.push(wrap_value(Value::from_bool(false)));
        Ok(())
    } else {
        interp.stack.push(original_val.clone());
        Err(AjisaiError::from(format!(
            "BOOL: cannot convert number {} to boolean (only 1 and 0 are allowed)",
            fraction_to_string(n)
        )))
    }
}

/// NIL - 文字列をNilに変換
///
/// 【使用法】
/// ```ajisai
/// [ 'nil' ] NIL → [ NIL ]
/// [ 'NIL' ] NIL → [ NIL ]
/// ```
///
/// 【エラー】
/// - "nil"以外の文字列
/// - Boolean型
/// - Number型
/// - Nil型（同型変換）
pub fn op_nil(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("NIL does not support Stack (..) mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 既にNIL形式の場合は冗長な変換エラー
    if val.is_nil() {
        interp.stack.push(val);
        return Err(AjisaiError::from("NIL: value is already nil"));
    }

    // 文字列の場合
    if is_string_value(&val) {
        let s = value_as_string(&val).unwrap_or_default();
        let upper = s.to_uppercase();
        if upper == "NIL" {
            interp.stack.push(wrap_value(Value::nil()));
            return Ok(());
        } else {
            let err_msg = format!("NIL: cannot parse '{}' as nil (expected 'nil')", s);
            interp.stack.push(val);
            return Err(AjisaiError::from(err_msg));
        }
    }

    // 真偽値の場合
    if is_boolean_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::from("NIL: cannot convert boolean format to nil"));
    }

    // 数値の場合
    if is_number_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::from("NIL: cannot convert number format to nil"));
    }

    // その他はエラー
    interp.stack.push(val);
    Err(AjisaiError::from("NIL: requires string format"))
}

/// CHARS - 文字列を文字ベクタに分解
///
/// 【使用法】
/// ```ajisai
/// [ 'hello' ] CHARS → [ { 'h' 'e' 'l' 'l' 'o' } ]
/// ```
///
/// 【エラー】
/// - 空文字列
/// - String以外の型
pub fn op_chars(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            // NILの場合
            if val.is_nil() {
                interp.stack.push(val);
                return Err(AjisaiError::from("CHARS: cannot convert Nil to characters"));
            }

            // 文字列の場合
            if is_string_value(&val) {
                let s = value_as_string(&val).unwrap_or_default();
                if s.is_empty() {
                    interp.stack.push(val);
                    return Err(AjisaiError::from("CHARS: empty string has no characters"));
                }

                let chars: Vec<Value> = s.chars()
                    .map(|c| Value::from_string(&c.to_string()))
                    .collect();

                interp.stack.push(Value::from_vector(chars));
                return Ok(());
            }

            // 数値の場合
            if is_number_value(&val) {
                interp.stack.push(val);
                return Err(AjisaiError::from("CHARS: cannot convert Number to characters"));
            }

            // 真偽値の場合
            if is_boolean_value(&val) {
                interp.stack.push(val);
                return Err(AjisaiError::from("CHARS: cannot convert Boolean to characters"));
            }

            // その他はエラー
            interp.stack.push(val);
            Err(AjisaiError::from("CHARS: requires string format"))
        }
        OperationTarget::Stack => {
            // スタック上の各要素に対してCHARSを適用
            let stack_len = interp.stack.len();
            if stack_len == 0 {
                return Err(AjisaiError::StackUnderflow);
            }

            let mut results = Vec::with_capacity(stack_len);
            let elements: Vec<Value> = interp.stack.drain(..).collect();

            for elem in elements {
                // NILの場合
                if elem.is_nil() {
                    interp.stack = results;
                    interp.stack.push(elem);
                    return Err(AjisaiError::from("CHARS: cannot convert Nil to characters"));
                }

                // 文字列の場合
                if is_string_value(&elem) {
                    let s = value_as_string(&elem).unwrap_or_default();
                    if s.is_empty() {
                        interp.stack = results;
                        interp.stack.push(elem);
                        return Err(AjisaiError::from("CHARS: empty string has no characters"));
                    }
                    let chars: Vec<Value> = s.chars()
                        .map(|c| Value::from_string(&c.to_string()))
                        .collect();
                    results.push(Value::from_vector(chars));
                    continue;
                }

                // 数値の場合
                if is_number_value(&elem) {
                    interp.stack = results;
                    interp.stack.push(elem);
                    return Err(AjisaiError::from("CHARS: cannot convert Number to characters"));
                }

                // 真偽値の場合
                if is_boolean_value(&elem) {
                    interp.stack = results;
                    interp.stack.push(elem);
                    return Err(AjisaiError::from("CHARS: cannot convert Boolean to characters"));
                }

                // その他はエラー
                interp.stack = results;
                interp.stack.push(elem);
                return Err(AjisaiError::from("CHARS: requires string format"));
            }

            interp.stack = results;
            Ok(())
        }
    }
}

/// JOIN - 文字列ベクタを連結して単一文字列に
///
/// 【使用法】
/// ```ajisai
/// [ 'h' 'e' 'l' 'l' 'o' ] JOIN → 'hello'
/// ```
///
/// 【エラー】
/// - 空ベクタ
/// - String/Number以外の要素を含む場合
pub fn op_join(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            // NILの場合
            if val.is_nil() {
                interp.stack.push(val);
                return Err(AjisaiError::from("JOIN: requires vector format, got Nil"));
            }

            // ベクタ（複数要素）の場合
            if val.data.len() > 1 || (val.data.len() == 1 && val.shape.len() > 0) {
                // shapeベースでネストされた構造を再構築
                let elements = reconstruct_vector_elements(&val);
                if elements.is_empty() {
                    interp.stack.push(val);
                    return Err(AjisaiError::from("JOIN: empty vector has no strings to join"));
                }

                let mut result = String::new();
                for (i, elem) in elements.iter().enumerate() {
                    // 文字列の場合
                    if is_string_value(elem) {
                        if let Some(s) = value_as_string(elem) {
                            result.push_str(&s);
                            continue;
                        }
                    }

                    // 数値の場合（文字コードとして解釈）
                    if is_number_value(elem) {
                        if let Some(code) = elem.data[0].to_i64() {
                            if code >= 0 && code <= 0x10FFFF {
                                if let Some(c) = char::from_u32(code as u32) {
                                    result.push(c);
                                    continue;
                                }
                            }
                        }
                        interp.stack.push(val);
                        return Err(AjisaiError::from(format!(
                            "JOIN: invalid character code at index {}", i
                        )));
                    }

                    // その他の型
                    let type_name = if elem.is_nil() {
                        "nil"
                    } else if is_boolean_value(elem) {
                        "boolean"
                    } else {
                        "other format"
                    };
                    interp.stack.push(val);
                    return Err(AjisaiError::from(format!(
                        "JOIN: all elements must be strings, found {} at index {}",
                        type_name, i
                    )));
                }

                interp.stack.push(wrap_value(Value::from_string(&result)));
                return Ok(());
            }

            // 単一要素の場合（ベクタではない）
            let type_name = if is_string_value(&val) {
                "String"
            } else if is_number_value(&val) {
                "Number"
            } else if is_boolean_value(&val) {
                "Boolean"
            } else if is_datetime_value(&val) {
                "DateTime"
            } else {
                "other format"
            };
            interp.stack.push(val);
            Err(AjisaiError::from(format!("JOIN: requires vector format, got {}", type_name)))
        }
        OperationTarget::Stack => {
            // スタック上の各ベクタに対してJOINを適用
            let stack_len = interp.stack.len();
            if stack_len == 0 {
                return Err(AjisaiError::StackUnderflow);
            }

            let mut results = Vec::with_capacity(stack_len);
            let elements: Vec<Value> = interp.stack.drain(..).collect();

            for elem in elements {
                // NILの場合
                if elem.is_nil() {
                    interp.stack = results;
                    interp.stack.push(elem);
                    return Err(AjisaiError::from("JOIN: requires vector format, got Nil"));
                }

                // ベクタ（複数要素）の場合
                if elem.data.len() > 1 || (elem.data.len() == 1 && elem.shape.len() > 0) {
                    let vec_elements = reconstruct_vector_elements(&elem);
                    if vec_elements.is_empty() {
                        interp.stack = results;
                        interp.stack.push(elem);
                        return Err(AjisaiError::from("JOIN: empty vector has no strings to join"));
                    }

                    let mut result_str = String::new();
                    for (i, v) in vec_elements.iter().enumerate() {
                        // 文字列の場合
                        if is_string_value(v) {
                            if let Some(s) = value_as_string(v) {
                                result_str.push_str(&s);
                                continue;
                            }
                        }

                        // 数値の場合（文字コードとして解釈）
                        if is_number_value(v) {
                            if let Some(code) = v.data[0].to_i64() {
                                if code >= 0 && code <= 0x10FFFF {
                                    if let Some(c) = char::from_u32(code as u32) {
                                        result_str.push(c);
                                        continue;
                                    }
                                }
                            }
                            interp.stack = results;
                            interp.stack.push(elem);
                            return Err(AjisaiError::from(format!(
                                "JOIN: invalid character code at index {}", i
                            )));
                        }

                        // その他の型
                        let type_name = if v.is_nil() {
                            "nil"
                        } else if is_boolean_value(v) {
                            "boolean"
                        } else {
                            "other format"
                        };
                        interp.stack = results;
                        interp.stack.push(elem);
                        return Err(AjisaiError::from(format!(
                            "JOIN: all elements must be strings, found {} at index {}",
                            type_name, i
                        )));
                    }

                    results.push(wrap_value(Value::from_string(&result_str)));
                    continue;
                }

                // 単一要素の場合（ベクタではない）
                let type_name = if is_string_value(&elem) {
                    "String"
                } else if is_number_value(&elem) {
                    "Number"
                } else if is_boolean_value(&elem) {
                    "Boolean"
                } else if is_datetime_value(&elem) {
                    "DateTime"
                } else {
                    "other format"
                };
                interp.stack = results;
                interp.stack.push(elem);
                return Err(AjisaiError::from(format!("JOIN: requires vector format, got {}", type_name)));
            }

            interp.stack = results;
            Ok(())
        }
    }
}

/// ベクタの要素を再構築するヘルパー
fn reconstruct_vector_elements(val: &Value) -> Vec<Value> {
    // shapeが空または1次元の場合、各要素を個別のValueとして返す
    if val.shape.is_empty() || val.shape.len() == 1 {
        val.data.iter().map(|f| {
            // 元のdisplay_hintを継承
            match val.display_hint {
                DisplayHint::String => {
                    // 文字列の場合、各要素は1文字
                    let mut v = Value::from_fraction(f.clone());
                    v.display_hint = DisplayHint::String;
                    v.shape = vec![1];
                    v
                }
                _ => Value::from_fraction(f.clone())
            }
        }).collect()
    } else {
        // 多次元の場合は最外層の要素を再構築
        let outer_size = val.shape[0];
        let inner_size: usize = val.shape[1..].iter().product();
        let inner_shape = val.shape[1..].to_vec();

        (0..outer_size).map(|i| {
            let start = i * inner_size;
            let end = start + inner_size;
            let data = val.data[start..end].to_vec();
            Value {
                data,
                display_hint: val.display_hint,
                shape: inner_shape.clone(),
            }
        }).collect()
    }
}

/// 値を文字列表現に変換する（内部ヘルパー）
fn value_to_string_repr(value: &Value) -> String {
    if value.is_nil() {
        return "NIL".to_string();
    }

    if is_boolean_value(value) {
        return if !value.data[0].is_zero() { "TRUE".to_string() } else { "FALSE".to_string() };
    }

    if is_string_value(value) {
        return value_as_string(value).unwrap_or_default();
    }

    if is_datetime_value(value) {
        return format!("@{}", fraction_to_string(&value.data[0]));
    }

    if is_number_value(value) {
        return fraction_to_string(&value.data[0]);
    }

    // ベクタの場合
    value.data.iter()
        .map(|f| fraction_to_string(f))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;
    use num_traits::One;

    #[test]
    fn test_value_to_string_repr() {
        // Number
        let num = Value::from_fraction(Fraction::new(BigInt::from(42), BigInt::one()));
        assert_eq!(value_to_string_repr(&num), "42");

        // Boolean
        let bool_val = Value::from_bool(true);
        assert_eq!(value_to_string_repr(&bool_val), "TRUE");

        // Nil
        let nil = Value::nil();
        assert_eq!(value_to_string_repr(&nil), "NIL");
    }

    #[test]
    fn test_str_conversion() {
        let mut interp = Interpreter::new();

        // Number → String
        interp.stack.push(wrap_number(
            Fraction::new(BigInt::from(42), BigInt::one())
        ));
        op_str(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "42");
        }
    }

    #[test]
    fn test_num_conversion() {
        let mut interp = Interpreter::new();

        // String → Number
        interp.stack.push(wrap_value(Value::from_string("42")));
        op_num(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_number_value(val));
            assert_eq!(val.data[0].numerator, BigInt::from(42));
        }

        // Boolean → Number (TRUE → 1)
        interp.stack.clear();
        interp.stack.push(wrap_value(Value::from_bool(true)));
        op_num(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_number_value(val));
            assert_eq!(val.data[0].numerator, BigInt::from(1));
        }

        // Boolean → Number (FALSE → 0)
        interp.stack.clear();
        interp.stack.push(wrap_value(Value::from_bool(false)));
        op_num(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_number_value(val));
            assert_eq!(val.data[0].numerator, BigInt::from(0));
        }
    }

    #[test]
    fn test_bool_conversion() {
        let mut interp = Interpreter::new();

        // String → Boolean (TRUE)
        interp.stack.push(wrap_value(Value::from_string("TRUE")));
        op_bool(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            assert!(!val.data[0].is_zero());
        }

        // String → Boolean ('1')
        interp.stack.clear();
        interp.stack.push(wrap_value(Value::from_string("1")));
        op_bool(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            assert!(!val.data[0].is_zero());
        }

        // Number → Boolean (1 → TRUE)
        interp.stack.clear();
        interp.stack.push(wrap_number(Fraction::new(BigInt::from(1), BigInt::from(1))));
        op_bool(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            assert!(!val.data[0].is_zero());
        }

        // Number → Boolean (0 → FALSE)
        interp.stack.clear();
        interp.stack.push(wrap_number(Fraction::new(BigInt::from(0), BigInt::from(1))));
        op_bool(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            assert!(val.data[0].is_zero());
        }
    }

    #[tokio::test]
    async fn test_chars_basic() {
        let mut interp = Interpreter::new();
        interp.execute("[ 'hello' ] CHARS JOIN").await.unwrap();
        assert_eq!(interp.stack.len(), 1);

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "hello");
        }
    }

    #[tokio::test]
    #[ignore] // TODO: Unicode strings need proper code point handling in unified architecture
    async fn test_chars_unicode() {
        let mut interp = Interpreter::new();
        interp.execute("[ '日本語' ] CHARS JOIN").await.unwrap();
        assert_eq!(interp.stack.len(), 1);

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "日本語");
        }
    }

    #[tokio::test]
    async fn test_chars_structure_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 42 ] CHARS").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_join_basic() {
        let mut interp = Interpreter::new();
        interp.execute("[ 'h' 'e' 'l' 'l' 'o' ] JOIN").await.unwrap();
        assert_eq!(interp.stack.len(), 1);

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "hello");
        }
    }

    #[tokio::test]
    #[ignore] // TODO: Variable-length strings in vectors need proper handling in unified architecture
    async fn test_join_multichar() {
        let mut interp = Interpreter::new();
        interp.execute("[ 'hel' 'lo' ] JOIN").await.unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "hello");
        }
    }

    #[tokio::test]
    async fn test_join_empty_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ ] JOIN").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chars_join_roundtrip() {
        let mut interp = Interpreter::new();
        interp.execute("[ 'hello' ] CHARS JOIN").await.unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "hello");
        }
    }

    #[tokio::test]
    async fn test_chars_reverse_join() {
        let mut interp = Interpreter::new();
        interp.execute("[ 'hello' ] CHARS REVERSE JOIN").await.unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "olleh");
        }
    }

    #[tokio::test]
    async fn test_nil_success_case() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 'nil' ] NIL").await;
        assert!(result.is_ok());
        assert_eq!(interp.stack.len(), 1);

        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil());
        }
    }

    #[tokio::test]
    async fn test_nil_case_insensitive() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 'NIL' ] NIL").await;
        assert!(result.is_ok());

        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil());
        }
    }
}
