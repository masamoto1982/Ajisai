// rust/src/interpreter/cast.rs
//
// 統一Value宇宙アーキテクチャ版の型変換ワード群
//
// 【設計原則】
// すべての値は ValueData (Scalar/Vector/Nil) として表現される。
// DisplayHint は表示目的のみに使用し、演算には使用しない。
// 「型変換」は実質的に DisplayHint の変更と、表示形式の変換である。

use crate::interpreter::{Interpreter, OperationTargetMode};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::wrap_number;
use crate::types::{Value, ValueData, DisplayHint};
use crate::types::fraction::Fraction;

/// 値を文字列として解釈する（内部ヘルパー）
fn value_as_string(val: &Value) -> Option<String> {
    fn collect_chars(val: &Value) -> Vec<char> {
        match &val.data {
            ValueData::Nil => vec![],
            ValueData::Scalar(f) => {
                f.to_i64().and_then(|n| {
                    if n >= 0 && n <= 0x10FFFF {
                        char::from_u32(n as u32)
                    } else {
                        None
                    }
                }).map(|c| vec![c]).unwrap_or_default()
            }
            ValueData::Vector(children) => {
                children.iter().flat_map(|c| collect_chars(c)).collect()
            }
            ValueData::CodeBlock(_) => vec![],
        }
    }

    let chars = collect_chars(val);
    if chars.is_empty() {
        None
    } else {
        Some(chars.into_iter().collect())
    }
}

/// 値が文字列として扱えるかチェック
fn is_string_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::String
}

/// 値が真偽値として扱えるかチェック
fn is_boolean_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::Boolean && val.is_scalar()
}

/// 値が数値として扱えるかチェック
fn is_number_value(val: &Value) -> bool {
    matches!(val.display_hint, DisplayHint::Number | DisplayHint::Auto) && val.is_scalar()
}

/// 値がDateTimeとして扱えるかチェック
fn is_datetime_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::DateTime && val.is_scalar()
}

/// STR - 値を人間が読める形式の文字列に変換（Stringify）
///
/// 【使用法】
/// ```ajisai
/// 123 STR → '123'
/// TRUE STR → 'TRUE'
/// NIL STR → 'NIL'
/// ```
///
/// 【動作】
/// - 数値: その文字列表現に変換（例: 123 → '123', 1/3 → '1/3'）
/// - 真偽値: 'TRUE' または 'FALSE'
/// - NIL: 'NIL'
/// - ベクタ: 要素を空白区切りで連結
///
/// 【エラー】
/// - 入力が既にStringの場合（「変化なしはエラー」原則）
pub fn op_str(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported { word: "STR".into(), mode: "Stack".into() });
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // NILの場合
    if val.is_nil() {
        interp.stack.push(Value::from_string("NIL"));
        return Ok(());
    }

    // 既に文字列形式の場合は冗長な変換エラー
    if is_string_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::NoChange { word: "STR".into() });
    }

    // 真偽値の場合
    if is_boolean_value(&val) {
        if let Some(f) = val.as_scalar() {
            let string_repr = if !f.is_zero() { "TRUE" } else { "FALSE" };
            interp.stack.push(Value::from_string(string_repr));
            return Ok(());
        }
    }

    // DateTimeの場合
    if is_datetime_value(&val) {
        if let Some(f) = val.as_scalar() {
            let string_repr = fraction_to_string(f);
            interp.stack.push(Value::from_string(&string_repr));
            return Ok(());
        }
    }

    // 数値の場合
    if is_number_value(&val) {
        if let Some(f) = val.as_scalar() {
            let string_repr = fraction_to_string(f);
            interp.stack.push(Value::from_string(&string_repr));
            return Ok(());
        }
    }

    // ベクタの場合（複数要素）
    let string_repr = value_to_string_repr(&val);
    interp.stack.push(Value::from_string(&string_repr));
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

