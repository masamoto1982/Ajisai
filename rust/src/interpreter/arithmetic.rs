// rust/src/interpreter/arithmetic.rs
//
// 【責務】
// 算術演算（+、-、*、/）を実装する。
// StackTopモードではベクタ間の要素ごと演算をサポートし、
// Stackモードでは複数要素の畳み込み演算を実行する。
// ブロードキャスト機能（スカラーとベクタの演算）も提供する。

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{get_integer_from_value, extract_number, wrap_in_square_vector};
use crate::types::{Value, ValueType, BracketType};
use crate::types::fraction::Fraction;
use num_traits::Zero;

// ============================================================================
// 二項演算の汎用実装
// ============================================================================

/// 二項算術演算の汎用ハンドラ
///
/// 【責務】
/// - StackTopモード: ベクタ間の要素ごと演算、ブロードキャスト対応
/// - Stackモード: N個の要素を左から右へ畳み込み演算
/// - "No change is an error" 原則の適用
///
/// 【StackTopモードの動作】
/// 1. 要素数が等しい場合: 要素ごとに演算
/// 2. 片方が単一要素の場合: スカラーとしてブロードキャスト
/// 3. それ以外: 長さ不一致エラー
///
/// 【Stackモードの動作】
/// - スタックトップから指定個数の要素を取得し、順に演算
/// - 例: `[2] [3] [4] [3] STACK +` → `[2+3+4] = [9]`
///
/// 【"No change is an error" 原則】
/// - 演算結果が入力と同一の場合はエラー（例: [0]と加算、[1]と乗算）
///
/// 【引数】
/// - op: Fraction同士の演算関数
fn binary_arithmetic_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> Result<Fraction>,
{
    match interp.operation_target {
        // StackTopモード: ベクタ間の要素ごと演算
        OperationTarget::StackTop => {
            let b_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let a_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            let (a_vec, a_bracket) = match a_val.val_type {
                ValueType::Vector(v, b) => (v, b),
                _ => {
                    interp.stack.push(a_val);
                    interp.stack.push(b_val);
                    return Err(AjisaiError::type_error("vector", "other type"));
                }
            };
            let (b_vec, _) = match b_val.val_type {
                ValueType::Vector(v, b) => (v, b),
                _ => {
                    interp.stack.push(Value { val_type: ValueType::Vector(a_vec, a_bracket) });
                    interp.stack.push(b_val);
                    return Err(AjisaiError::type_error("vector", "other type"));
                }
            };

            let a_len = a_vec.len();
            let b_len = b_vec.len();

            let mut result_vec = Vec::new();

            // ブロードキャスト判定と要素ごと演算
            if a_len > 1 && b_len == 1 {
                // aがベクタ、bがスカラー: bを各要素にブロードキャスト
                let scalar = &b_vec[0];
                for elem in &a_vec {
                    let res_num = op(extract_number(elem)?, extract_number(scalar)?)?;
                    result_vec.push(Value { val_type: ValueType::Number(res_num) });
                }
            } else if a_len == 1 && b_len > 1 {
                // aがスカラー、bがベクタ: aを各要素にブロードキャスト
                let scalar = &a_vec[0];
                for elem in &b_vec {
                    let res_num = op(extract_number(scalar)?, extract_number(elem)?)?;
                    result_vec.push(Value { val_type: ValueType::Number(res_num) });
                }
            } else {
                // 要素数が等しい、または両方とも単一要素
                if a_len != b_len {
                    interp.stack.push(Value { val_type: ValueType::Vector(a_vec, a_bracket) });
                    interp.stack.push(Value { val_type: ValueType::Vector(b_vec, BracketType::Square) });
                    return Err(AjisaiError::VectorLengthMismatch{ len1: a_len, len2: b_len });
                }
                for (a, b) in a_vec.iter().zip(b_vec.iter()) {
                    let res_num = op(extract_number(a)?, extract_number(b)?)?;
                    result_vec.push(Value { val_type: ValueType::Number(res_num) });
                }
            }

            // "No change is an error" 原則のチェック
            let result_value = Value { val_type: ValueType::Vector(result_vec.clone(), a_bracket.clone()) };
            let original_a = Value { val_type: ValueType::Vector(a_vec, a_bracket.clone()) };
            let original_b = Value { val_type: ValueType::Vector(b_vec, BracketType::Square) };

            if result_value == original_a || result_value == original_b {
                interp.stack.push(original_a);
                interp.stack.push(original_b);
                return Err(AjisaiError::from("Arithmetic operation resulted in no change"));
            }

            interp.stack.push(result_value);
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

            let mut acc_num = extract_number(&items[0])?.clone();
            let original_first = acc_num.clone();

            for item in items.iter().skip(1) {
                acc_num = op(&acc_num, extract_number(item)?)?;
            }

            // "No change is an error" 原則のチェック
            if acc_num == original_first {
                interp.stack.extend(items);
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK operation resulted in no change"));
            }

            let result_val = Value { val_type: ValueType::Number(acc_num) };
            interp.stack.push(wrap_in_square_vector(result_val));
        }
    }
    Ok(())
}

