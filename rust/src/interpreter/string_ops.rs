// rust/src/interpreter/string_ops.rs
//
// 文字列操作（内部的にはすべて分数Vector操作）
//
// 統一分数アーキテクチャでは、文字列は分数のベクタとして表現される。
// このモジュールは文字列っぽい操作を提供するが、
// 内部的にはすべて分数ベクタ操作である。

use crate::interpreter::Interpreter;
use crate::error::{AjisaiError, Result};
use crate::types::{Value, DisplayHint};
use crate::types::fraction::Fraction;

/// CONCAT - 連結（文字列でも配列でも同じ）
pub fn op_concat(interp: &mut Interpreter) -> Result<()> {
    let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let mut result = a_val.data.clone();
    result.extend(b_val.data.iter().cloned());

    // ヒントは引き継ぐ（両方 String なら String）
    let hint = if a_val.display_hint == DisplayHint::String
                && b_val.display_hint == DisplayHint::String {
        DisplayHint::String
    } else {
        DisplayHint::Auto
    };

    interp.stack.push(Value { data: result, display_hint: hint });
    Ok(())
}

/// UPPER - 大文字変換
pub fn op_upper(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let result: Vec<Fraction> = val.data.iter().map(|f| {
        if let Some(n) = f.to_i64() {
            // 小文字 a-z (97-122) → 大文字 A-Z (65-90)
            if n >= 97 && n <= 122 {
                Fraction::from(n - 32)
            } else {
                f.clone()
            }
        } else {
            f.clone()
        }
    }).collect();

    interp.stack.push(Value {
        data: result,
        display_hint: DisplayHint::String,
    });
    Ok(())
}

/// LOWER - 小文字変換
pub fn op_lower(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let result: Vec<Fraction> = val.data.iter().map(|f| {
        if let Some(n) = f.to_i64() {
            // 大文字 A-Z (65-90) → 小文字 a-z (97-122)
            if n >= 65 && n <= 90 {
                Fraction::from(n + 32)
            } else {
                f.clone()
            }
        } else {
            f.clone()
        }
    }).collect();

    interp.stack.push(Value {
        data: result,
        display_hint: DisplayHint::String,
    });
    Ok(())
}

/// AS-STR - 文字列として表示するヒントを設定
pub fn op_as_str(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    interp.stack.push(val.with_hint(DisplayHint::String));
    Ok(())
}

/// AS-NUM - 数値として表示するヒントを設定
pub fn op_as_num(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    interp.stack.push(val.with_hint(DisplayHint::Number));
    Ok(())
}

/// AS-BOOL - 真偽値として表示するヒントを設定
pub fn op_as_bool(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    interp.stack.push(val.with_hint(DisplayHint::Boolean));
    Ok(())
}

/// CHARS - 文字列を文字コードのベクタに変換（ヒントをNumberに設定）
pub fn op_chars(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    // データはそのまま、ヒントだけ変更
    interp.stack.push(val.with_hint(DisplayHint::Number));
    Ok(())
}

/// JOIN - 文字コードのベクタを文字列として解釈（ヒントをStringに設定）
pub fn op_join(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    // データはそのまま、ヒントだけ変更
    interp.stack.push(val.with_hint(DisplayHint::String));
    Ok(())
}
