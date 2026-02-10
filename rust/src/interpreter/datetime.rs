// rust/src/interpreter/datetime.rs
//
// 【責務】
// 日付時刻変換ワード群を実装する。
// NOW: 現在のUnixタイムスタンプを取得
// DATETIME: タイムスタンプ（数値） → Vector（日付時刻）
// TIMESTAMP: Vector（日付時刻） → タイムスタンプ（数値）
//
// 統一Value宇宙アーキテクチャ版
//
// ============================================================================
// 【設計思想】タイムゾーン処理における設計選択
// ============================================================================
//
// この実装は、他の言語・システムにおけるタイムゾーン処理の設計を分析し、
// それぞれのアプローチの特性を踏まえて設計されている。将来的な拡張を考慮し、
// 以下の設計原則を厳守すること。
//
// ## 原則1: タイムゾーンをデータ型ではなく変換パラメータとして扱う
//
// AjisaiではアプローチBを採用：
//   - タイムスタンプは常に単一の数値（Unix時刻）として保持
//   - DATETIME/TIMESTAMPワードは変換パラメータとしてタイムゾーンを要求
//   - データとしてタイムゾーン情報を保持しない
//
// ## 原則2: タイムゾーン指定を必須とする
//
// AjisaiではアプローチBを採用：
//   - DATETIME/TIMESTAMPワードは必ずタイムゾーン文字列を要求
//   - タイムゾーンを省略するとスタックアンダーフローエラー
//   - 毎回の変換で明示的にタイムゾーンを指定することを強制
//
// ============================================================================
// 【設計原則】実装詳細
// ============================================================================
//
// - タイムゾーンはブラウザのローカルタイムゾーンを使用（将来拡張予定）
// - タイムスタンプはUnix時刻（1970-01-01 00:00:00 UTCからの秒数）
// - Vectorフォーマット: [年 月 日 時 分 秒] または [年 月 日 時 分 秒 サブ秒]
// - 実在しない日時（2023-13-32など）はエラー
// - 分数システムと親和性を保つ（サブ秒精度を分数で表現可能）

use crate::interpreter::{Interpreter, OperationTargetMode};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::wrap_datetime;
use crate::types::{Value, ValueData, DisplayHint};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{ToPrimitive, One, Zero};
use wasm_bindgen::prelude::*;

// ============================================================================
// ヘルパー関数（統一Value宇宙アーキテクチャ用）
// ============================================================================

/// ベクタ値かどうかを判定
fn is_vector_value(val: &Value) -> bool {
    matches!(&val.data, ValueData::Vector(_))
}

/// 文字列値かどうかを判定
fn is_string_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::String && !val.is_nil()
}

/// 数値値かどうかを判定
fn is_number_value(val: &Value) -> bool {
    matches!(val.display_hint, DisplayHint::Number | DisplayHint::Auto | DisplayHint::DateTime) && val.is_scalar()
}

/// Valueから文字列を取得
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

/// ベクタの子要素を取得
fn get_vector_children(val: &Value) -> Option<&Vec<Value>> {
    if let ValueData::Vector(children) = &val.data {
        Some(children)
    } else {
        None
    }
}

/// NOW - 現在のUnixタイムスタンプを取得
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Date, js_name = now)]
    fn date_now() -> f64;
}

pub fn op_now(interp: &mut Interpreter) -> Result<()> {
    // NOWはStackモードをサポートしない（日付時刻ワード）
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported { word: "NOW".into(), mode: "Stack".into() });
    }

    // JavaScriptのDate.now()を呼び出し（ミリ秒単位）
    let now_ms = date_now();

    // ミリ秒を秒に変換して分数として表現
    let ms_bigint = BigInt::from(now_ms as i64);
    let thousand = BigInt::from(1000);

    let timestamp = Fraction::new(ms_bigint, thousand);

    // DateTime結果を単一要素Vectorとして返す
    interp.stack.push(wrap_datetime(timestamp));

    Ok(())
}

