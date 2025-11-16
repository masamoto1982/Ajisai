// rust/src/interpreter/io.rs
//
// 【責務】
// 入出力操作（PRINT、CR、SPACE、SPACES、EMIT）を実装する。
// スタックの値を出力バッファに書き込む機能を提供する。

use crate::interpreter::{Interpreter};
use crate::error::{AjisaiError, Result};
use crate::types::ValueType;
use num_bigint::BigInt;
use num_traits::{Zero, One, ToPrimitive};

/// PRINT - スタックトップの値を出力する
///
/// 【責務】
/// - スタックトップの値をポップして出力バッファに追加
/// - 値の後にスペースを追加
///
/// 【使用法】
/// - `[42] PRINT` → "42 " を出力
/// - `['hello'] PRINT` → "'hello' " を出力
///
/// 【引数スタック】
/// - value: 出力する値（任意の型）
///
/// 【戻り値スタック】
/// - なし（値は消費される）
///
/// 【エラー】
/// - スタックが空の場合
pub fn op_print(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    interp.output_buffer.push_str(&format!("{} ", val));
    Ok(())
}

/// CR - 改行を出力する
///
/// 【責務】
/// - 出力バッファに改行文字を追加
///
/// 【使用法】
/// - `CR` → 改行を出力
///
/// 【引数スタック】
/// - なし
///
/// 【戻り値スタック】
/// - なし
///
/// 【エラー】
/// - なし
pub fn op_cr(interp: &mut Interpreter) -> Result<()> {
    interp.output_buffer.push('\n');
    Ok(())
}

/// SPACE - スペースを出力する
///
/// 【責務】
/// - 出力バッファにスペース1文字を追加
///
/// 【使用法】
/// - `SPACE` → " " を出力
///
/// 【引数スタック】
/// - なし
///
/// 【戻り値スタック】
/// - なし
///
/// 【エラー】
/// - なし
pub fn op_space(interp: &mut Interpreter) -> Result<()> {
    interp.output_buffer.push(' ');
    Ok(())
}

/// SPACES - 指定数のスペースを出力する
///
/// 【責務】
/// - 出力バッファに指定された数のスペースを追加
/// - カウントは非負の整数である必要がある
///
/// 【使用法】
/// - `[5] SPACES` → "     " (5個のスペース) を出力
/// - `[0] SPACES` → "" (何も出力しない)
///
/// 【引数スタック】
/// - [count]: スペースの数（単一要素ベクタの非負整数）
///
/// 【戻り値スタック】
/// - なし
///
/// 【エラー】
/// - カウントが非負整数でない場合
/// - スタックが空の場合
pub fn op_spaces(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    match val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => match &v[0].val_type {
            ValueType::Number(n) => {
                if n.denominator == BigInt::one() && n.numerator >= BigInt::zero() {
                    if let Some(count) = n.numerator.to_usize() {
                        interp.output_buffer.push_str(&" ".repeat(count));
                        return Ok(());
                    }
                }
                Err(AjisaiError::from("SPACES requires a non-negative integer"))
            },
            _ => Err(AjisaiError::type_error("number", "other type")),
        },
        _ => Err(AjisaiError::type_error("single-element vector with number", "other type")),
    }
}

/// EMIT - ASCII文字を出力する
///
/// 【責務】
/// - 0-255の整数をASCII文字として出力バッファに追加
/// - 文字コードを文字に変換して出力
///
/// 【使用法】
/// - `[65] EMIT` → "A" を出力
/// - `[72] EMIT [69] EMIT [76] EMIT [76] EMIT [79] EMIT` → "HELLO" を出力
///
/// 【引数スタック】
/// - [code]: ASCII文字コード（単一要素ベクタの0-255の整数）
///
/// 【戻り値スタック】
/// - なし
///
/// 【エラー】
/// - 文字コードが0-255の範囲外の場合
/// - 整数でない場合
/// - スタックが空の場合
pub fn op_emit(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    match val.val_type {
        ValueType::Vector(v, _) if v.len() == 1 => match &v[0].val_type {
            ValueType::Number(n) => {
                if n.denominator == BigInt::one() && n.numerator >= BigInt::zero() && n.numerator <= BigInt::from(255) {
                    if let Some(byte) = n.numerator.to_u8() {
                        interp.output_buffer.push(byte as char);
                        return Ok(());
                    }
                }
                Err(AjisaiError::from("EMIT requires an integer between 0 and 255"))
            },
            _ => Err(AjisaiError::type_error("number", "other type")),
        },
        _ => Err(AjisaiError::type_error("single-element vector with number", "other type")),
    }
}
