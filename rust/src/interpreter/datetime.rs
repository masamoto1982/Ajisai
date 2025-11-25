// rust/src/interpreter/datetime.rs
//
// 【責務】
// 日付時刻変換ワード群を実装する。
// NOW: 現在のUnixタイムスタンプを取得
// DATETIME: タイムスタンプ（数値） → Vector（日付時刻）
// TIMESTAMP: Vector（日付時刻） → タイムスタンプ（数値）
//
// ============================================================================
// 【設計思想】タイムゾーン処理の理想的な設計
// ============================================================================
//
// この実装は、BigQuery SQLおよびJava 8のタイムゾーン設計における問題点を
// 解消することを目的として設計されている。将来的な拡張を考慮し、以下の
// 設計原則を厳守すること。
//
// ## 原則1: タイムゾーンは「変換パラメータ」であり「データ型の一部」ではない
//
// タイムスタンプ（instant）は地球全体で普遍的な一瞬を表す。これは東京時間でも
// ソウル時間でもない。タイムゾーンは、その普遍的な瞬間を人間が理解できる
// 形式（年月日時分秒）に変換する際の「変換パラメータ」である。
//
// 【悪い例】Java 8のZonedDateTime/OffsetDateTime
//   - タイムゾーン情報をinstantに紐付けてしまう設計的欠陥
//   - 同じ瞬間が複数の異なる表現を持つことになり混乱を招く
//   - 参考: https://qiita.com/twrcd1227/items/21864c0e7c8abc4c3ae4
//
// 【良い例】Ajisaiの設計
//   - タイムスタンプは常に単一の数値（Unix時刻）
//   - タイムゾーンはDATETIME/TIMESTAMPワードの変換パラメータとして指定
//   - データとしてはタイムゾーン情報を保持しない
//
// ## 原則2: タイムゾーン指定を「必須」とすることで意識を強制する
//
// タイムゾーン指定を省略可能にすると、開発者が「どのタイムゾーンか？」を
// 考えずにコードを書いてしまい、タイムゾーン関連のバグが発生する。
//
// 【悪い例】BigQuery SQLのDATETIME/TIMESTAMP関数
//   - タイムゾーン指定が省略可能（オプショナル）
//   - デフォルトタイムゾーンに依存してしまい、意識が薄れる
//   - 参考: https://zenn.dev/su_k/articles/69c62aa7fb70c3
//
// 【良い例】Ajisaiの設計
//   - DATETIME/TIMESTAMPワードは必ずタイムゾーン文字列を要求
//   - タイムゾーンを省略するとエラーになる
//   - 毎回「どのタイムゾーンか？」を明示的に考えることを強制
//
// ## 将来の拡張方針
//
// 現在はブラウザのローカルタイムゾーン ('LOCAL') のみをサポートしているが、
// 将来的には以下のような拡張を想定している：
//
// 1. UTCタイムゾーンのサポート
//    例: [ 1732531200 ] 'UTC' DATETIME
//
// 2. IANA タイムゾーンデータベースのサポート
//    例: [ 1732531200 ] 'Asia/Tokyo' DATETIME
//        [ 1732531200 ] 'America/New_York' DATETIME
//
// 3. オフセット指定のサポート
//    例: [ 1732531200 ] '+09:00' DATETIME
//        [ 1732531200 ] '-05:00' DATETIME
//
// これらの拡張を行う際も、タイムゾーンを必須とする設計原則は堅持すること。
// タイムゾーン文字列のパース処理を拡張するだけで、基本的なアーキテクチャは
// 変更不要である。
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

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::wrap_in_square_vector;
use crate::types::{Value, ValueType};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_traits::{ToPrimitive, One, Zero};
use wasm_bindgen::prelude::*;

/// NOW - 現在のUnixタイムスタンプを取得
///
/// 【責務】
/// - 現在時刻をUnixタイムスタンプ（秒単位の分数）として返す
/// - ミリ秒精度で取得し、分数として表現
///
/// 【使用法】
/// ```ajisai
/// NOW → [ 1732531200 1/2 + ]  // 1732531200.5秒（ミリ秒精度）
/// ```
///
/// 【戻り値】
/// - Unixタイムスタンプ（分数）
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Date, js_name = now)]
    fn date_now() -> f64;
}

