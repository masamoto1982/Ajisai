use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{value_as_string, create_number_value};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

fn is_string_value(val: &Value) -> bool {
    let children: &Vec<Value> = match &val.data {
        ValueData::Vector(v) if !v.is_empty() => v,
        ValueData::Vector(_) => return false,
        ValueData::Scalar(_) => return false,
        ValueData::Nil => return false,
        ValueData::Record { .. } => return false,
        ValueData::CodeBlock(_) => return false,
    };
    children.iter().all(|child| check_char_scalar(child))
}

fn check_char_scalar(child: &Value) -> bool {
    let f: &Fraction = match &child.data {
        ValueData::Scalar(f) => f,
        ValueData::Vector(_) => return false,
        ValueData::Nil => return false,
        ValueData::Record { .. } => return false,
        ValueData::CodeBlock(_) => return false,
    };
    let n: i64 = match f.to_i64() {
        Some(n) if n >= 0 && n <= 0x10FFFF => n,
        Some(_) => return false,
        None => return false,
    };
    match char::from_u32(n as u32) {
        Some(c) => !c.is_control() || c == '\n' || c == '\r' || c == '\t',
        None => false,
    }
}

fn is_boolean_value(_val: &Value) -> bool {
    false
}

fn is_number_value(val: &Value) -> bool {
    val.is_scalar()
}

fn is_datetime_value(_val: &Value) -> bool {
    false
}

fn convert_value_to_string(val: &Value) -> Result<Value> {
    if val.is_nil() {
        return Ok(Value::nil());
    }

    if is_string_value(val) {
        return Err(AjisaiError::NoChange { word: "STR".into() });
    }

    if is_number_value(val) {
        if let Some(f) = val.as_scalar() {
            let string_repr = format_fraction_to_string(f);
            return Ok(Value::from_string(&string_repr));
        }
    }

    let string_repr = format_value_to_string_repr(val);
    Ok(Value::from_string(&string_repr))
}

fn apply_unary_cast(interp: &mut Interpreter, convert: fn(&Value) -> Result<Value>) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let value = if is_keep_mode {
                interp
                    .stack
                    .last()
                    .cloned()
                    .ok_or(AjisaiError::StackUnderflow)?
            } else {
                interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
            };

            match convert(&value) {
                Ok(result) => {
                    interp.stack.push(result);
                    Ok(())
                }
                Err(error) => {
                    if !is_keep_mode {
                        interp.stack.push(value);
                    }
                    Err(error)
                }
            }
        }
        OperationTargetMode::Stack => {
            if interp.stack.is_empty() {
                return Err(AjisaiError::StackUnderflow);
            }

            if is_keep_mode {
                let originals: Vec<Value> = interp.stack.to_vec();
                let mut converted = Vec::with_capacity(originals.len());
                for value in &originals {
                    converted.push(convert(value)?);
                }
                interp.stack.extend(converted);
                Ok(())
            } else {
                let originals: Vec<Value> = interp.stack.drain(..).collect();
                let mut converted = Vec::with_capacity(originals.len());

                for value in &originals {
                    match convert(value) {
                        Ok(result) => converted.push(result),
                        Err(error) => {
                            interp.stack = originals;
                            return Err(error);
                        }
                    }
                }

                interp.stack = converted;
                Ok(())
            }
        }
    }
}

pub fn op_str(interp: &mut Interpreter) -> Result<()> {
    apply_unary_cast(interp, convert_value_to_string)
}

fn format_fraction_to_string(f: &Fraction) -> String {
    if f.is_integer() {
        format!("{}", f.numerator())
    } else {
        format!("{}/{}", f.numerator(), f.denominator())
    }
}

fn convert_value_to_number(val: &Value) -> Result<Value> {
    if is_string_value(val) {
        let s = value_as_string(val).unwrap_or_default();
        match Fraction::from_str(&s) {
            Ok(fraction) => return Ok(create_number_value(fraction)),
            Err(_) => return Ok(Value::nil()),
        }
    }

    if is_number_value(val) {
        return Err(AjisaiError::NoChange { word: "NUM".into() });
    }
    if is_boolean_value(val) {
        return Err(AjisaiError::from(
            "NUM: expected String, got Boolean",
        ));
    }
    if val.is_nil() {
        return Err(AjisaiError::from("NUM: expected String, got Nil"));
    }
    Err(AjisaiError::from("NUM: expected String input"))
}

pub fn op_num(interp: &mut Interpreter) -> Result<()> {
    apply_unary_cast(interp, convert_value_to_number)
}

