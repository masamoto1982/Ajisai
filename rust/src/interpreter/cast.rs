// rust/src/interpreter/cast.rs
//
// 【責務】
// CAST ワードを実装する。型変換の双方向性を持つ唯一のワード。
// String ⇔ Vector の相互変換、および各型から String への変換を提供する。
//
// 【設計原則】
// - 同型変換はエラー（型安全性の維持）
// - 明示的な変換のみ（暗黙的型変換を排除）
// - 型推論によるパース（String → Vector 時）

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{wrap_in_square_vector, extract_single_element};
use crate::types::{Value, ValueType, BracketType};
use crate::types::fraction::Fraction;

/// CAST - 型を変換する
///
/// 【責務】
/// - Vector → String: ベクトルを文字列表現に変換
/// - String → Vector: 文字列をパースしてベクトルに変換（型推論付き）
/// - Number/Boolean/Nil/Symbol → String: 各型を文字列化
/// - 同型変換はエラー
///
/// 【使用法】
/// ```ajisai
/// # Vector → String
/// [ 1 2 3 ] CAST → [ '1 2 3' ]
///
/// # String → Vector（型推論）
/// [ '1 2 3' ] CAST → [ 1 2 3 ]
/// [ 'TRUE FALSE' ] CAST → [ TRUE FALSE ]
/// [ 'hello world' ] CAST → [ 'hello' 'world' ]
/// [ '42 nil 3.14' ] CAST → [ 42 nil 7/25 ]
///
/// # 各型 → String
/// [ 42 ] CAST → [ '42' ]
/// [ TRUE ] CAST → [ 'TRUE' ]
/// [ nil ] CAST → [ 'nil' ]
/// ```
///
/// 【エラー】
/// - 同型変換（String → String, Vector → Vector等）の場合
/// - StackTopモード以外での使用
pub fn op_cast(interp: &mut Interpreter) -> Result<()> {
    // CASTはStackTopモードのみサポート
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("CAST only supports StackTop mode"));
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 単一要素ベクトルの中身を取得
    let inner_val = extract_single_element(&val)?;

    match &inner_val.val_type {
        // Vector → String
        ValueType::Vector(_, _) | ValueType::SingletonVector(_, _) => {
            let string_repr = value_to_string_repr(inner_val);
            interp.stack.push(wrap_in_square_vector(
                Value { val_type: ValueType::String(string_repr) }
            ));
            Ok(())
        }

        // String → Vector
        ValueType::String(s) => {
            let parsed_vec = parse_string_to_typed_vector(s)?;
            interp.stack.push(parsed_vec);
            Ok(())
        }

        // Number/Boolean/Nil/Symbol → String
        ValueType::Number(_) | ValueType::Boolean(_) |
        ValueType::Nil | ValueType::Symbol(_) => {
            let string_repr = value_to_string_repr(inner_val);
            interp.stack.push(wrap_in_square_vector(
                Value { val_type: ValueType::String(string_repr) }
            ));
            Ok(())
        }
    }
}

/// 値を文字列表現に変換する（内部ヘルパー）
///
/// 【責務】
/// - 各型を人間が読める文字列表現に変換
/// - ベクトルは要素をスペース区切りで結合
/// - ネストされたベクトルは括弧で表現
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

/// 文字列を型推論してベクトルに変換する（内部ヘルパー）
///
/// 【責務】
/// - 文字列をスペースで分割
/// - 各要素を型推論してパース
///   - 数値として解釈可能 → Number
///   - "TRUE"/"FALSE" → Boolean
///   - "nil" → Nil
///   - それ以外 → String
///
/// 【引数】
/// - s: パースする文字列
///
/// 【戻り値】
/// - パースされたベクトル（角括弧で包まれた状態）
///
/// 【エラー】
/// - 数値パースに失敗した場合（形式は数値だが範囲外等）
fn parse_string_to_typed_vector(s: &str) -> Result<Value> {
    // 空文字列の場合は空ベクトル
    if s.trim().is_empty() {
        return Ok(Value {
            val_type: ValueType::Vector(Vec::new(), BracketType::Square)
        });
    }

    let parts: Vec<&str> = s.split_whitespace().collect();
    let mut elements = Vec::new();

    for part in parts {
        let elem = parse_token_with_type_inference(part)?;
        elements.push(elem);
    }

    Ok(Value {
        val_type: ValueType::Vector(elements, BracketType::Square)
    })
}

/// 単一のトークン文字列を型推論してValueに変換する（内部ヘルパー）
///
/// 【責務】
/// - 文字列の内容から型を推論
/// - 優先順位: Number > Boolean > Nil > String
///
/// 【引数】
/// - token: パースするトークン文字列
///
/// 【戻り値】
/// - 推論された型のValue
///
/// 【エラー】
/// - 数値パースに失敗した場合
fn parse_token_with_type_inference(token: &str) -> Result<Value> {
    // 数値として解釈を試みる
    if let Ok(fraction) = Fraction::from_str(token) {
        return Ok(Value { val_type: ValueType::Number(fraction) });
    }

    // Booleanとして解釈（大文字小文字を無視）
    let upper = token.to_uppercase();
    if upper == "TRUE" {
        return Ok(Value { val_type: ValueType::Boolean(true) });
    }
    if upper == "FALSE" {
        return Ok(Value { val_type: ValueType::Boolean(false) });
    }

    // Nilとして解釈（大文字小文字を無視）
    if upper == "NIL" {
        return Ok(Value { val_type: ValueType::Nil });
    }

    // それ以外は文字列として扱う
    Ok(Value { val_type: ValueType::String(token.to_string()) })
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
    fn test_parse_token_with_type_inference() {
        // Number
        let num = parse_token_with_type_inference("42").unwrap();
        assert!(matches!(num.val_type, ValueType::Number(_)));

        // Boolean
        let bool_true = parse_token_with_type_inference("TRUE").unwrap();
        assert_eq!(bool_true.val_type, ValueType::Boolean(true));

        let bool_false = parse_token_with_type_inference("false").unwrap(); // 小文字でもOK
        assert_eq!(bool_false.val_type, ValueType::Boolean(false));

        // Nil
        let nil = parse_token_with_type_inference("nil").unwrap();
        assert_eq!(nil.val_type, ValueType::Nil);

        // String
        let string = parse_token_with_type_inference("hello").unwrap();
        assert!(matches!(string.val_type, ValueType::String(s) if s == "hello"));
    }

    #[test]
    fn test_parse_string_to_typed_vector() {
        let result = parse_string_to_typed_vector("1 TRUE nil hello").unwrap();

        if let ValueType::Vector(elements, _) = result.val_type {
            assert_eq!(elements.len(), 4);
            assert!(matches!(elements[0].val_type, ValueType::Number(_)));
            assert!(matches!(elements[1].val_type, ValueType::Boolean(true)));
            assert!(matches!(elements[2].val_type, ValueType::Nil));
            assert!(matches!(elements[3].val_type, ValueType::String(ref s) if s == "hello"));
        } else {
            panic!("Expected Vector");
        }
    }
}
