// rust/src/interpreter/cast.rs
//
// 【責務】
// 型変換ワード群を実装する。
// STR: 任意の型 → String
// NUM: String/Boolean → Number
// BOOL: String/Number → Boolean
// NIL: String → Nil
//
// 【設計原則】
// - 同型変換はエラー（型安全性の維持）
// - 明示的な変換のみ（暗黙的型変換を排除）
// - 型推論を排除（パース失敗は即エラー）

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{wrap_in_square_vector, extract_single_element};
use crate::types::{Value, ValueType};
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
    let val_clone = val.clone();
    let inner_val = extract_single_element(&val_clone)?;

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

/// BOOL - 文字列または数値を真偽値に変換
///
/// 【責務】
/// - String → Boolean（"TRUE"/"FALSE", "1"/"0", "真"/"偽"、大小文字無視）
/// - Number → Boolean（1 → TRUE、0 → FALSE）
/// - Boolean → エラー（同型変換）
/// - Nil → エラー
/// - 他の型はエラー
///
/// 【使用法】
/// ```ajisai
/// [ 'TRUE' ] BOOL → [ TRUE ]
/// [ 'false' ] BOOL → [ FALSE ]
/// [ '1' ] BOOL → [ TRUE ]
/// [ '0' ] BOOL → [ FALSE ]
/// [ '真' ] BOOL → [ TRUE ]
/// [ '偽' ] BOOL → [ FALSE ]
/// [ 1 ] BOOL → [ TRUE ]
/// [ 0 ] BOOL → [ FALSE ]
/// [ 'hello' ] BOOL → ERROR
/// [ TRUE ] BOOL → ERROR（同型変換）
/// [ nil ] BOOL → ERROR
/// ```
///
/// 【エラー】
/// - 真偽値として認識できない文字列
/// - 1または0以外の数値
/// - Boolean型（同型変換）
/// - Nil型
/// - その他の型
pub fn op_bool(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("BOOL only supports StackTop mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let val_clone = val.clone();
    let inner_val = extract_single_element(&val_clone)?;

    match &inner_val.val_type {
        ValueType::String(s) => {
            let upper = s.to_uppercase();
            let bool_val = if upper == "TRUE" || upper == "1" || s == "真" {
                true
            } else if upper == "FALSE" || upper == "0" || s == "偽" {
                false
            } else {
                return Err(AjisaiError::from(format!(
                    "BOOL: cannot parse '{}' as boolean (expected 'TRUE'/'FALSE', '1'/'0', '真'/'偽')", s
                )));
            };
            interp.stack.push(wrap_in_square_vector(
                Value { val_type: ValueType::Boolean(bool_val) }
            ));
            Ok(())
        }
        ValueType::Number(n) => {
            use num_bigint::BigInt;
            use num_traits::One;

            let one = Fraction::new(BigInt::one(), BigInt::one());
            let zero = Fraction::new(BigInt::from(0), BigInt::one());

            if n == &one {
                interp.stack.push(wrap_in_square_vector(
                    Value { val_type: ValueType::Boolean(true) }
                ));
                Ok(())
            } else if n == &zero {
                interp.stack.push(wrap_in_square_vector(
                    Value { val_type: ValueType::Boolean(false) }
                ));
                Ok(())
            } else {
                interp.stack.push(val);
                Err(AjisaiError::from(format!(
                    "BOOL: cannot convert number {} to boolean (only 1 and 0 are allowed)",
                    if n.denominator == BigInt::one() {
                        format!("{}", n.numerator)
                    } else {
                        format!("{}/{}", n.numerator, n.denominator)
                    }
                )))
            }
        }
        ValueType::Boolean(_) => {
            interp.stack.push(val);
            Err(AjisaiError::from("BOOL: same-type conversion (Boolean → Boolean) is not allowed"))
        }
        ValueType::Nil => {
            interp.stack.push(val);
            Err(AjisaiError::from("BOOL: cannot convert Nil to Boolean"))
        }
        _ => {
            interp.stack.push(val);
            Err(AjisaiError::from("BOOL: requires String or Number type"))
        }
    }
}

/// NIL - 文字列をNilに変換
///
/// 【責務】
/// - String → Nil（"nil" のみ、大小文字無視）
/// - Boolean → エラー
/// - Number → エラー
/// - Nil → エラー（同型変換）
/// - 他の型はエラー
///
/// 【使用法】
/// ```ajisai
/// [ 'nil' ] NIL → [ nil ]
/// [ 'NIL' ] NIL → [ nil ]
/// [ 'hello' ] NIL → ERROR
/// [ TRUE ] NIL → ERROR
/// [ 42 ] NIL → ERROR
/// [ nil ] NIL → ERROR（同型変換）
/// ```
///
/// 【エラー】
/// - "nil"以外の文字列
/// - Boolean型
/// - Number型
/// - Nil型（同型変換）
/// - その他の型
pub fn op_nil(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("NIL only supports StackTop mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let val_clone = val.clone();
    let inner_val = extract_single_element(&val_clone)?;

    match &inner_val.val_type {
        ValueType::String(s) => {
            let upper = s.to_uppercase();
            if upper == "NIL" {
                interp.stack.push(wrap_in_square_vector(
                    Value { val_type: ValueType::Nil }
                ));
                Ok(())
            } else {
                interp.stack.push(val);
                Err(AjisaiError::from(format!(
                    "NIL: cannot parse '{}' as nil (expected 'nil')", s
                )))
            }
        }
        ValueType::Boolean(_) => {
            interp.stack.push(val);
            Err(AjisaiError::from("NIL: cannot convert Boolean to Nil"))
        }
        ValueType::Number(_) => {
            interp.stack.push(val);
            Err(AjisaiError::from("NIL: cannot convert Number to Nil"))
        }
        ValueType::Nil => {
            interp.stack.push(val);
            Err(AjisaiError::from("NIL: same-type conversion (Nil → Nil) is not allowed"))
        }
        _ => {
            interp.stack.push(val);
            Err(AjisaiError::from("NIL: requires String type"))
        }
    }
}

/// CHARS - 文字列を文字ベクタに分解
///
/// 【責務】
/// - String → Vector[String]（各要素は1文字）
/// - UTF-8マルチバイト文字を正しく処理
///
/// 【エラー】
/// - 空文字列（"No change is an error"原則）
/// - String以外の型
pub fn op_chars(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let inner_val = extract_single_element(&val)?;

            match &inner_val.val_type {
                ValueType::String(s) => {
                    if s.is_empty() {
                        interp.stack.push(val);
                        return Err(AjisaiError::from("CHARS: empty string has no characters"));
                    }

                    let chars: Vec<Value> = s.chars()
                        .map(|c| Value { val_type: ValueType::String(c.to_string()) })
                        .collect();

                    interp.stack.push(Value {
                        val_type: ValueType::Vector(chars)
                    });
                    Ok(())
                }
                _ => {
                    interp.stack.push(val);
                    Err(AjisaiError::type_error("string", "other type"))
                }
            }
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
                let inner = extract_single_element(&elem)?;
                match &inner.val_type {
                    ValueType::String(s) => {
                        if s.is_empty() {
                            // エラー時はスタックを復元
                            interp.stack = results;
                            interp.stack.push(elem);
                            return Err(AjisaiError::from("CHARS: empty string has no characters"));
                        }
                        let chars: Vec<Value> = s.chars()
                            .map(|c| Value { val_type: ValueType::String(c.to_string()) })
                            .collect();
                        results.push(Value {
                            val_type: ValueType::Vector(chars)
                        });
                    }
                    _ => {
                        interp.stack = results;
                        interp.stack.push(elem);
                        return Err(AjisaiError::type_error("string", "other type"));
                    }
                }
            }

            interp.stack = results;
            Ok(())
        }
    }
}

