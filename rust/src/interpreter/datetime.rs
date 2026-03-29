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

use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{
    is_string_value, is_vector_value, value_as_string, create_datetime_value,
};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::Value;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive, Zero};
use wasm_bindgen::prelude::*;

fn parse_timezone_from_value(tz_val: &Value, word: &str) -> Result<String> {
    if !is_vector_value(tz_val) {
        return Err(AjisaiError::from(format!(
            "{}: timezone must be a String (e.g., 'LOCAL')",
            word
        )));
    }

    let Some(children) = tz_val.as_vector() else {
        return Err(AjisaiError::from(format!(
            "{}: timezone must be a String (e.g., 'LOCAL')",
            word
        )));
    };

    if children.len() != 1 || !is_string_value(&children[0]) {
        return Err(AjisaiError::from(format!(
            "{}: timezone must be a String (e.g., 'LOCAL')",
            word
        )));
    }

    let timezone = value_as_string(&children[0]).unwrap_or_default();
    if timezone.to_uppercase() != "LOCAL" {
        return Err(AjisaiError::from(format!(
            "{}: unsupported timezone '{}'. Currently only 'LOCAL' is supported",
            word, timezone
        )));
    }

    Ok(timezone)
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
        return Err(AjisaiError::ModeUnsupported {
            word: "NOW".into(),
            mode: "Stack".into(),
        });
    }

    // JavaScriptのDate.now()を呼び出し（ミリ秒単位）
    let now_ms = date_now();

    // ミリ秒を秒に変換して分数として表現
    let ms_bigint = BigInt::from(now_ms as i64);
    let thousand = BigInt::from(1000);

    let timestamp = Fraction::new(ms_bigint, thousand);

    // DateTime結果を単一要素Vectorとして返す
    interp.stack.push(create_datetime_value(timestamp));

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
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "DATETIME".into(),
            mode: "Stack".into(),
        });
    }

    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let (tz_val, val) = if is_keep_mode {
        if interp.stack.len() < 2 {
            return Err(AjisaiError::StackUnderflow);
        }
        let len = interp.stack.len();
        (interp.stack[len - 1].clone(), interp.stack[len - 2].clone())
    } else {
        let tz_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        (tz_val, val)
    };

    let timezone = match parse_timezone_from_value(&tz_val, "DATETIME") {
        Ok(tz) => tz,
        Err(e) => {
            if !is_keep_mode {
                interp.stack.push(val);
                interp.stack.push(tz_val);
            }
            return Err(e);
        }
    };

    let timestamp = if val.is_scalar() {
        if let Some(f) = val.as_scalar() {
            f.clone()
        } else {
            if !is_keep_mode {
                interp.stack.push(val);
                interp.stack.push(tz_val);
            }
            return Err(AjisaiError::from(
                "DATETIME: requires Number or DateTime type for timestamp",
            ));
        }
    } else if is_vector_value(&val) {
        if let Some(children) = val.as_vector() {
            if children.len() == 1 && children[0].is_scalar() {
                if let Some(f) = children[0].as_scalar() {
                    f.clone()
                } else {
                    if !is_keep_mode {
                        interp.stack.push(val);
                        interp.stack.push(tz_val);
                    }
                    return Err(AjisaiError::from(
                        "DATETIME: requires Number or DateTime type for timestamp",
                    ));
                }
            } else {
                if !is_keep_mode {
                    interp.stack.push(val);
                    interp.stack.push(tz_val);
                }
                return Err(AjisaiError::from(
                    "DATETIME: requires Number or DateTime type for timestamp",
                ));
            }
        } else {
            if !is_keep_mode {
                interp.stack.push(val);
                interp.stack.push(tz_val);
            }
            return Err(AjisaiError::from(
                "DATETIME: requires Number or DateTime type for timestamp",
            ));
        }
    } else {
        if !is_keep_mode {
            interp.stack.push(val);
            interp.stack.push(tz_val);
        }
        return Err(AjisaiError::from(
            "DATETIME: requires Number or DateTime type for timestamp",
        ));
    };

    if timezone.to_uppercase() != "LOCAL" {
        if !is_keep_mode {
            interp.stack.push(val);
            interp.stack.push(tz_val);
        }
        return Err(AjisaiError::from(format!(
            "DATETIME: unsupported timezone '{}'. Currently only 'LOCAL' is supported",
            timezone
        )));
    }

    let (ts_num, ts_den) = timestamp.to_bigint_pair();
    let seconds = &ts_num / &ts_den;
    let remainder_numerator = &ts_num % &ts_den;
    let subsec_fraction = if !remainder_numerator.is_zero() {
        Some(Fraction::new(remainder_numerator, ts_den))
    } else {
        None
    };

    let seconds_f64 = seconds
        .to_f64()
        .ok_or(AjisaiError::from("DATETIME: timestamp too large"))?;
    let millis = seconds_f64 * 1000.0;

    let date = Date::new_with_millis(millis);

    let year = date.get_full_year();
    let month = date.get_month() + 1;
    let day = date.get_date();
    let hour = date.get_hours();
    let minute = date.get_minutes();
    let second = date.get_seconds();

    let mut values = vec![
        Value::from_number(Fraction::from(year as i64)),
        Value::from_number(Fraction::from(month as i64)),
        Value::from_number(Fraction::from(day as i64)),
        Value::from_number(Fraction::from(hour as i64)),
        Value::from_number(Fraction::from(minute as i64)),
        Value::from_number(Fraction::from(second as i64)),
    ];

    if let Some(subsec) = subsec_fraction {
        values.push(Value::from_number(subsec));
    }

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
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "TIMESTAMP".into(),
            mode: "Stack".into(),
        });
    }

    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;
    let (tz_val, val) = if is_keep_mode {
        if interp.stack.len() < 2 {
            return Err(AjisaiError::StackUnderflow);
        }
        let len = interp.stack.len();
        (interp.stack[len - 1].clone(), interp.stack[len - 2].clone())
    } else {
        let tz_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        (tz_val, val)
    };

    if let Err(e) = parse_timezone_from_value(&tz_val, "TIMESTAMP") {
        if !is_keep_mode {
            interp.stack.push(val);
            interp.stack.push(tz_val);
        }
        return Err(e);
    }

    let components = if is_vector_value(&val) {
        if let Some(children) = val.as_vector() {
            if children.len() == 1 && is_vector_value(&children[0]) {
                if let Some(inner_children) = children[0].as_vector() {
                    inner_children.clone()
                } else {
                    if !is_keep_mode {
                        interp.stack.push(val);
                        interp.stack.push(tz_val);
                    }
                    return Err(AjisaiError::from(
                        "TIMESTAMP: requires Vector type for datetime",
                    ));
                }
            } else {
                if !is_keep_mode {
                    interp.stack.push(val);
                    interp.stack.push(tz_val);
                }
                return Err(AjisaiError::from(
                    "TIMESTAMP: requires Vector type for datetime",
                ));
            }
        } else {
            if !is_keep_mode {
                interp.stack.push(val);
                interp.stack.push(tz_val);
            }
            return Err(AjisaiError::from(
                "TIMESTAMP: requires Vector type for datetime",
            ));
        }
    } else {
        if !is_keep_mode {
            interp.stack.push(val);
            interp.stack.push(tz_val);
        }
        return Err(AjisaiError::from(
            "TIMESTAMP: requires Vector type for datetime",
        ));
    };

    if components.len() != 6 && components.len() != 7 {
        if !is_keep_mode {
            interp.stack.push(val);
            interp.stack.push(tz_val);
        }
        return Err(AjisaiError::from(
            "TIMESTAMP: Vector must have 6 or 7 elements [year month day hour minute second (subsec)]",
        ));
    }

    let mut integers = Vec::new();
    for (i, component) in components.iter().take(6).enumerate() {
        if component.is_scalar() {
            if let Some(frac) = component.as_scalar() {
                if !frac.is_integer() {
                    if !is_keep_mode {
                        interp.stack.push(val);
                        interp.stack.push(tz_val);
                    }
                    return Err(AjisaiError::from(format!(
                        "TIMESTAMP: element {} must be an integer, got {}/{}",
                        i, frac.numerator(), frac.denominator()
                    )));
                }
                let int_val = frac.numerator().to_i32().ok_or_else(|| {
                    AjisaiError::from(format!("TIMESTAMP: element {} too large", i))
                })?;
                integers.push(int_val);
            } else {
                if !is_keep_mode {
                    interp.stack.push(val);
                    interp.stack.push(tz_val);
                }
                return Err(AjisaiError::from(format!(
                    "TIMESTAMP: element {} must be a Number",
                    i
                )));
            }
        } else {
            if !is_keep_mode {
                interp.stack.push(val);
                interp.stack.push(tz_val);
            }
            return Err(AjisaiError::from(format!(
                "TIMESTAMP: element {} must be a Number",
                i
            )));
        }
    }

    let year = integers[0];
    let month = integers[1];
    let day = integers[2];
    let hour = integers[3];
    let minute = integers[4];
    let second = integers[5];

    let subsec = if components.len() == 7 {
        if components[6].is_scalar() {
            if let Some(f) = components[6].as_scalar() {
                Some(f.clone())
            } else {
                if !is_keep_mode {
                    interp.stack.push(val);
                    interp.stack.push(tz_val);
                }
                return Err(AjisaiError::from("TIMESTAMP: subsecond must be a Number"));
            }
        } else {
            if !is_keep_mode {
                interp.stack.push(val);
                interp.stack.push(tz_val);
            }
            return Err(AjisaiError::from("TIMESTAMP: subsecond must be a Number"));
        }
    } else {
        None
    };

    let date = Date::new_date(year, month - 1, day, hour, minute, second);

    let created_year = date.get_full_year();
    let created_month = date.get_month() + 1;
    let created_day = date.get_date();
    let created_hour = date.get_hours();
    let created_minute = date.get_minutes();
    let created_second = date.get_seconds();

    if created_year != year
        || created_month != month
        || created_day != day
        || created_hour != hour
        || created_minute != minute
        || created_second != second
    {
        if !is_keep_mode {
            interp.stack.push(val);
            interp.stack.push(tz_val);
        }
        return Err(AjisaiError::from(format!(
            "TIMESTAMP: invalid date/time [{} {} {} {} {} {}]",
            year, month, day, hour, minute, second
        )));
    }

    let timestamp_ms = date.get_time();
    let ms_bigint = BigInt::from(timestamp_ms as i64);
    let thousand = BigInt::from(1000);
    let mut timestamp = Fraction::new(ms_bigint, thousand);

    if let Some(subsec_frac) = subsec {
        timestamp = timestamp.add(&subsec_frac);
    }

    interp.stack.push(create_datetime_value(timestamp));

    Ok(())
}

