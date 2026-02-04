// rust/src/interpreter/comparison.rs
//
// 統一Value宇宙アーキテクチャ版の比較演算
//
// 比較演算の結果は Boolean ヒント付きの値として返す

use crate::interpreter::{Interpreter, OperationTargetMode};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::get_integer_from_value;
use crate::types::{Value, ValueData};
use crate::types::fraction::Fraction;

// ============================================================================
// ヘルパー関数
// ============================================================================

/// 値からスカラー（Fraction）を抽出
/// - Scalar: そのままFractionを返す
/// - 単一要素Vector: 内部のスカラーを抽出
/// - それ以外: None
fn extract_scalar_for_comparison(val: &Value) -> Option<&Fraction> {
    match &val.data {
        ValueData::Scalar(f) => Some(f),
        ValueData::Vector(children) if children.len() == 1 => {
            // 単一要素ベクタの場合、その中のスカラーを取り出す
            extract_scalar_for_comparison(&children[0])
        },
        _ => None
    }
}

// ============================================================================
// 比較演算子
// ============================================================================

/// 二項比較演算の汎用ハンドラ
fn binary_comparison_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> bool,
{
    match interp.operation_target_mode {
        // StackTopモード: 2つの単一要素値を比較
        OperationTargetMode::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let b_val = interp.stack.pop().unwrap();
            let a_val = interp.stack.pop().unwrap();

            // スカラーを抽出（単一要素ベクタも許容）
            let a_scalar = match extract_scalar_for_comparison(&a_val) {
                Some(f) => f,
                None => {
                    interp.stack.push(a_val);
                    interp.stack.push(b_val);
                    return Err(AjisaiError::structure_error("scalar value", "non-scalar value"));
                }
            };
            let b_scalar = match extract_scalar_for_comparison(&b_val) {
                Some(f) => f,
                None => {
                    interp.stack.push(a_val);
                    interp.stack.push(b_val);
                    return Err(AjisaiError::structure_error("scalar value", "non-scalar value"));
                }
            };

            let result = op(a_scalar, b_scalar);
            interp.stack.push(Value::from_bool(result));
            Ok(())
        },

        // Stackモード: N個の要素を順に比較
        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            // カウント0, 1はエラー（"No change is an error"原則）
            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK comparison with count 0 or 1 results in no change"));
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();

            // 全ての隣接ペアをチェック
            let mut all_true = true;
            for i in 0..items.len() - 1 {
                // スカラーを抽出（単一要素ベクタも許容）
                let a_scalar = match extract_scalar_for_comparison(&items[i]) {
                    Some(f) => f,
                    None => {
                        interp.stack.extend(items);
                        interp.stack.push(count_val);
                        return Err(AjisaiError::structure_error("scalar value", "non-scalar value"));
                    }
                };
                let b_scalar = match extract_scalar_for_comparison(&items[i + 1]) {
                    Some(f) => f,
                    None => {
                        interp.stack.extend(items);
                        interp.stack.push(count_val);
                        return Err(AjisaiError::structure_error("scalar value", "non-scalar value"));
                    }
                };

                if !op(a_scalar, b_scalar) {
                    all_true = false;
                    break;
                }
            }

            interp.stack.push(Value::from_bool(all_true));
            Ok(())
        }
    }
}

/// < 演算子 - 小なり
pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.lt(b))
}

/// <= 演算子 - 小なりイコール
pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.le(b))
}

// > と >= は廃止されました
// 代わりに < NOT または オペランドを逆にして < を使用してください

/// = 演算子 - 等価比較
///
/// データが完全に等しいかを比較（DisplayHintは無視）
pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target_mode {
        // StackTopモード: 2つの値を比較
        OperationTargetMode::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let b_val = interp.stack.pop().unwrap();
            let a_val = interp.stack.pop().unwrap();

            // データが等しいかを比較（DisplayHintは無視）
            let result = a_val.data == b_val.data;
            interp.stack.push(Value::from_bool(result));
            Ok(())
        },

        // Stackモード: N個の要素を順に比較
        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            // カウント0, 1はエラー（"No change is an error"原則）
            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::from("STACK comparison with count 0 or 1 results in no change"));
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();

            // 全ての隣接ペアをチェック（データのみ比較）
            let mut all_equal = true;
            for i in 0..items.len() - 1 {
                if items[i].data != items[i + 1].data {
                    all_equal = false;
                    break;
                }
            }

            interp.stack.push(Value::from_bool(all_equal));
            Ok(())
        }
    }
}
