// rust/src/interpreter/arithmetic.rs
//
// 統一分数アーキテクチャ版の算術演算
//
// 型チェックは存在しない。すべて分数演算として実行する。

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::get_integer_from_value;
use crate::types::{Value, DisplayHint};
use crate::types::fraction::Fraction;

// ============================================================================
// ブロードキャスト付き二項演算
// ============================================================================

/// ブロードキャスト付き二項演算
fn broadcast_binary_op<F>(
    a: &[Fraction],
    b: &[Fraction],
    op: F,
) -> Result<Vec<Fraction>>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction>,
{
    match (a.len(), b.len()) {
        // 両方空
        (0, 0) => Ok(Vec::new()),

        // 片方が空（NILとの演算）
        (0, _) | (_, 0) => Err(AjisaiError::from("Cannot operate with NIL")),

        // 両方スカラー
        (1, 1) => Ok(vec![op(&a[0], &b[0])?]),

        // aがスカラー、bがベクター（ブロードキャスト）
        (1, _) => b.iter().map(|bi| op(&a[0], bi)).collect(),

        // aがベクター、bがスカラー（ブロードキャスト）
        (_, 1) => a.iter().map(|ai| op(ai, &b[0])).collect(),

        // 両方同じ長さ
        (la, lb) if la == lb => {
            a.iter().zip(b.iter()).map(|(ai, bi)| op(ai, bi)).collect()
        }

        // 長さ不一致
        (la, lb) => Err(AjisaiError::VectorLengthMismatch { len1: la, len2: lb }),
    }
}

// ============================================================================
// 二項演算の汎用実装
// ============================================================================

/// 二項算術演算の汎用ハンドラ
fn binary_arithmetic_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction> + Copy,
{
    match interp.operation_target {
        // StackTopモード: ベクタ間の要素ごと演算
        OperationTarget::StackTop => {
            let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            let result_data = match broadcast_binary_op(&a_val.data, &b_val.data, op) {
                Ok(data) => data,
                Err(e) => {
                    interp.stack.push(a_val);
                    interp.stack.push(b_val);
                    return Err(e);
                }
            };

            // "No change is an error" 原則のチェック（REDUCE等では無効化）
            if !interp.disable_no_change_check && (result_data == a_val.data || result_data == b_val.data) {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::from("Arithmetic operation resulted in no change"));
            }

            interp.stack.push(Value {
                data: result_data,
                display_hint: DisplayHint::Auto,
            });
        },

        // Stackモード: N個の要素を畳み込む
        OperationTarget::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            // カウント0はエラー（"No change is an error"原則）
            if count == 0 {
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK operation with count 0 results in no change"));
            }

            // カウント1もエラー（1要素の畳み込みは変化なし）
            if count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK operation with count 1 results in no change"));
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = interp.stack.drain(interp.stack.len() - count ..).collect();

            // 各要素がスカラーであることを確認
            if items.iter().any(|v| v.data.len() != 1) {
                interp.stack.extend(items);
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK mode requires single-element values"));
            }

            let mut acc = items[0].data[0].clone();
            let original_first = acc.clone();

            for item in items.iter().skip(1) {
                acc = op(&acc, &item.data[0])?;
            }

            // "No change is an error" 原則のチェック（REDUCE等では無効化）
            if !interp.disable_no_change_check && acc == original_first {
                interp.stack.extend(items);
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK operation resulted in no change"));
            }

            interp.stack.push(Value::from_fraction(acc));
        }
    }
    Ok(())
}

// ============================================================================
// 算術演算の実装
// ============================================================================

/// + 演算子 - 加算（型チェックなし）
pub fn op_add(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| Ok(a.add(b)))
}

/// - 演算子 - 減算（型チェックなし）
pub fn op_sub(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| Ok(a.sub(b)))
}