/// JOIN - 文字列ベクタを連結して単一文字列に
///
/// 【責務】
/// - Vector[String] → String
/// - 全要素がString型であることを検証
///
/// 【エラー】
/// - 空ベクタ
/// - String以外の要素を含む場合
pub fn op_join(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            match &val.val_type {
                ValueType::Vector(vec) => {
                    if vec.is_empty() {
                        interp.stack.push(val);
                        return Err(AjisaiError::from("JOIN: empty vector has no strings to join"));
                    }

                    let mut result = String::new();
                    for (i, elem) in vec.iter().enumerate() {
                        match &elem.val_type {
                            ValueType::String(s) => {
                                result.push_str(s);
                            }
                            other => {
                                let type_name = match other {
                                    ValueType::Number(_) => "number",
                                    ValueType::Boolean(_) => "boolean",
                                    ValueType::Nil => "nil",
                                    ValueType::Vector(_) => "vector",
                                    ValueType::Symbol(_) => "symbol",
                                    _ => "other type",
                                };
                                interp.stack.push(val);
                                return Err(AjisaiError::from(format!(
                                    "JOIN: all elements must be strings, found {} at index {}",
                                    type_name, i
                                )));
                            }
                        }
                    }

                    interp.stack.push(wrap_in_square_vector(
                        Value { val_type: ValueType::String(result) }
                    ));
                    Ok(())
                }
                _ => {
                    interp.stack.push(val);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
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
                match &elem.val_type {
                    ValueType::Vector(vec) => {
                        if vec.is_empty() {
                            interp.stack = results;
                            interp.stack.push(elem);
                            return Err(AjisaiError::from("JOIN: empty vector has no strings to join"));
                        }

                        let mut result = String::new();
                        for (i, v) in vec.iter().enumerate() {
                            match &v.val_type {
                                ValueType::String(s) => {
                                    result.push_str(s);
                                }
                                other => {
                                    let type_name = match other {
                                        ValueType::Number(_) => "number",
                                        ValueType::Boolean(_) => "boolean",
                                        ValueType::Nil => "nil",
                                        ValueType::Vector(_) => "vector",
                                        ValueType::Symbol(_) => "symbol",
                                        _ => "other type",
                                    };
                                    interp.stack = results;
                                    interp.stack.push(elem);
                                    return Err(AjisaiError::from(format!(
                                        "JOIN: all elements must be strings, found {} at index {}",
                                        type_name, i
                                    )));
                                }
                            }
                        }

                        results.push(wrap_in_square_vector(
                            Value { val_type: ValueType::String(result) }
                        ));
                    }
                    _ => {
                        interp.stack = results;
                        interp.stack.push(elem);
                        return Err(AjisaiError::type_error("vector", "other type"));
                    }
                }
            }

            interp.stack = results;
            Ok(())
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
        ValueType::Nil => "NIL".to_string(),
        ValueType::Vector(vec) => {
            vec.iter()
                .map(|v| value_to_string_repr(v))
                .collect::<Vec<_>>()
                .join(" ")
        }
        ValueType::TailCallMarker => "<TAIL_CALL>".to_string(),
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
        assert_eq!(value_to_string_repr(&nil), "NIL");

        // Vector
        let vec = Value {
            val_type: ValueType::Vector(
                vec![
                    Value { val_type: ValueType::Number(Fraction::new(BigInt::from(1), BigInt::one())) },
                    Value { val_type: ValueType::Number(Fraction::new(BigInt::from(2), BigInt::one())) },
                    Value { val_type: ValueType::Number(Fraction::new(BigInt::from(3), BigInt::one())) },
                ]
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
            if let ValueType::Vector(v) = &val.val_type {
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
            if let ValueType::Vector(v) = &val.val_type {
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
            if let ValueType::Vector(v) = &val.val_type {
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
            if let ValueType::Vector(v) = &val.val_type {
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

        // String → Boolean (TRUE/FALSE)
        interp.stack.push(wrap_in_square_vector(
            Value { val_type: ValueType::String("TRUE".to_string()) }
        ));
        op_bool(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                if let ValueType::Boolean(b) = &v[0].val_type {
                    assert_eq!(*b, true);
                }
            }
        }

        // String → Boolean ('1'/'0')
        interp.stack.clear();
        interp.stack.push(wrap_in_square_vector(
            Value { val_type: ValueType::String("1".to_string()) }
        ));
        op_bool(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                if let ValueType::Boolean(b) = &v[0].val_type {
                    assert_eq!(*b, true);
                }
            }
        }

        interp.stack.clear();
        interp.stack.push(wrap_in_square_vector(
            Value { val_type: ValueType::String("0".to_string()) }
        ));
        op_bool(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                if let ValueType::Boolean(b) = &v[0].val_type {
                    assert_eq!(*b, false);
                }
            }
        }

        // String → Boolean ('真'/'偽')
        interp.stack.clear();
        interp.stack.push(wrap_in_square_vector(
            Value { val_type: ValueType::String("真".to_string()) }
        ));
        op_bool(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                if let ValueType::Boolean(b) = &v[0].val_type {
                    assert_eq!(*b, true);
                }
            }
        }

        interp.stack.clear();
        interp.stack.push(wrap_in_square_vector(
            Value { val_type: ValueType::String("偽".to_string()) }
        ));
        op_bool(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                if let ValueType::Boolean(b) = &v[0].val_type {
                    assert_eq!(*b, false);
                }
            }
        }

        // Number → Boolean (1 → TRUE)
        interp.stack.clear();
        interp.stack.push(wrap_in_square_vector(
            Value { val_type: ValueType::Number(Fraction::new(BigInt::from(1), BigInt::from(1))) }
        ));
        op_bool(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                if let ValueType::Boolean(b) = &v[0].val_type {
                    assert_eq!(*b, true);
                }
            }
        }

        // Number → Boolean (0 → FALSE)
        interp.stack.clear();
        interp.stack.push(wrap_in_square_vector(
            Value { val_type: ValueType::Number(Fraction::new(BigInt::from(0), BigInt::from(1))) }
        ));
        op_bool(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                if let ValueType::Boolean(b) = &v[0].val_type {
                    assert_eq!(*b, false);
                }
            }
        }
    }

    #[tokio::test]
    async fn test_chars_basic() {
        let mut interp = Interpreter::new();
        interp.execute("[ 'hello' ] CHARS").await.unwrap();
        assert_eq!(interp.stack.len(), 1);

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 5);
                assert!(matches!(&v[0].val_type, ValueType::String(s) if s == "h"));
                assert!(matches!(&v[1].val_type, ValueType::String(s) if s == "e"));
                assert!(matches!(&v[2].val_type, ValueType::String(s) if s == "l"));
                assert!(matches!(&v[3].val_type, ValueType::String(s) if s == "l"));
                assert!(matches!(&v[4].val_type, ValueType::String(s) if s == "o"));
            }
        }
    }

    #[tokio::test]
    async fn test_chars_unicode() {
        let mut interp = Interpreter::new();
        interp.execute("[ '日本語' ] CHARS").await.unwrap();
        assert_eq!(interp.stack.len(), 1);

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 3);
                assert!(matches!(&v[0].val_type, ValueType::String(s) if s == "日"));
                assert!(matches!(&v[1].val_type, ValueType::String(s) if s == "本"));
                assert!(matches!(&v[2].val_type, ValueType::String(s) if s == "語"));
            }
        }
    }

    #[tokio::test]
    async fn test_chars_empty_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ '' ] CHARS").await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("empty string"));
        }
    }

    #[tokio::test]
    async fn test_chars_type_error() {
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
            if let ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 1);
                assert!(matches!(&v[0].val_type, ValueType::String(s) if s == "hello"));
            }
        }
    }

    #[tokio::test]
    async fn test_join_multichar() {
        let mut interp = Interpreter::new();
        interp.execute("[ 'hel' 'lo' ] JOIN").await.unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 1);
                assert!(matches!(&v[0].val_type, ValueType::String(s) if s == "hello"));
            }
        }
    }

    #[tokio::test]
    async fn test_join_empty_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ ] JOIN").await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("empty vector"));
        }
    }

    #[tokio::test]
    async fn test_join_type_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 'a' 1 'b' ] JOIN").await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("must be strings"));
        }
    }

    #[tokio::test]
    async fn test_chars_join_roundtrip() {
        let mut interp = Interpreter::new();
        interp.execute("[ 'hello' ] CHARS JOIN").await.unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 1);
                assert!(matches!(&v[0].val_type, ValueType::String(s) if s == "hello"));
            }
        }
    }

    #[tokio::test]
    async fn test_chars_reverse_join() {
        let mut interp = Interpreter::new();
        interp.execute("[ 'hello' ] CHARS REVERSE JOIN").await.unwrap();

        if let Some(val) = interp.stack.last() {
            if let ValueType::Vector(v) = &val.val_type {
                assert_eq!(v.len(), 1);
                assert!(matches!(&v[0].val_type, ValueType::String(s) if s == "olleh"));
            }
        }
    }

}
