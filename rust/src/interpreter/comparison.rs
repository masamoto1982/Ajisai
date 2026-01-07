// rust/src/interpreter/comparison.rs
//
// 統一分数アーキテクチャ版の比較演算
//
// 比較演算の結果は Boolean ヒント付きの値として返す

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::get_integer_from_value;
use crate::types::{Value, DisplayHint};
use crate::types::fraction::Fraction;

// ============================================================================
// 比較演算子
// ============================================================================

/// 二項比較演算の汎用ハンドラ
fn binary_comparison_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> bool,
{
    match interp.operation_target {
        // StackTopモード: 2つの単一要素値を比較
        OperationTarget::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let b_val = interp.stack.pop().unwrap();
            let a_val = interp.stack.pop().unwrap();

            // 単一要素であることを確認
            if a_val.data.len() != 1 || b_val.data.len() != 1 {
                interp.stack.push(a_val);
                interp.stack.push(b_val);
                return Err(AjisaiError::type_error("single-element value", "multi-element or empty value"));
            }

            let result = op(&a_val.data[0], &b_val.data[0]);
            interp.stack.push(Value::from_bool(result));
            Ok(())
        },

        // Stackモード: N個の要素を順に比較
        OperationTarget::Stack => {
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
                // 単一要素であることを確認
                if items[i].data.len() != 1 || items[i + 1].data.len() != 1 {
                    interp.stack.extend(items);
                    interp.stack.push(count_val);
                    return Err(AjisaiError::type_error("single-element value", "multi-element or empty value"));
                }

                if !op(&items[i].data[0], &items[i + 1].data[0]) {
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

/// > 演算子 - 大なり
pub fn op_gt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.gt(b))
}

/// >= 演算子 - 大なりイコール
pub fn op_ge(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.ge(b))
}

/// = 演算子 - 等価比較
///
/// データが完全に等しいかを比較（DisplayHintは無視）
pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        // StackTopモード: 2つの値を比較
        OperationTarget::StackTop => {
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
        OperationTarget::Stack => {
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