// ============================================================================
// 算術演算の実装
// ============================================================================

/// + 演算子 - 加算
///
/// 【責務】
/// - 数値の加算を実行
/// - ベクタ間の要素ごと加算
/// - スカラーブロードキャスト加算
///
/// 【使用法】
/// - StackTopモード: `[1 2 3] [4 5 6] +` → `[5 7 9]`
/// - StackTopモード（ブロードキャスト）: `[1 2 3] [10] +` → `[11 12 13]`
/// - Stackモード: `[1] [2] [3] [3] STACK +` → `[6]`
///
/// 【引数スタック】
/// - (StackTopモード) b: 右オペランド（ベクタ）
/// - (StackTopモード) a: 左オペランド（ベクタ）
/// - (Stackモード) [count]: 演算対象の要素数
///
/// 【戻り値スタック】
/// - 加算結果のベクタ
///
/// 【エラー】
/// - ベクタ長が不一致（ブロードキャスト不可の場合）
/// - 演算結果に変化がない場合
pub fn op_add(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| Ok(a.add(b)))
}

/// - 演算子 - 減算
///
/// 【責務】
/// - 数値の減算を実行
/// - ベクタ間の要素ごと減算
/// - スカラーブロードキャスト減算
///
/// 【使用法】
/// - StackTopモード: `[5 7 9] [1 2 3] -` → `[4 5 6]`
/// - StackTopモード（ブロードキャスト）: `[10 20 30] [5] -` → `[5 15 25]`
/// - Stackモード: `[10] [3] [2] [3] STACK -` → `[5]` (10-3-2)
///
/// 【引数スタック】
/// - (StackTopモード) b: 右オペランド（ベクタ）
/// - (StackTopモード) a: 左オペランド（ベクタ）
/// - (Stackモード) [count]: 演算対象の要素数
///
/// 【戻り値スタック】
/// - 減算結果のベクタ
///
/// 【エラー】
/// - ベクタ長が不一致（ブロードキャスト不可の場合）
/// - 演算結果に変化がない場合
pub fn op_sub(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| Ok(a.sub(b)))
}

/// * 演算子 - 乗算
///
/// 【責務】
/// - 数値の乗算を実行
/// - ベクタ間の要素ごと乗算
/// - スカラーブロードキャスト乗算
///
/// 【使用法】
/// - StackTopモード: `[1 2 3] [4 5 6] *` → `[4 10 18]`
/// - StackTopモード（ブロードキャスト）: `[1 2 3] [10] *` → `[10 20 30]`
/// - Stackモード: `[2] [3] [4] [3] STACK *` → `[24]` (2*3*4)
///
/// 【引数スタック】
/// - (StackTopモード) b: 右オペランド（ベクタ）
/// - (StackTopモード) a: 左オペランド（ベクタ）
/// - (Stackモード) [count]: 演算対象の要素数
///
/// 【戻り値スタック】
/// - 乗算結果のベクタ
///
/// 【エラー】
/// - ベクタ長が不一致（ブロードキャスト不可の場合）
/// - 演算結果に変化がない場合
pub fn op_mul(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| Ok(a.mul(b)))
}

/// / 演算子 - 除算
///
/// 【責務】
/// - 数値の除算を実行
/// - ベクタ間の要素ごと除算
/// - スカラーブロードキャスト除算
/// - ゼロ除算チェック
///
/// 【使用法】
/// - StackTopモード: `[10 20 30] [2 4 5] /` → `[5 5 6]`
/// - StackTopモード（ブロードキャスト）: `[10 20 30] [10] /` → `[1 2 3]`
/// - Stackモード: `[100] [2] [5] [3] STACK /` → `[10]` (100/2/5)
///
/// 【引数スタック】
/// - (StackTopモード) b: 右オペランド（ベクタ）
/// - (StackTopモード) a: 左オペランド（ベクタ）
/// - (Stackモード) [count]: 演算対象の要素数
///
/// 【戻り値スタック】
/// - 除算結果のベクタ
///
/// 【エラー】
/// - ゼロ除算の場合
/// - ベクタ長が不一致（ブロードキャスト不可の場合）
/// - 演算結果に変化がない場合
pub fn op_div(interp: &mut Interpreter) -> Result<()> {
    binary_arithmetic_op(interp, |a, b| {
        if b.numerator.is_zero() {
            Err(AjisaiError::DivisionByZero)
        } else {
            Ok(a.div(b))
        }
    })
}
