// rust/src/interpreter/control.rs
//
// 【責務】
// 制御フロー操作（TIMES、WAIT）を実装する。
// カスタムワードの繰り返し実行や遅延実行をサポートする。

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::interpreter::helpers::{get_integer_from_value, get_word_name_from_value};
use crate::types::ValueType;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};
use gloo_timers::future::sleep;
use std::time::Duration;

/// TIMES - ワードをN回繰り返し実行する
///
/// 【責務】
/// - 指定されたカスタムワードを指定回数繰り返し実行
/// - ビルトインワードには使用不可（カスタムワードのみ）
///
/// 【使用法】
/// - `'MYWORD' [5] TIMES` → MYWORDを5回実行
///
/// 【引数スタック】
/// - [count]: 実行回数（単一要素ベクタの整数）
/// - ['name']: ワード名（単一要素ベクタの文字列）
///
/// 【戻り値スタック】
/// - なし（ワードの実行結果がスタックに残る）
///
/// 【エラー】
/// - ワードが存在しない場合
/// - ビルトインワードを指定した場合
/// - カウントが整数でない場合
pub(crate) fn execute_times(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::from("TIMES requires word name and count. Usage: 'WORD' [ n ] TIMES"));
    }

    let count_val = interp.stack.pop().unwrap();
    let name_val = interp.stack.pop().unwrap();

    let count = get_integer_from_value(&count_val)?;
    let word_name = get_word_name_from_value(&name_val)?;

    if let Some(def) = interp.dictionary.get(&word_name) {
        if def.is_builtin {
            return Err(AjisaiError::from("TIMES can only be used with custom words"));
        }
    } else {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    for _ in 0..count {
        interp.execute_word_sync(&word_name)?;
    }

    Ok(())
}

/// WAIT - 遅延後にワードを実行する
///
/// 【責務】
/// - 指定ミリ秒後にカスタムワードを実行
/// - ビルトインワードには使用不可（カスタムワードのみ）
///
/// 【使用法】
/// - `'MYWORD' [1000] WAIT` → 1000ms後にMYWORDを実行
///
/// 【引数スタック】
/// - [delay_ms]: 遅延時間（ミリ秒、単一要素ベクタの整数）
/// - ['name']: ワード名（単一要素ベクタの文字列）
///
/// 【戻り値スタック】
/// - なし（ワードの実行結果がスタックに残る）
///
/// 【エラー】
/// - ワードが存在しない場合
/// - ビルトインワードを指定した場合
/// - 遅延時間が整数でない場合
///
/// 【注意事項】
/// - async関数として実装されており、指定時間だけ実行を遅延させる
pub(crate) async fn execute_wait(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::from("WAIT requires word name and delay. Usage: 'WORD' [ ms ] WAIT"));
    }

    let delay_val = interp.stack.pop().unwrap();
    let name_val = interp.stack.pop().unwrap();

    let delay_ms = match &delay_val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => {
            match &v[0].val_type {
                ValueType::Number(n) if n.denominator == BigInt::one() => {
                    n.numerator.to_u64().ok_or_else(|| AjisaiError::from("Delay too large"))?
                },
                _ => return Err(AjisaiError::type_error("integer", "other type")),
            }
        },
        _ => return Err(AjisaiError::type_error("single-element vector with integer", "other type")),
    };

    let word_name = get_word_name_from_value(&name_val)?;

    if let Some(def) = interp.dictionary.get(&word_name) {
        if def.is_builtin {
            return Err(AjisaiError::from("WAIT can only be used with custom words"));
        }
    } else {
        return Err(AjisaiError::UnknownWord(word_name));
    }

    // 指定時間だけ遅延
    sleep(Duration::from_millis(delay_ms)).await;

    // ワードを実行
    interp.execute_word_sync(&word_name)?;

    Ok(())
}