/// * 演算子 - 乗算（型チェックなし）
pub fn op_mul(interp: &mut Interpreter) -> Result<()> {
    // StackTopモードでブロードキャストの場合、整数スカラー最適化を試みる
    if interp.operation_target == OperationTarget::StackTop {
        if let (Some(b_val), Some(a_val)) = (interp.stack.pop(), interp.stack.pop()) {
            let a_data = &a_val.data;
            let b_data = &b_val.data;

            // NILチェック
            if a_data.is_empty() || b_data.is_empty() {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::from("Cannot operate with NIL"));
            }

            let a_len = a_data.len();
            let b_len = b_data.len();
            let mut result_vec = Vec::with_capacity(a_len.max(b_len));

            // ブロードキャスト判定と整数スカラー最適化
            if a_len > 1 && b_len == 1 {
                // bがスカラー: 整数の場合は最適化
                let scalar = &b_data[0];
                if scalar.is_integer() {
                    for elem in a_data {
                        result_vec.push(elem.mul_by_integer(scalar));
                    }
                } else {
                    for elem in a_data {
                        result_vec.push(elem.mul(scalar));
                    }
                }
            } else if a_len == 1 && b_len > 1 {
                // aがスカラー: 整数の場合は最適化
                let scalar = &a_data[0];
                if scalar.is_integer() {
                    for elem in b_data {
                        result_vec.push(elem.mul_by_integer(scalar));
                    }
                } else {
                    for elem in b_data {
                        result_vec.push(scalar.mul(elem));
                    }
                }
            } else if a_len != b_len {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::VectorLengthMismatch { len1: a_len, len2: b_len });
            } else {
                // 要素ごと演算
                for (a, b) in a_data.iter().zip(b_data.iter()) {
                    result_vec.push(a.mul(b));
                }
            }

            // "No change is an error" 原則のチェック
            if !interp.disable_no_change_check && (result_vec == *a_data || result_vec == *b_data) {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::from("Arithmetic operation resulted in no change"));
            }

            interp.stack.push(Value {
                data: result_vec,
                display_hint: DisplayHint::Auto,
            });
            return Ok(());
        } else {
            return Err(AjisaiError::StackUnderflow);
        }
    }

    // Stackモードは汎用ハンドラを使用
    binary_arithmetic_op(interp, |a, b| Ok(a.mul(b)))
}

/// / 演算子 - 除算（型チェックなし）
pub fn op_div(interp: &mut Interpreter) -> Result<()> {
    // StackTopモードでブロードキャストの場合、整数スカラー最適化を試みる
    if interp.operation_target == OperationTarget::StackTop {
        if let (Some(b_val), Some(a_val)) = (interp.stack.pop(), interp.stack.pop()) {
            let a_data = &a_val.data;
            let b_data = &b_val.data;

            // NILチェック
            if a_data.is_empty() || b_data.is_empty() {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::from("Cannot operate with NIL"));
            }

            let a_len = a_data.len();
            let b_len = b_data.len();
            let mut result_vec = Vec::with_capacity(a_len.max(b_len));

            // ブロードキャスト判定と整数スカラー最適化
            if a_len > 1 && b_len == 1 {
                // bがスカラー（除数）: 整数の場合は最適化
                let scalar = &b_data[0];
                if scalar.is_zero() {
                    interp.stack.push(a_val);
                    interp.stack.push(b_val);
                    return Err(AjisaiError::DivisionByZero);
                }
                if scalar.is_integer() {
                    for elem in a_data {
                        result_vec.push(elem.div_by_integer(scalar));
                    }
                } else {
                    for elem in a_data {
                        result_vec.push(elem.div(scalar));
                    }
                }
            } else if a_len == 1 && b_len > 1 {
                // aがスカラー（被除数）: bの各要素で割る
                let scalar = &a_data[0];
                for elem in b_data {
                    if elem.is_zero() {
                        interp.stack.push(a_val);
                        interp.stack.push(b_val);
                        return Err(AjisaiError::DivisionByZero);
                    }
                    result_vec.push(scalar.div(elem));
                }
            } else if a_len != b_len {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::VectorLengthMismatch { len1: a_len, len2: b_len });
            } else {
                // 要素ごと演算
                for (a, b) in a_data.iter().zip(b_data.iter()) {
                    if b.is_zero() {
                        interp.stack.push(a_val);
                        interp.stack.push(b_val);
                        return Err(AjisaiError::DivisionByZero);
                    }
                    result_vec.push(a.div(b));
                }
            }

            // "No change is an error" 原則のチェック
            if !interp.disable_no_change_check && (result_vec == *a_data || result_vec == *b_data) {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::from("Arithmetic operation resulted in no change"));
            }

            interp.stack.push(Value {
                data: result_vec,
                display_hint: DisplayHint::Auto,
            });
            return Ok(());
        } else {
            return Err(AjisaiError::StackUnderflow);
        }
    }

    // Stackモードは汎用ハンドラを使用
    binary_arithmetic_op(interp, |a, b| {
        if b.is_zero() {
            Err(AjisaiError::DivisionByZero)
        } else {
            Ok(a.div(b))
        }
    })
}
