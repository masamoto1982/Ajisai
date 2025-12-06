// rust/src/interpreter/datetime.rs
//
// 【責務】
// 日付時刻変換ワード群を実装する。
// NOW: 現在のUnixタイムスタンプを取得
// DATETIME: タイムスタンプ（数値） → Vector（日付時刻）
// TIMESTAMP: Vector（日付時刻） → タイムスタンプ（数値）
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
// ### 背景：タイムスタンプとタイムゾーンの関係性
//
// タイムスタンプ（instant）は、ある特定の瞬間を表す数値であり、地球上の
// すべての地点で同一の値を持つ。例えば Unix時刻 1732531200 は、東京でも
// ニューヨークでも同じ瞬間を指す。
//
// 一方、この瞬間を人間が理解できる形式（年月日時分秒）で表現する際には、
// どのタイムゾーンで表現するかによって異なる値になる：
//   - UTC: 2024-11-25 05:00:00
//   - Asia/Tokyo (UTC+9): 2024-11-25 14:00:00
//   - America/New_York (UTC-5): 2024-11-25 00:00:00
//
// ### アプローチの比較
//
// **アプローチA: タイムゾーン情報をデータ型に含める**
//   例: Java 8の ZonedDateTime, OffsetDateTime
//   特性:
//     - 各値がタイムゾーン情報を保持する
//     - 同じ瞬間が複数の異なるオブジェクトとして表現可能
//     - 例: ZonedDateTime("2024-11-25T14:00:00+09:00[Asia/Tokyo]")
//          ZonedDateTime("2024-11-25T05:00:00Z[UTC]")
//          → 両者は同じ瞬間だが異なるオブジェクト
//   問題点:
//     - 同一性の判定が複雑化（同じ瞬間なのに異なる表現）
//     - タイムゾーン情報の伝播により、データ構造が肥大化
//     - 瞬間（instant）の概念とタイムゾーンという表現方法が混在
//   参考: https://qiita.com/twrcd1227/items/21864c0e7c8abc4c3ae4
//   （記事では「瞬間は地球全体で普遍的であり、東京時間・ソウル時間という
//     概念は存在しない」という指摘がある）
//
// **アプローチB: タイムゾーンを変換時のパラメータとして扱う**
//   例: Ajisaiの設計、一部のデータベースシステム
//   特性:
//     - タイムスタンプは常に単一の数値（Unix時刻など）
//     - タイムゾーンは、表示・入力時の変換パラメータとして指定
//     - 例: timestamp = 1732531200
//          to_datetime(timestamp, 'Asia/Tokyo') → [2024 11 25 14 0 0]
//          to_datetime(timestamp, 'UTC') → [2024 11 25 5 0 0]
//   利点:
//     - 瞬間の概念とタイムゾーンの概念が分離され、責務が明確
//     - データ構造がシンプル（数値のみ）
//     - 同一性の判定が容易（数値の比較のみ）
//
// ### Ajisaiの設計選択
//
// AjisaiではアプローチBを採用：
//   - タイムスタンプは常に単一の数値（Unix時刻）として保持
//   - DATETIME/TIMESTAMPワードは変換パラメータとしてタイムゾーンを要求
//   - データとしてタイムゾーン情報を保持しない
//
// この設計により、「瞬間」という普遍的な概念と「表現方法」という
// ローカルな概念を明確に分離する。
//
// ## 原則2: タイムゾーン指定を必須とする
//
// ### 背景：オプショナル引数の問題
//
// タイムゾーン変換において、タイムゾーン指定を省略可能にするかどうかは
// 重要な設計判断である。
//
// ### アプローチの比較
//
// **アプローチA: タイムゾーン指定を省略可能にする**
//   例: BigQuery SQLの DATETIME(), TIMESTAMP() 関数
//   特性:
//     - タイムゾーンを指定しない場合、デフォルトタイムゾーンを使用
//     - 例: DATETIME(timestamp) → セッションのタイムゾーンを使用
//          DATETIME(timestamp, 'Asia/Tokyo') → 明示的に指定
//   問題点:
//     - デフォルト値への依存により、「どのタイムゾーンか」の意識が薄れる
//     - 実行環境の設定に依存し、移植性が低下
//     - コードレビュー時に意図が不明確（省略は意図的か、単なる見落としか）
//   参考: https://zenn.dev/su_k/articles/69c62aa7fb70c3
//   （記事では「省略可能なタイムゾーンパラメータがタイムゾーン意識を
//     弱めてしまう」という課題が指摘されている）
//
// **アプローチB: タイムゾーン指定を必須にする**
//   例: Ajisaiの設計
//   特性:
//     - すべての変換でタイムゾーンを明示的に指定
//     - 例: [ timestamp ] 'LOCAL' DATETIME
//          [ timestamp ] 'UTC' DATETIME （将来サポート予定）
//     - 省略時はエラー
//   利点:
//     - 開発者が常に「どのタイムゾーンか」を意識する
//     - コードの意図が明確
//     - 実行環境に依存せず、動作が予測可能
//
// ### Ajisaiの設計選択
//
// AjisaiではアプローチBを採用：
//   - DATETIME/TIMESTAMPワードは必ずタイムゾーン文字列を要求
//   - タイムゾーンを省略するとスタックアンダーフローエラー
//   - 毎回の変換で明示的にタイムゾーンを指定することを強制
//
// この設計により、タイムゾーン関連のバグを設計段階で防止する。
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
    use crate::types::tensor::Tensor;

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

    // 数値結果はTensorとして返す
    let tensor = Tensor::vector(vec![timestamp]);
    interp.stack.push(Value::from_tensor(tensor));

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
        ValueType::Vector(v) if v.len() == 1 => {
            match &v[0].val_type {
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
        ValueType::Vector(v) if v.len() == 1 => {
            match &v[0].val_type {
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

    // Tensorを構築（全て数値なので）
    use crate::types::tensor::Tensor;
    let mut fractions = vec![
        Fraction::new(BigInt::from(year), BigInt::one()),
        Fraction::new(BigInt::from(month), BigInt::one()),
        Fraction::new(BigInt::from(day), BigInt::one()),
        Fraction::new(BigInt::from(hour), BigInt::one()),
        Fraction::new(BigInt::from(minute), BigInt::one()),
        Fraction::new(BigInt::from(second), BigInt::one()),
    ];

    // サブ秒精度がある場合は追加
    if let Some(subsec) = subsec_fraction {
        fractions.push(subsec);
    }

    // 日付時刻成分は全て数値なのでTensorとして返す
    let tensor = Tensor::vector(fractions);
    interp.stack.push(Value::from_tensor(tensor));

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
        ValueType::Vector(v) if v.len() == 1 => {
            match &v[0].val_type {
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
        ValueType::Vector(v) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Vector(inner) => inner.clone(),
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

    // 数値結果はTensorとして返す
    use crate::types::tensor::Tensor;
    let tensor = Tensor::vector(vec![timestamp]);
    interp.stack.push(Value::from_tensor(tensor));

    Ok(())
}