pub fn op_now(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("NOW only supports StackTop mode"));
    }

    // JavaScriptのDate.now()を呼び出し（ミリ秒単位）
    let now_ms = date_now();

    // ミリ秒を秒に変換して分数として表現
    // now_ms / 1000 = now_ms / 1000 (分数として)
    let ms_bigint = BigInt::from(now_ms as i64);
    let thousand = BigInt::from(1000);

    let timestamp = Fraction::new(ms_bigint, thousand);

    interp.stack.push(wrap_in_square_vector(
        Value { val_type: ValueType::Number(timestamp) }
    ));

    Ok(())
}

/// DATETIME - タイムスタンプをローカル日付時刻Vectorに変換
///
/// 【責務】
/// - Unixタイムスタンプ → 指定タイムゾーンの日付時刻Vector
/// - サブ秒精度がある場合は7番目の要素として含める
/// - タイムゾーン指定を必須とし、意識を強制する
///
/// 【使用法】
/// ```ajisai
/// [ 1732531200 ] 'LOCAL' DATETIME → [ [ 2024 11 25 14 0 0 ] ]
/// [ 1732531200 1/2 + ] 'LOCAL' DATETIME → [ [ 2024 11 25 14 0 0 1/2 ] ]
/// ```
///
/// 【引数】
/// - タイムスタンプ（数値、分数可）
/// - タイムゾーン（文字列）
///   - 'LOCAL': ブラウザのローカルタイムゾーン
///
/// 【戻り値】
/// - Vector: [年 月 日 時 分 秒] または [年 月 日 時 分 秒 サブ秒]
///
/// 【エラー】
/// - スタックが空
/// - 数値型でない
/// - タイムゾーン文字列が不正
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
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("DATETIME only supports StackTop mode"));
    }

    // タイムゾーン文字列を取得
    let tz_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let timezone = match &tz_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::String(s) => s.clone(),
                _ => {
                    interp.stack.push(tz_val);
                    return Err(AjisaiError::from("DATETIME: timezone must be a String (e.g., 'LOCAL')"));
                }
            }
        }
        ValueType::SingletonVector(boxed, _) => {
            match &boxed.val_type {
                ValueType::String(s) => s.clone(),
                _ => {
                    interp.stack.push(tz_val);
                    return Err(AjisaiError::from("DATETIME: timezone must be a String (e.g., 'LOCAL')"));
                }
            }
        }
        _ => {
            interp.stack.push(tz_val);
            return Err(AjisaiError::from("DATETIME: timezone must be a String (e.g., 'LOCAL')"));
        }
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

    // Vectorから数値を抽出
    let timestamp = match &val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) => n.clone(),
                _ => {
                    interp.stack.push(val);
                    interp.stack.push(tz_val);
                    return Err(AjisaiError::from("DATETIME: requires Number type for timestamp"));
                }
            }
        }
        ValueType::SingletonVector(boxed, _) => {
            match &boxed.val_type {
                ValueType::Number(n) => n.clone(),
                _ => {
                    interp.stack.push(val);
                    interp.stack.push(tz_val);
                    return Err(AjisaiError::from("DATETIME: requires Number type for timestamp"));
                }
            }
        }
        _ => {
            interp.stack.push(val);
            interp.stack.push(tz_val);
            return Err(AjisaiError::from("DATETIME: requires Number type for timestamp"));
        }
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
    let mut components = vec![
        Value { val_type: ValueType::Number(Fraction::new(BigInt::from(year), BigInt::one())) },
        Value { val_type: ValueType::Number(Fraction::new(BigInt::from(month), BigInt::one())) },
        Value { val_type: ValueType::Number(Fraction::new(BigInt::from(day), BigInt::one())) },
        Value { val_type: ValueType::Number(Fraction::new(BigInt::from(hour), BigInt::one())) },
        Value { val_type: ValueType::Number(Fraction::new(BigInt::from(minute), BigInt::one())) },
        Value { val_type: ValueType::Number(Fraction::new(BigInt::from(second), BigInt::one())) },
    ];

    // サブ秒精度がある場合は追加
    if let Some(subsec) = subsec_fraction {
        components.push(Value { val_type: ValueType::Number(subsec) });
    }

    let datetime_vec = Value {
        val_type: ValueType::Vector(components, crate::types::BracketType::Square)
    };

    interp.stack.push(wrap_in_square_vector(datetime_vec));

    Ok(())
}