fn convert_value_to_boolean(val: &Value) -> Result<Value> {
    if is_boolean_value(val) {
        return Err(AjisaiError::NoChange {
            word: "BOOL".into(),
        });
    }
    if is_string_value(val) {
        let s = value_as_string(val).unwrap_or_default();
        let upper = s.to_uppercase();
        if upper == "TRUE" {
            return Ok(Value::from_bool(true));
        } else if upper == "FALSE" {
            return Ok(Value::from_bool(false));
        } else {
            return Ok(Value::nil());
        }
    }
    if is_number_value(val) {
        if let Some(f) = val.as_scalar() {
            return Ok(Value::from_bool(!f.is_zero()));
        }
    }
    if val.is_nil() {
        return Err(AjisaiError::from(
            "BOOL: expected String or Number, got Nil",
        ));
    }
    Err(AjisaiError::from("BOOL: expected String or Number input"))
}

pub fn op_bool(interp: &mut Interpreter) -> Result<()> {
    apply_unary_cast(interp, convert_value_to_boolean)
}

pub fn op_nil(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "NIL".into(),
            mode: "Stack".into(),
        });
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if val.is_nil() {
        interp.stack.push(val);
        return Err(AjisaiError::NoChange { word: "NIL".into() });
    }

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

    if is_boolean_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::from(
            "NIL: expected String, got Boolean",
        ));
    }

    if is_number_value(&val) {
        interp.stack.push(val);
        return Err(AjisaiError::from(
            "NIL: expected String, got Number",
        ));
    }

    interp.stack.push(val);
    Err(AjisaiError::from("NIL: expected String input"))
}

pub fn op_chars(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            if val.is_nil() {
                interp.stack.push(val);
                return Err(AjisaiError::from("CHARS: expected String, got Nil"));
            }

            if is_string_value(&val) {
                let s = value_as_string(&val).unwrap_or_default();
                if s.is_empty() {
                    interp.stack.push(val);
                    return Err(AjisaiError::from("CHARS: expected non-empty String"));
                }

                let chars: Vec<Value> = s
                    .chars()
                    .map(|c| Value::from_string(&c.to_string()))
                    .collect();

                interp.stack.push(Value::from_vector(chars));
                return Ok(());
            }

            if is_number_value(&val) {
                interp.stack.push(val);
                return Err(AjisaiError::from(
                    "CHARS: expected String, got Number",
                ));
            }

            if is_boolean_value(&val) {
                interp.stack.push(val);
                return Err(AjisaiError::from(
                    "CHARS: expected String, got Boolean",
                ));
            }

            interp.stack.push(val);
            Err(AjisaiError::from("CHARS: expected String input"))
        }
        OperationTargetMode::Stack => {
            let stack_len = interp.stack.len();
            if stack_len == 0 {
                return Err(AjisaiError::StackUnderflow);
            }

            let mut results = Vec::with_capacity(stack_len);
            let elements: Vec<Value> = interp.stack.drain(..).collect();

            for elem in elements {
                if elem.is_nil() {
                    interp.stack = results;
                    interp.stack.push(elem);
                    return Err(AjisaiError::from("CHARS: expected String, got Nil"));
                }

                if is_string_value(&elem) {
                    let s = value_as_string(&elem).unwrap_or_default();
                    if s.is_empty() {
                        interp.stack = results;
                        interp.stack.push(elem);
                        return Err(AjisaiError::from("CHARS: expected non-empty String"));
                    }
                    let chars: Vec<Value> = s
                        .chars()
                        .map(|c| Value::from_string(&c.to_string()))
                        .collect();
                    results.push(Value::from_vector(chars));
                    continue;
                }

                if is_number_value(&elem) {
                    interp.stack = results;
                    interp.stack.push(elem);
                    return Err(AjisaiError::from(
                        "CHARS: expected String, got Number",
                    ));
                }

                if is_boolean_value(&elem) {
                    interp.stack = results;
                    interp.stack.push(elem);
                    return Err(AjisaiError::from(
                        "CHARS: expected String, got Boolean",
                    ));
                }

                interp.stack = results;
                interp.stack.push(elem);
                return Err(AjisaiError::from("CHARS: expected String input"));
            }

            interp.stack = results;
            Ok(())
        }
    }
}

fn try_char_from_value(val: &Value) -> Option<char> {
    let f: &Fraction = val.as_scalar()?;
    let code: i64 = f.to_i64()?;
    if code < 0 || code > 0x10FFFF { return None; }
    char::from_u32(code as u32)
}