/// DATETIME - タイムスタンプをローカル日付時刻Vectorに変換
#[wasm_bindgen]
extern "C" {
    type Date;

    #[wasm_bindgen(constructor)]
    fn new_with_millis(millis: f64) -> Date;

    #[wasm_bindgen(method, getter, js_name = getFullYear)]
    fn get_full_year(this: &Date) -> i32;

    #[wasm_bindgen(method, getter, js_name = getMonth)]
    fn get_month(this: &Date) -> i32;

    #[wasm_bindgen(method, getter, js_name = getDate)]
    fn get_date(this: &Date) -> i32;

    #[wasm_bindgen(method, getter, js_name = getHours)]
    fn get_hours(this: &Date) -> i32;

    #[wasm_bindgen(method, getter, js_name = getMinutes)]
    fn get_minutes(this: &Date) -> i32;

    #[wasm_bindgen(method, getter, js_name = getSeconds)]
    fn get_seconds(this: &Date) -> i32;

    #[wasm_bindgen(method, getter, js_name = getMilliseconds)]
    fn get_milliseconds(this: &Date) -> i32;
}

pub fn op_datetime(interp: &mut Interpreter) -> Result<()> {
    // DATETIMEはStackモードをサポートしない（日付時刻ワード）
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported { word: "DATETIME".into(), mode: "Stack".into() });
    }

    // タイムゾーン文字列を取得
    let tz_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let timezone = if is_vector_value(&tz_val) {
        if let Some(children) = get_vector_children(&tz_val) {
            if children.len() == 1 && is_string_value(&children[0]) {
                value_as_string(&children[0]).unwrap_or_default()
            } else {
                interp.stack.push(tz_val);
                return Err(AjisaiError::from("DATETIME: timezone must be a String (e.g., 'LOCAL')"));
            }
        } else {
            interp.stack.push(tz_val);
            return Err(AjisaiError::from("DATETIME: timezone must be a String (e.g., 'LOCAL')"));
        }
    } else {
        interp.stack.push(tz_val);
        return Err(AjisaiError::from("DATETIME: timezone must be a String (e.g., 'LOCAL')"));
    };

    // タイムゾーンの検証（現在はLOCALのみサポート）
    if timezone.to_uppercase() != "LOCAL" {
        interp.stack.push(tz_val);
        return Err(AjisaiError::from(
            format!("DATETIME: unsupported timezone '{}'. Currently only 'LOCAL' is supported", timezone)
        ));
    }

    // タイムスタンプを取得
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // Vectorから数値またはDateTime型を抽出
    let timestamp = if is_vector_value(&val) {
        if let Some(children) = get_vector_children(&val) {
            if children.len() == 1 && is_number_value(&children[0]) {
                if let Some(f) = children[0].as_scalar() {
                    f.clone()
                } else {
                    interp.stack.push(val);
                    interp.stack.push(tz_val);
                    return Err(AjisaiError::from("DATETIME: requires Number or DateTime type for timestamp"));
                }
            } else {
                interp.stack.push(val);
                interp.stack.push(tz_val);
                return Err(AjisaiError::from("DATETIME: requires Number or DateTime type for timestamp"));
            }
        } else {
            interp.stack.push(val);
            interp.stack.push(tz_val);
            return Err(AjisaiError::from("DATETIME: requires Number or DateTime type for timestamp"));
        }
    } else {
        interp.stack.push(val);
        interp.stack.push(tz_val);
        return Err(AjisaiError::from("DATETIME: requires Number or DateTime type for timestamp"));
    };

    // タイムスタンプを秒とサブ秒に分離
    let seconds = &timestamp.numerator / &timestamp.denominator;
    let remainder_numerator = &timestamp.numerator % &timestamp.denominator;
    let subsec_fraction = if !remainder_numerator.is_zero() {
        Some(Fraction::new(remainder_numerator, timestamp.denominator.clone()))
    } else {
        None
    };

    // 秒をミリ秒に変換してJavaScriptのDateオブジェクトを作成
    let seconds_f64 = seconds.to_f64().ok_or(AjisaiError::from("DATETIME: timestamp too large"))?;
    let millis = seconds_f64 * 1000.0;

    let date = Date::new_with_millis(millis);

    // 日付時刻成分を取得
    let year = date.get_full_year();
    let month = date.get_month() + 1; // JavaScriptは0-indexed
    let day = date.get_date();
    let hour = date.get_hours();
    let minute = date.get_minutes();
    let second = date.get_seconds();

    // Vectorを構築
    let mut values = vec![
        Value::from_number(Fraction::new(BigInt::from(year), BigInt::one())),
        Value::from_number(Fraction::new(BigInt::from(month), BigInt::one())),
        Value::from_number(Fraction::new(BigInt::from(day), BigInt::one())),
        Value::from_number(Fraction::new(BigInt::from(hour), BigInt::one())),
        Value::from_number(Fraction::new(BigInt::from(minute), BigInt::one())),
        Value::from_number(Fraction::new(BigInt::from(second), BigInt::one())),
    ];

    // サブ秒精度がある場合は追加
    if let Some(subsec) = subsec_fraction {
        values.push(Value::from_number(subsec));
    }

    // 日付時刻成分をVectorとして返す
    interp.stack.push(Value::from_vector(values));

    Ok(())
}

