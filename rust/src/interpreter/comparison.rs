// rust/src/interpreter/comparison.rs
//
// 統一Value宇宙アーキテクチャ版の比較演算
//
// 比較演算の結果は Boolean ヒント付きの値として返す

use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::get_integer_from_value;
use crate::interpreter::tensor_ops::FlatTensor;
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

// ============================================================================
// ヘルパー関数
// ============================================================================

fn extract_scalar_for_comparison(val: &Value) -> Result<Fraction> {
    match &val.data {
        ValueData::Scalar(f) => Ok(f.clone()),
        ValueData::Vector(_) | ValueData::Record { .. } => {
            let tensor = FlatTensor::from_value(val)?;
            if tensor.data.len() != 1 {
                return Err(AjisaiError::structure_error(
                    "scalar value",
                    "non-scalar value",
                ));
            }
            Ok(tensor.data[0].clone())
        }
        _ => Err(AjisaiError::structure_error(
            "scalar value",
            "non-scalar value",
        )),
    }
}

fn all_adjacent_pairs_match<F>(items: &[Value], op: F) -> Result<bool>
where
    F: Fn(&Fraction, &Fraction) -> bool,
{
    for pair in items.windows(2) {
        let a_scalar = extract_scalar_for_comparison(&pair[0])?;
        let b_scalar = extract_scalar_for_comparison(&pair[1])?;
        if !op(&a_scalar, &b_scalar) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn all_adjacent_data_equal(items: &[Value]) -> bool {
    items.windows(2).all(|pair| pair[0].data == pair[1].data)
}

// ============================================================================
// 比較演算子
// ============================================================================

/// 二項比較演算の汎用ハンドラ（Fold型）
///
/// 【消費モード】
/// - Consume（デフォルト）: オペランドを消費し、結果をプッシュ
/// - Keep（,,）: オペランドを保持し、結果を追加
fn binary_comparison_op<F>(interp: &mut Interpreter, op: F, op_name: &str) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> bool,
{
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        // StackTopモード: 2つの単一要素値を比較
        OperationTargetMode::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let (a_val, b_val) = if is_keep_mode {
                // Keep mode: peek without removing
                let stack_len = interp.stack.len();
                let a_val = interp.stack[stack_len - 2].clone();
                let b_val = interp.stack[stack_len - 1].clone();
                (a_val, b_val)
            } else {
                // Consume mode: pop
                let b_val = interp.stack.pop().unwrap();
                let a_val = interp.stack.pop().unwrap();
                (a_val, b_val)
            };

            // スカラーを抽出（単一要素ベクタも許容）
            let a_scalar = match extract_scalar_for_comparison(&a_val) {
                Ok(f) => f,
                Err(e) => {
                    if !is_keep_mode {
                        interp.stack.push(a_val);
                        interp.stack.push(b_val);
                    }
                    return Err(e);
                }
            };
            let b_scalar = match extract_scalar_for_comparison(&b_val) {
                Ok(f) => f,
                Err(e) => {
                    if !is_keep_mode {
                        interp.stack.push(a_val);
                        interp.stack.push(b_val);
                    }
                    return Err(e);
                }
            };

            let result = op(&a_scalar, &b_scalar);
            if interp.gui_mode {
                interp
                    .stack
                    .push(Value::from_vector(vec![Value::from_bool(result)]));
            } else {
                interp.stack.push(Value::from_bool(result));
            }
            Ok(())
        }

        // Stackモード: N個の要素を順に比較
        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            // カウント0, 1はエラー（"No change is an error"原則）
            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::NoChange {
                    word: op_name.into(),
                });
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = if is_keep_mode {
                // Keep mode: peek without removing
                let stack_len = interp.stack.len();
                interp.stack[stack_len - count..].iter().cloned().collect()
            } else {
                // Consume mode: drain
                interp.stack.drain(interp.stack.len() - count..).collect()
            };

            // 全ての隣接ペアをチェック
            let all_true = match all_adjacent_pairs_match(&items, op) {
                Ok(v) => v,
                Err(e) => {
                    if !is_keep_mode {
                        interp.stack.extend(items);
                    }
                    interp.stack.push(count_val);
                    return Err(e);
                }
            };

            interp.stack.push(Value::from_bool(all_true));
            Ok(())
        }
    }
}

/// < 演算子 - 小なり
pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.lt(b), "<")
}

/// <= 演算子 - 小なりイコール
pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.le(b), "<=")
}

// > と >= は廃止されました
// 代わりに < NOT または オペランドを逆にして < を使用してください

/// = 演算子 - 等価比較
///
/// データが完全に等しいかを比較（DisplayHintは無視）
pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        // StackTopモード: 2つの値を比較
        OperationTargetMode::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let (a_val, b_val) = if is_keep_mode {
                let stack_len = interp.stack.len();
                let a_val = interp.stack[stack_len - 2].clone();
                let b_val = interp.stack[stack_len - 1].clone();
                (a_val, b_val)
            } else {
                let b_val = interp.stack.pop().unwrap();
                let a_val = interp.stack.pop().unwrap();
                (a_val, b_val)
            };

            // データが等しいかを比較（DisplayHintは無視）
            let result = a_val.data == b_val.data;
            if interp.gui_mode {
                interp
                    .stack
                    .push(Value::from_vector(vec![Value::from_bool(result)]));
            } else {
                interp.stack.push(Value::from_bool(result));
            }
            Ok(())
        }

        // Stackモード: N個の要素を順に比較
        OperationTargetMode::Stack => {
            let count_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count = get_integer_from_value(&count_val)? as usize;

            // カウント0, 1はエラー（"No change is an error"原則）
            if count == 0 || count == 1 {
                interp.stack.push(count_val);
                return Err(AjisaiError::NoChange { word: "=".into() });
            }

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let items: Vec<Value> = if is_keep_mode {
                let stack_len = interp.stack.len();
                interp.stack[stack_len - count..].iter().cloned().collect()
            } else {
                interp.stack.drain(interp.stack.len() - count..).collect()
            };

            let all_equal = all_adjacent_data_equal(&items);
            interp.stack.push(Value::from_bool(all_equal));
            Ok(())
        }
    }
}