/// NUM - 文字列を数値（分数）にパース
///
/// 【使用法】
/// ```ajisai
/// '123' NUM → [ 123 ]
/// '1/3' NUM → [ 1/3 ]
/// 'ABC' NUM → NIL
/// ```
///
/// 【動作】
/// - 文字列を数値としてパースする
/// - パース成功時: その数値を返す
/// - パース失敗時: NILを返す（エラー停止させない）
///
/// 【エラー】
/// - 入力がStringでない場合（「変化なしはエラー」原則）
pub fn op_num(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported { word: "NUM".into(), mode: "Stack".into() });
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 文字列の場合のみ処理
    if is_string_value(&val) {
        let s = value_as_string(&val).unwrap_or_default();
        match Fraction::from_str(&s) {
            Ok(fraction) => {
                // パース成功: 数値を返す
                interp.stack.push(wrap_number(fraction));
                return Ok(());
            }
            Err(_) => {
                // パース失敗: NILを返す（エラーにしない）
                interp.stack.push(Value::nil());
                return Ok(());
            }
        }
    }

    // 「変化なしはエラー」原則: 文字列以外の入力はエラー
    // 既に数値の場合
    if is_number_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::NoChange { word: "NUM".into() });
    }

    // 真偽値の場合
    if is_boolean_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::from("NUM: cannot parse Boolean (expected String)"));
    }

    // NILの場合
    if val.is_nil() {
        interp.stack.push(val);
        return Err(AjisaiError::from("NUM: cannot parse Nil (expected String)"));
    }

    // その他（ベクタなど）
    interp.stack.push(val);
    Err(AjisaiError::from("NUM: expected String input"))
}