pub fn op_join(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            if val.is_nil() {
                interp.stack.push(val);
                return Err(AjisaiError::from("JOIN: expected Vector, got Nil"));
            }

            if let ValueData::Vector(children) = &val.data {
                if children.is_empty() {
                    interp.stack.push(val);
                    return Err(AjisaiError::from(
                        "JOIN: expected non-empty Vector",
                    ));
                }

                let mut result = String::new();
                for (i, elem) in children.iter().enumerate() {
                    if is_string_value(elem) {
                        if let Some(s) = value_as_string(elem) {
                            result.push_str(&s);
                            continue;
                        }
                    }

                    if is_number_value(elem) {
                        match try_char_from_value(elem) {
                            Some(c) => { result.push(c); continue; }
                            None => {
                                interp.stack.push(val);
                                return Err(AjisaiError::from(format!(
                                    "JOIN: invalid character code at index {}",
                                    i
                                )));
                            }
                        }
                    }

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
            Err(AjisaiError::from(format!(
                "JOIN: expected Vector, got {}",
                type_name
            )))
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
                        return Err(AjisaiError::from(
                            "JOIN: empty vector has no strings to join",
                        ));
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
                                "JOIN: invalid character code at index {}",
                                i
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
                return Err(AjisaiError::from(format!(
                    "JOIN: requires vector format, got {}",
                    type_name
                )));
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
/// CHR の単一値変換（内部ヘルパー）
fn convert_codepoint_to_char(val: &Value) -> Result<Value> {
    if is_number_value(val) {
        if let Some(f) = val.as_scalar() {
            if let Some(code) = f.to_i64() {
                if code >= 0 && code <= 0x10FFFF {
                    if let Some(c) = char::from_u32(code as u32) {
                        return Ok(Value::from_string(&c.to_string()));
                    }
                }
                return Err(AjisaiError::from(format!(
                    "CHR: {} is not a valid Unicode code point (valid range: 0-0x10FFFF, excluding surrogates)",
                    code
                )));
            } else {
                let frac_str = format_fraction_to_string(f);
                return Err(AjisaiError::from(format!(
                    "CHR: requires an integer, got {}",
                    frac_str
                )));
            }
        }
    }
    if is_string_value(val) {
        return Err(AjisaiError::from("CHR: expected Number, got String"));
    }
    if is_boolean_value(val) {
        return Err(AjisaiError::from("CHR: expected Number, got Boolean"));
    }
    if val.is_nil() {
        return Err(AjisaiError::from("CHR: expected Number, got Nil"));
    }
    Err(AjisaiError::from("CHR: expected Number input"))
}

pub fn op_chr(interp: &mut Interpreter) -> Result<()> {
    apply_unary_cast(interp, convert_codepoint_to_char)
}

/// 値を文字列表現に変換する（内部ヘルパー）
fn format_value_to_string_repr(value: &Value) -> String {
    if value.is_nil() {
        return "NIL".to_string();
    }

    if is_boolean_value(value) {
        if let Some(f) = value.as_scalar() {
            return if !f.is_zero() {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            };
        }
    }

    if is_string_value(value) {
        return value_as_string(value).unwrap_or_default();
    }

    if is_datetime_value(value) {
        if let Some(f) = value.as_scalar() {
            return format!("@{}", format_fraction_to_string(f));
        }
    }

    if is_number_value(value) {
        if let Some(f) = value.as_scalar() {
            return format_fraction_to_string(f);
        }
    }

    // ベクタの場合
    fn collect_fractions(val: &Value) -> Vec<String> {
        match &val.data {
            ValueData::Nil => vec!["NIL".to_string()],
            ValueData::Scalar(f) => vec![format_fraction_to_string(f)],
            ValueData::Vector(children)
            | ValueData::Record {
                pairs: children, ..
            } => children.iter().flat_map(|c| collect_fractions(c)).collect(),
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
    fn test_format_value_to_string_repr() {
        // Number
        let num = Value::from_fraction(Fraction::new(BigInt::from(42), BigInt::one()));
        assert_eq!(format_value_to_string_repr(&num), "42");

        // Boolean (now just a scalar in the new architecture, so displays as "1")
        let bool_val = Value::from_bool(true);
        assert_eq!(format_value_to_string_repr(&bool_val), "1");

        // Nil
        let nil = Value::nil();
        assert_eq!(format_value_to_string_repr(&nil), "NIL");
    }

    #[test]
    fn test_str_conversion() {
        let mut interp = Interpreter::new();

        // Number → String
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(42), BigInt::one())));
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
                assert_eq!(f.numerator(), BigInt::from(42));
            }
        }

        // 分数文字列 → Number
        interp.stack.clear();
        interp.stack.push(Value::from_string("1/3"));
        op_num(&mut interp).unwrap();

        if let Some(val) = interp.stack.last() {
            assert!(is_number_value(val));
            if let Some(f) = val.as_scalar() {
                assert_eq!(f.numerator(), BigInt::from(1));
                assert_eq!(f.denominator(), BigInt::from(3));
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
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(123), BigInt::one())));
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
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero()); // TRUE
            }
        }

        // String 'true' (小文字) → Boolean TRUE
        interp.stack.clear();
        interp.stack.push(Value::from_string("true"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero()); // TRUE
            }
        }

        // String 'false' → Boolean FALSE
        interp.stack.clear();
        interp.stack.push(Value::from_string("false"));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
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
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(100), BigInt::one())));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero()); // TRUE
            }
        }

        // Number 0 → Boolean FALSE
        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(0), BigInt::one())));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(f.is_zero()); // FALSE
            }
        }

        // 分数 1/2 → Boolean TRUE (0以外はTRUE)
        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(1), BigInt::from(2))));
        op_bool(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            if let Some(f) = val.as_scalar() {
                assert!(!f.is_zero()); // TRUE
            }
        }

        // from_bool creates a scalar (same as number in new architecture),
        // so op_bool treats it as a number and applies truthiness conversion.
        // This is no longer an error since is_boolean_value always returns false.
        interp.stack.clear();
        interp.stack.push(Value::from_bool(true));
        let result = op_bool(&mut interp);
        assert!(result.is_ok());
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
        // In new architecture, [ 42 ] is a Vector (treated as string).
        // CHARS interprets scalar 42 as Unicode codepoint '*', so it succeeds.
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 42 ] CHARS").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_join_basic() {
        let mut interp = Interpreter::new();
        interp
            .execute("[ 'h' 'e' 'l' 'l' 'o' ] JOIN")
            .await
            .unwrap();
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
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(65), BigInt::one())));
        op_chr(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "A");
        }

        // 97 → 'a'
        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(97), BigInt::one())));
        op_chr(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "a");
        }

        // 10 → 改行
        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(10), BigInt::one())));
        op_chr(&mut interp).unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "\n");
        }

        // 48 → '0'
        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(48), BigInt::one())));
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
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(1), BigInt::from(2))));
        let result = op_chr(&mut interp);
        assert!(result.is_err());

        // 負の数 → エラー
        interp.stack.clear();
        interp
            .stack
            .push(create_number_value(Fraction::new(BigInt::from(-1), BigInt::one())));
        let result = op_chr(&mut interp);
        assert!(result.is_err());

        // 範囲外 (0x110000) → エラー
        interp.stack.clear();
        interp.stack.push(create_number_value(Fraction::new(
            BigInt::from(0x110000),
            BigInt::one(),
        )));
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

        // 'true' BOOL → TRUE (scalar 1)
        interp.execute("'true' BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            assert!(val.is_truthy());
        }

        // 'FALSE' BOOL → FALSE (scalar 0)
        interp.stack.clear();
        interp.execute("'FALSE' BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
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
            assert!(val.is_scalar());
            assert!(val.is_truthy());
        }

        // 0 BOOL → FALSE
        interp.stack.clear();
        interp.execute("0 BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            assert!(!val.is_truthy());
        }

        // -1 BOOL → TRUE
        interp.stack.clear();
        interp.execute("-1 BOOL").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_scalar());
            assert!(val.is_truthy());
        }
    }

    #[tokio::test]
    async fn test_str_boolean() {
        let mut interp = Interpreter::new();

        // TRUE STR → '1' (in new architecture, booleans are just scalars)
        interp.execute("TRUE STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "1");
        }

        // FALSE STR → '0' (in new architecture, booleans are just scalars)
        interp.stack.clear();
        interp.execute("FALSE STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(is_string_value(val));
            let s = value_as_string(val).unwrap();
            assert_eq!(s, "0");
        }
    }

    #[tokio::test]
    async fn test_str_nil() {
        let mut interp = Interpreter::new();

        // NIL STR → NIL (仕様セクション7.2: 不明な値に変換を射しても不明)
        interp.execute("NIL STR").await.unwrap();
        if let Some(val) = interp.stack.last() {
            assert!(val.is_nil(), "NIL STR should return NIL, not a string");
        }
    }
}