/// TIMESTAMP - ローカル日付時刻Vectorをタイムスタンプに変換
///
/// 【責務】
/// - 指定タイムゾーンの日付時刻Vector → Unixタイムスタンプ
/// - 実在しない日時（2023-13-32など）はエラー
/// - サブ秒精度をサポート
/// - タイムゾーン指定を必須とし、意識を強制する
///
/// 【使用法】
/// ```ajisai
/// [ [ 2024 11 25 14 0 0 ] ] 'LOCAL' TIMESTAMP → [ 1732531200 ]
/// [ [ 2024 11 25 14 0 0 1/2 ] ] 'LOCAL' TIMESTAMP → [ 1732531200 1/2 + ]
/// [ [ 2023 13 32 0 0 0 ] ] 'LOCAL' TIMESTAMP → ERROR（実在しない日付）
/// ```
///
/// 【引数】
/// - Vector: [年 月 日 時 分 秒] または [年 月 日 時 分 秒 サブ秒]
/// - タイムゾーン（文字列）
///   - 'LOCAL': ブラウザのローカルタイムゾーン
///
/// 【戻り値】
/// - タイムスタンプ（数値、分数）
///
/// 【エラー】
/// - スタックが空
/// - Vector型でない
/// - 要素数が6または7でない
/// - 各要素が整数でない（サブ秒を除く）
/// - 実在しない日時
/// - タイムゾーン文字列が不正
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
    if interp.operation_target != OperationTarget::StackTop {
        return Err(AjisaiError::from("TIMESTAMP only supports StackTop mode"));
    }

    // タイムゾーン文字列を取得
    let tz_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let timezone = match &tz_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::String(s) => s.clone(),
                _ => {
                    interp.stack.push(tz_val);
                    return Err(AjisaiError::from("TIMESTAMP: timezone must be a String (e.g., 'LOCAL')"));
                }
            }
        }
        ValueType::SingletonVector(boxed, _) => {
            match &boxed.val_type {
                ValueType::String(s) => s.clone(),
                _ => {
                    interp.stack.push(tz_val);
                    return Err(AjisaiError::from("TIMESTAMP: timezone must be a String (e.g., 'LOCAL')"));
                }
            }
        }
        _ => {
            interp.stack.push(tz_val);
            return Err(AjisaiError::from("TIMESTAMP: timezone must be a String (e.g., 'LOCAL')"));
        }
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

    // Vectorから日付時刻成分を抽出
    let components = match &val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Vector(inner, _) => inner.clone(),
                ValueType::SingletonVector(boxed, _) => {
                    match &boxed.val_type {
                        ValueType::Vector(inner, _) => inner.clone(),
                        _ => {
                            interp.stack.push(val);
                            interp.stack.push(tz_val);
                            return Err(AjisaiError::from("TIMESTAMP: requires Vector type for datetime"));
                        }
                    }
                }
                _ => {
                    interp.stack.push(val);
                    interp.stack.push(tz_val);
                    return Err(AjisaiError::from("TIMESTAMP: requires Vector type for datetime"));
                }
            }
        }
        ValueType::SingletonVector(boxed, _) => {
            match &boxed.val_type {
                ValueType::Vector(inner, _) => inner.clone(),
                _ => {
                    interp.stack.push(val);
                    interp.stack.push(tz_val);
                    return Err(AjisaiError::from("TIMESTAMP: requires Vector type for datetime"));
                }
            }
        }
        _ => {
            interp.stack.push(val);
            interp.stack.push(tz_val);
            return Err(AjisaiError::from("TIMESTAMP: requires Vector type for datetime"));
        }
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
        match &component.val_type {
            ValueType::Number(n) => {
                if n.denominator != BigInt::one() {
                    interp.stack.push(val);
                    interp.stack.push(tz_val);
                    return Err(AjisaiError::from(
                        format!("TIMESTAMP: element {} must be an integer, got {}/{}",
                            i, n.numerator, n.denominator)
                    ));
                }
                let int_val = n.numerator.to_i32().ok_or_else(|| {
                    AjisaiError::from(format!("TIMESTAMP: element {} too large", i))
                })?;
                integers.push(int_val);
            }
            _ => {
                interp.stack.push(val);
                interp.stack.push(tz_val);
                return Err(AjisaiError::from(
                    format!("TIMESTAMP: element {} must be a Number", i)
                ));
            }
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
        match &components[6].val_type {
            ValueType::Number(n) => Some(n.clone()),
            _ => {
                interp.stack.push(val);
                interp.stack.push(tz_val);
                return Err(AjisaiError::from("TIMESTAMP: subsecond must be a Number"));
            }
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

    interp.stack.push(wrap_in_square_vector(
        Value { val_type: ValueType::Number(timestamp) }
    ));

    Ok(())
}
