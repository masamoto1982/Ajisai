// rust/src/interpreter/io.rs
//
// 【責務】
// 入出力操作（PRINT）を実装する。
// スタックの値を出力バッファに書き込む機能を提供する。

use crate::interpreter::Interpreter;
use crate::error::{AjisaiError, Result};

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
    let val = interp.stack_pop().ok_or(AjisaiError::StackUnderflow)?;
    interp.output_buffer.push_str(&format!("{} ", val));
    Ok(())
}