/// TIMESTAMP - ローカル日付時刻Vectorをタイムスタンプに変換
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(static_method_of = Date, js_name = UTC)]
    fn utc(year: i32, month: i32, day: i32, hour: i32, minute: i32, second: i32) -> f64;

    #[wasm_bindgen(constructor, js_name = Date)]
    fn new_date(year: i32, month: i32, day: i32, hour: i32, minute: i32, second: i32) -> Date;

    #[wasm_bindgen(method, js_name = getTime)]
    fn get_time(this: &Date) -> f64;

    #[wasm_bindgen(method, js_name = getTimezoneOffset)]
    fn get_timezone_offset(this: &Date) -> f64;
}

pub fn op_timestamp(interp: &mut Interpreter) -> Result<()> {
    // TIMESTAMPはStackモードをサポートしない（日付時刻ワード）
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported { word: "TIMESTAMP".into(), mode: "Stack".into() });
    }

    // タイムゾーン文字列を取得
    let tz_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let timezone = if is_vector_value(&tz_val) {
        if let Some(children) = get_vector_children(&tz_val) {
            if children.len() == 1 && is_string_value(&children[0]) {
                value_as_string(&children[0]).unwrap_or_default()
            } else {
                interp.stack.push(tz_val);
                return Err(AjisaiError::from("TIMESTAMP: timezone must be a String (e.g., 'LOCAL')"));
            }
        } else {
            interp.stack.push(tz_val);
            return Err(AjisaiError::from("TIMESTAMP: timezone must be a String (e.g., 'LOCAL')"));
        }
    } else {
        interp.stack.push(tz_val);
        return Err(AjisaiError::from("TIMESTAMP: timezone must be a String (e.g., 'LOCAL')"));
    };

    // タイムゾーンの検証（現在はLOCALのみサポート）
    if timezone.to_uppercase() != "LOCAL" {
        interp.stack.push(tz_val);
        return Err(AjisaiError::from(
            format!("TIMESTAMP: unsupported timezone '{}'. Currently only 'LOCAL' is supported", timezone)
        ));
    }

    // 日付時刻Vectorを取得
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // Vectorから日付時刻成分を抽出（[[year month day hour minute second]]形式）
    let components = if is_vector_value(&val) {
        if let Some(children) = get_vector_children(&val) {
            if children.len() == 1 && is_vector_value(&children[0]) {
                if let Some(inner_children) = get_vector_children(&children[0]) {
                    inner_children.clone()
                } else {
                    interp.stack.push(val);
                    interp.stack.push(tz_val);
                    return Err(AjisaiError::from("TIMESTAMP: requires Vector type for datetime"));
                }
            } else {
                interp.stack.push(val);
                interp.stack.push(tz_val);
                return Err(AjisaiError::from("TIMESTAMP: requires Vector type for datetime"));
            }
        } else {
            interp.stack.push(val);
            interp.stack.push(tz_val);
            return Err(AjisaiError::from("TIMESTAMP: requires Vector type for datetime"));
        }
    } else {
        interp.stack.push(val);
        interp.stack.push(tz_val);
        return Err(AjisaiError::from("TIMESTAMP: requires Vector type for datetime"));
    };

    // 要素数チェック（6または7）
    if components.len() != 6 && components.len() != 7 {
        interp.stack.push(val);
        interp.stack.push(tz_val);
        return Err(AjisaiError::from(
            "TIMESTAMP: Vector must have 6 or 7 elements [year month day hour minute second (subsec)]"
        ));
    }

    // 各成分を整数として抽出（サブ秒を除く）
    let mut integers = Vec::new();
    for (i, component) in components.iter().take(6).enumerate() {
        if is_number_value(component) {
            if let Some(frac) = component.as_scalar() {
                if frac.denominator != BigInt::one() {
                    interp.stack.push(val);
                    interp.stack.push(tz_val);
                    return Err(AjisaiError::from(
                        format!("TIMESTAMP: element {} must be an integer, got {}/{}",
                            i, frac.numerator, frac.denominator)
                    ));
                }
                let int_val = frac.numerator.to_i32().ok_or_else(|| {
                    AjisaiError::from(format!("TIMESTAMP: element {} too large", i))
                })?;
                integers.push(int_val);
            } else {
                interp.stack.push(val);
                interp.stack.push(tz_val);
                return Err(AjisaiError::from(
                    format!("TIMESTAMP: element {} must be a Number", i)
                ));
            }
        } else {
            interp.stack.push(val);
            interp.stack.push(tz_val);
            return Err(AjisaiError::from(
                format!("TIMESTAMP: element {} must be a Number", i)
            ));
        }
    }

    let year = integers[0];
    let month = integers[1];
    let day = integers[2];
    let hour = integers[3];
    let minute = integers[4];
    let second = integers[5];

    // サブ秒成分を抽出（あれば）
    let subsec = if components.len() == 7 {
        if is_number_value(&components[6]) {
            if let Some(f) = components[6].as_scalar() {
                Some(f.clone())
            } else {
                interp.stack.push(val);
                interp.stack.push(tz_val);
                return Err(AjisaiError::from("TIMESTAMP: subsecond must be a Number"));
            }
        } else {
            interp.stack.push(val);
            interp.stack.push(tz_val);
            return Err(AjisaiError::from("TIMESTAMP: subsecond must be a Number"));
        }
    } else {
        None
    };

    // 日付の妥当性チェック：JavaScriptのDateオブジェクトを作成して検証
    let date = Date::new_date(year, month - 1, day, hour, minute, second);

    // 作成したDateオブジェクトから各成分を取得して、入力と一致するか確認
    let created_year = date.get_full_year();
    let created_month = date.get_month() + 1;
    let created_day = date.get_date();
    let created_hour = date.get_hours();
    let created_minute = date.get_minutes();
    let created_second = date.get_seconds();

    if created_year != year || created_month != month || created_day != day ||
       created_hour != hour || created_minute != minute || created_second != second {
        interp.stack.push(val);
        interp.stack.push(tz_val);
        return Err(AjisaiError::from(
            format!("TIMESTAMP: invalid date/time [{} {} {} {} {} {}]",
                year, month, day, hour, minute, second)
        ));
    }

    // タイムスタンプを計算（ミリ秒単位）
    let timestamp_ms = date.get_time();

    // ミリ秒を秒に変換（分数として）
    let ms_bigint = BigInt::from(timestamp_ms as i64);
    let thousand = BigInt::from(1000);
    let mut timestamp = Fraction::new(ms_bigint, thousand);

    // サブ秒を加算
    if let Some(subsec_frac) = subsec {
        timestamp = timestamp.add(&subsec_frac);
    }

    // DateTime結果を単一要素Vectorとして返す
    interp.stack.push(wrap_datetime(timestamp));

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_now_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        // Stackモード（..）でNOWを呼び出した場合はエラー
        let result = interp.execute(".. NOW").await;
        assert!(result.is_err(), "NOW should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("NOW") && err_msg.contains("Stack mode"),
                "Expected Stack mode error for NOW, got: {}", err_msg);
    }

    #[tokio::test]
    async fn test_datetime_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        // Stackモード（..）でDATETIMEを呼び出した場合はエラー
        let result = interp.execute("[ 1732531200 ] 'LOCAL' .. DATETIME").await;
        assert!(result.is_err(), "DATETIME should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("DATETIME") && err_msg.contains("Stack mode"),
                "Expected Stack mode error for DATETIME, got: {}", err_msg);
    }

    #[tokio::test]
    async fn test_timestamp_rejects_stack_mode() {
        let mut interp = Interpreter::new();

        // Stackモード（..）でTIMESTAMPを呼び出した場合はエラー
        let result = interp.execute("[ [ 2024 11 25 14 0 0 ] ] 'LOCAL' .. TIMESTAMP").await;
        assert!(result.is_err(), "TIMESTAMP should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("TIMESTAMP") && err_msg.contains("Stack mode"),
                "Expected Stack mode error for TIMESTAMP, got: {}", err_msg);
    }
}