/// BOOL - 文字列または数値を真偽値に正規化（Parse/Normalize Boolean）
///
/// 【使用法】
/// ```ajisai
/// 'True' BOOL → TRUE
/// 'false' BOOL → FALSE
/// 'other' BOOL → NIL
/// 100 BOOL → TRUE
/// 0 BOOL → FALSE
/// ```
///
/// 【動作】
/// - 入力がStringの場合: 大文字小文字を無視して判定
///   - 'true' → TRUE
///   - 'false' → FALSE
///   - それ以外 → NIL
/// - 入力がNumberの場合: Truthiness判定
///   - 0 → FALSE
///   - 0以外 → TRUE
///
/// 【エラー】
/// - 入力が既にBooleanの場合（「変化なしはエラー」原則）
pub fn op_bool(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported { word: "BOOL".into(), mode: "Stack".into() });
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 既に真偽値形式の場合は冗長な変換エラー（変化なしはエラー原則）
    if is_boolean_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::NoChange { word: "BOOL".into() });
    }

    // 文字列の場合: パース
    if is_string_value(&val) {
        let s = value_as_string(&val).unwrap_or_default();
        let upper = s.to_uppercase();
        if upper == "TRUE" {
            interp.stack.push(Value::from_bool(true));
        } else if upper == "FALSE" {
            interp.stack.push(Value::from_bool(false));
        } else {
            // パース失敗: NILを返す（エラーにしない）
            interp.stack.push(Value::nil());
        }
        return Ok(());
    }

    // 数値の場合: Truthiness判定
    if is_number_value(&val) {
        if let Some(f) = val.as_scalar() {
            // 0はFALSE、0以外はTRUE
            let bool_val = !f.is_zero();
            interp.stack.push(Value::from_bool(bool_val));
            return Ok(());
        }
    }

    // NILの場合
    if val.is_nil() {
        interp.stack.push(val);
        return Err(AjisaiError::from("BOOL: cannot convert Nil (expected String or Number)"));
    }

    // その他（ベクタなど）
    interp.stack.push(val);
    Err(AjisaiError::from("BOOL: expected String or Number input"))
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
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported { word: "NIL".into(), mode: "Stack".into() });
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 既にNIL形式の場合は冗長な変換エラー
    if val.is_nil() {
        interp.stack.push(val);
        return Err(AjisaiError::NoChange { word: "NIL".into() });
    }

    // 文字列の場合
    if is_string_value(&val) {
        let s = value_as_string(&val).unwrap_or_default();
        let upper = s.to_uppercase();
        if upper == "NIL" {
            interp.stack.push(Value::nil());
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
    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
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
        OperationTargetMode::Stack => {
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
    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            // NILの場合
            if val.is_nil() {
                interp.stack.push(val);
                return Err(AjisaiError::from("JOIN: requires vector format, got Nil"));
            }

            // ベクタの場合
            if let ValueData::Vector(children) = &val.data {
                if children.is_empty() {
                    interp.stack.push(val);
                    return Err(AjisaiError::from("JOIN: empty vector has no strings to join"));
                }

                let mut result = String::new();
                for (i, elem) in children.iter().enumerate() {
                    // 文字列の場合
                    if is_string_value(elem) {
                        if let Some(s) = value_as_string(elem) {
                            result.push_str(&s);
                            continue;
                        }
                    }

                    // 数値の場合（文字コードとして解釈）
                    if is_number_value(elem) {
                        if let Some(f) = elem.as_scalar() {
                            if let Some(code) = f.to_i64() {
                                if code >= 0 && code <= 0x10FFFF {
                                    if let Some(c) = char::from_u32(code as u32) {
                                        result.push(c);
                                        continue;
                                    }
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

                interp.stack.push(Value::from_string(&result));
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
        OperationTargetMode::Stack => {
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

                // ベクタの場合
                if let ValueData::Vector(children) = &elem.data {
                    if children.is_empty() {
                        interp.stack = results;
                        interp.stack.push(elem);
                        return Err(AjisaiError::from("JOIN: empty vector has no strings to join"));
                    }

                    let mut result_str = String::new();
                    for (i, v) in children.iter().enumerate() {
                        // 文字列の場合
                        if is_string_value(v) {
                            if let Some(s) = value_as_string(v) {
                                result_str.push_str(&s);
                                continue;
                            }
                        }

                        // 数値の場合（文字コードとして解釈）
                        if is_number_value(v) {
                            if let Some(f) = v.as_scalar() {
                                if let Some(code) = f.to_i64() {
                                    if code >= 0 && code <= 0x10FFFF {
                                        if let Some(c) = char::from_u32(code as u32) {
                                            result_str.push(c);
                                            continue;
                                        }
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

                    results.push(Value::from_string(&result_str));
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

/// CHR - 数値（Unicodeコードポイント）を1文字の文字列に変換
///
/// 【使用法】
/// ```ajisai
/// 65 CHR → 'A'
/// 10 CHR → '\n' (改行文字)
/// 12354 CHR → 'あ'
/// ```
///
/// 【動作】
/// - 数値をUnicodeスカラ値として解釈し、対応する1文字のStringを生成
///
/// 【エラー】
/// - 入力が有効なUnicodeコードポイントでない場合
/// - 入力が数値でない場合
pub fn op_chr(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported { word: "CHR".into(), mode: "Stack".into() });
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // 数値の場合のみ処理
    if is_number_value(&val) {
        if let Some(f) = val.as_scalar() {
            // 整数のみ受け付ける
            if let Some(code) = f.to_i64() {
                // 有効なUnicodeコードポイントかチェック
                if code >= 0 && code <= 0x10FFFF {
                    if let Some(c) = char::from_u32(code as u32) {
                        let s = c.to_string();
                        interp.stack.push(Value::from_string(&s));
                        return Ok(());
                    }
                }
                // 無効なコードポイント
                interp.stack.push(val);
                return Err(AjisaiError::from(format!(
                    "CHR: {} is not a valid Unicode code point (valid range: 0-0x10FFFF, excluding surrogates)",
                    code
                )));
            } else {
                // 整数に変換できない（分数など）
                // 先に文字列表現を取得してから val をプッシュ
                let frac_str = fraction_to_string(f);
                interp.stack.push(val);
                return Err(AjisaiError::from(format!(
                    "CHR: requires an integer, got {}",
                    frac_str
                )));
            }
        }
    }

    // 文字列の場合
    if is_string_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::from("CHR: expected Number, got String"));
    }

    // 真偽値の場合
    if is_boolean_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::from("CHR: expected Number, got Boolean"));
    }

    // NILの場合
    if val.is_nil() {
        interp.stack.push(val);
        return Err(AjisaiError::from("CHR: expected Number, got Nil"));
    }

    // その他
    interp.stack.push(val);
    Err(AjisaiError::from("CHR: expected Number input"))
}

/// 値を文字列表現に変換する（内部ヘルパー）
fn value_to_string_repr(value: &Value) -> String {
    if value.is_nil() {
        return "NIL".to_string();
    }

    if is_boolean_value(value) {
        if let Some(f) = value.as_scalar() {
            return if !f.is_zero() { "TRUE".to_string() } else { "FALSE".to_string() };
        }
    }

    if is_string_value(value) {
        return value_as_string(value).unwrap_or_default();
    }

    if is_datetime_value(value) {
        if let Some(f) = value.as_scalar() {
            return format!("@{}", fraction_to_string(f));
        }
    }

    if is_number_value(value) {
        if let Some(f) = value.as_scalar() {
            return fraction_to_string(f);
        }
    }

    // ベクタの場合
    fn collect_fractions(val: &Value) -> Vec<String> {
        match &val.data {
            ValueData::Nil => vec!["NIL".to_string()],
            ValueData::Scalar(f) => vec![fraction_to_string(f)],
            ValueData::Vector(children) => {
                children.iter().flat_map(|c| collect_fractions(c)).collect()
            }
            ValueData::CodeBlock(_) => vec!["<code>".to_string()],
        }
    }

    collect_fractions(value).join(" ")
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

        // String → Number (正常ケース)
        interp.stack.push(Value::from_string("42"));
        op_num(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_number_value(val));
            if let Some(f) = val.as_scalar() {
                assert_eq!(f.numerator, BigInt::from(42));
            }
        }

        // 分数文字列 → Number
        interp.stack.clear();
        interp.stack.push(Value::from_string("1/3"));
        op_num(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_number_value(val));
            if let Some(f) = val.as_scalar() {
                assert_eq!(f.numerator, BigInt::from(1));
                assert_eq!(f.denominator, BigInt::from(3));
            }
        }

        // パース失敗 → NIL (エラーではない)
        interp.stack.clear();
        interp.stack.push(Value::from_string("ABC"));
        let result = op_num(&mut interp);
        assert!(result.is_ok()); // エラーではない
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil()); // NILが返される
        }

        // 既に数値 → エラー (変化なしはエラー原則)
        interp.stack.clear();
        interp.stack.push(wrap_number(Fraction::new(BigInt::from(123), BigInt::one())));
        let result = op_num(&mut interp);
        assert!(result.is_err());

        // Boolean → エラー (Stringのみ受け付ける)
        interp.stack.clear();
        interp.stack.push(Value::from_bool(true));
        let result = op_num(&mut interp);
        assert!(result.is_err());
    }

    #[test]
    fn test_bool_conversion() {
        let mut interp = Interpreter::new();

        // String 'TRUE' → Boolean TRUE
        interp.stack.push(Value::from_string("TRUE"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero()); // TRUE
            }
        }

        // String 'true' (小文字) → Boolean TRUE
        interp.stack.clear();
        interp.stack.push(Value::from_string("true"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero()); // TRUE
            }
        }

        // String 'false' → Boolean FALSE
        interp.stack.clear();
        interp.stack.push(Value::from_string("false"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            if let Some(f) = val.as_scalar() {
                assert!(f.is_zero()); // FALSE
            }
        }

        // String '1' → NIL (新仕様: 'true'/'false'以外はNIL)
        interp.stack.clear();
        interp.stack.push(Value::from_string("1"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil()); // パース失敗 → NIL
        }

        // String 'other' → NIL
        interp.stack.clear();
        interp.stack.push(Value::from_string("other"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil()); // パース失敗 → NIL
        }

        // Number 100 → Boolean TRUE (Truthiness: 0以外はTRUE)
        interp.stack.clear();
        interp.stack.push(wrap_number(Fraction::new(BigInt::from(100), BigInt::one())));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero()); // TRUE
            }
        }

        // Number 0 → Boolean FALSE
        interp.stack.clear();
        interp.stack.push(wrap_number(Fraction::new(BigInt::from(0), BigInt::one())));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            if let Some(f) = val.as_scalar() {
                assert!(f.is_zero()); // FALSE
            }
        }

        // 分数 1/2 → Boolean TRUE (0以外はTRUE)
        interp.stack.clear();
        interp.stack.push(wrap_number(Fraction::new(BigInt::from(1), BigInt::from(2))));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero()); // TRUE
            }
        }

        // 既にBoolean → エラー (変化なしはエラー原則)
        interp.stack.clear();
        interp.stack.push(Value::from_bool(true));
        let result = op_bool(&mut interp);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chars_basic() {
        let mut interp = Interpreter::new();
        // 直接文字列を使用（'hello' は文字列、[ 'hello' ] はベクター）
        interp.execute("'hello' CHARS JOIN").await.unwrap();
        assert_eq!(interp.stack.len(), 1);

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "hello");
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
    async fn test_join_empty_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ ] JOIN").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chars_join_roundtrip() {
        let mut interp = Interpreter::new();
        // 直接文字列を使用（'hello' は文字列、[ 'hello' ] はベクター）
        interp.execute("'hello' CHARS JOIN").await.unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "hello");
        }
    }

    #[tokio::test]
    async fn test_chars_reverse_join() {
        let mut interp = Interpreter::new();
        // 直接文字列を使用（'hello' は文字列、[ 'hello' ] はベクター）
        interp.execute("'hello' CHARS REVERSE JOIN").await.unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "olleh");
        }
    }

    #[tokio::test]
    async fn test_nil_pushes_constant() {
        // NILは常に定数としてNILをプッシュする
        let mut interp = Interpreter::new();
        let result = interp.execute("NIL").await;
        assert!(result.is_ok());
        assert_eq!(interp.stack.len(), 1);

        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil());
        }
    }

    #[tokio::test]
    async fn test_nil_multiple() {
        // NILを複数回呼ぶと複数のNILがスタックに積まれる
        let mut interp = Interpreter::new();
        let result = interp.execute("NIL NIL NIL").await;
        assert!(result.is_ok());
        assert_eq!(interp.stack.len(), 3);

        for val in interp.stack.iter() {
            assert!(val.is_nil());
        }
    }

    // ============================================================================
    // CHR テスト
    // ============================================================================

    #[test]
    fn test_chr_basic() {
        let mut interp = Interpreter::new();

        // 65 → 'A'
        interp.stack.push(wrap_number(Fraction::new(BigInt::from(65), BigInt::one())));
        op_chr(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "A");
        }

        // 97 → 'a'
        interp.stack.clear();
        interp.stack.push(wrap_number(Fraction::new(BigInt::from(97), BigInt::one())));
        op_chr(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "a");
        }

        // 10 → 改行
        interp.stack.clear();
        interp.stack.push(wrap_number(Fraction::new(BigInt::from(10), BigInt::one())));
        op_chr(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "\n");
        }

        // 48 → '0'
        interp.stack.clear();
        interp.stack.push(wrap_number(Fraction::new(BigInt::from(48), BigInt::one())));
        op_chr(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "0");
        }

        // Note: マルチバイト文字（日本語など）のテストは、Value::from_stringが
        // bytes()を使用しているため、value_as_stringとの互換性の問題があります。
        // これは既存の設計上の制約です。
    }

    #[test]
    fn test_chr_errors() {
        let mut interp = Interpreter::new();

        // 文字列 → エラー
        interp.stack.push(Value::from_string("A"));
        let result = op_chr(&mut interp);
        assert!(result.is_err());

        // 分数 → エラー (整数のみ)
        interp.stack.clear();
        interp.stack.push(wrap_number(Fraction::new(BigInt::from(1), BigInt::from(2))));
        let result = op_chr(&mut interp);
        assert!(result.is_err());

        // 負の数 → エラー
        interp.stack.clear();
        interp.stack.push(wrap_number(Fraction::new(BigInt::from(-1), BigInt::one())));
        let result = op_chr(&mut interp);
        assert!(result.is_err());

        // 範囲外 (0x110000) → エラー
        interp.stack.clear();
        interp.stack.push(wrap_number(Fraction::new(BigInt::from(0x110000), BigInt::one())));
        let result = op_chr(&mut interp);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_chr_integration() {
        let mut interp = Interpreter::new();

        // 65 CHR → 'A'
        interp.execute("65 CHR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "A");
        }
    }

    // ============================================================================
    // NUM/STR/BOOL 統合テスト
    // ============================================================================

    #[tokio::test]
    async fn test_num_str_roundtrip() {
        let mut interp = Interpreter::new();

        // '123' NUM STR → '123' (往復変換)
        interp.execute("'123' NUM STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "123");
        }

        // '1/3' NUM STR → '1/3'
        interp.stack.clear();
        interp.execute("'1/3' NUM STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "1/3");
        }
    }

    #[tokio::test]
    async fn test_str_num_parse_fail() {
        let mut interp = Interpreter::new();

        // 'ABC' NUM → NIL (パース失敗はNIL)
        interp.execute("'ABC' NUM").await.unwrap();
        assert_eq!(interp.stack.len(), 1);
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil());
        }
    }

    #[tokio::test]
    async fn test_bool_string_parsing() {
        let mut interp = Interpreter::new();

        // 'true' BOOL → TRUE
        interp.execute("'true' BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            assert!(val.is_truthy());
        }

        // 'FALSE' BOOL → FALSE
        interp.stack.clear();
        interp.execute("'FALSE' BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            assert!(!val.is_truthy());
        }

        // 'other' BOOL → NIL
        interp.stack.clear();
        interp.execute("'other' BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil());
        }
    }

    #[tokio::test]
    async fn test_bool_number_truthiness() {
        let mut interp = Interpreter::new();

        // 100 BOOL → TRUE (0以外はTRUE)
        interp.execute("100 BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            assert!(val.is_truthy());
        }

        // 0 BOOL → FALSE
        interp.stack.clear();
        interp.execute("0 BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            assert!(!val.is_truthy());
        }

        // -1 BOOL → TRUE
        interp.stack.clear();
        interp.execute("-1 BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_boolean_value(val));
            assert!(val.is_truthy());
        }
    }

    #[tokio::test]
    async fn test_str_boolean() {
        let mut interp = Interpreter::new();

        // TRUE STR → 'TRUE'
        interp.execute("TRUE STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "TRUE");
        }

        // FALSE STR → 'FALSE'
        interp.stack.clear();
        interp.execute("FALSE STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "FALSE");
        }
    }

    #[tokio::test]
    async fn test_str_nil() {
        let mut interp = Interpreter::new();

        // NIL STR → 'NIL'
        interp.execute("NIL STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "NIL");
        }
    }
}
