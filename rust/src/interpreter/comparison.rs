// rust/src/interpreter/comparison.rs
//
// 【責務】
// 比較演算子（=、<、<=、>、>=）を実装する。
// すべての演算は単一要素ベクタを想定し、結果を単一要素ベクタとして返す。
//
// 【重要: 結果の型について】
// Phase 1.1: 比較演算の結果は常に Vector[Boolean] として返されます。
// Boolean値は数値ではないため、Tensorには変換されません。
// これは意図的な設計であり、算術演算（Tensorを返す）とは異なります。
//
// 例:
//   [3] [5] <     → [TRUE]  (Vector[Boolean])
//   [1 2] [3 4] + → [4 6]   (Tensor[Number])

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::interpreter::helpers::{extract_single_element, get_integer_from_value, wrap_result_value};
use crate::types::{Value, ValueType};
use crate::types::fraction::Fraction;

// ============================================================================
// 二項比較演算の汎用実装
// ============================================================================

/// 二項比較演算の汎用ハンドラ
///
/// 【責務】
/// - StackTopモード: 2つの単一要素ベクタから数値を取り出して比較
/// - Stackモード: N個の要素を順に比較し、全ての隣接ペアが条件を満たすかチェック
/// - 比較結果をBoolean値として返す
/// - すべての比較演算（<、<=、>、>=）で共通使用
///
/// 【StackTopモードの動作】
/// - スタックから2つのベクタをポップ
/// - 各ベクタから単一要素を抽出して比較
/// - 例: `[3] [5] <` → `[true]`
///
/// 【Stackモードの動作】
/// - スタックからカウント値をポップ
/// - 指定個数の要素を取得し、全ての隣接ペアが条件を満たすかチェック
/// - 例: `[1] [2] [3] [3] STACK <` → `(1<2) AND (2<3)` → `[false]`
///
/// 【引数】
/// - op: Fraction同士の比較関数
fn binary_comparison_op<F>(interp: &mut Interpreter, op: F) -> Result<()>
where
    F: Fn(&Fraction, &Fraction) -> bool,
{
    match interp.operation_target {
        // StackTopモード: 2つの単一要素ベクタまたはTensorを比較
        OperationTarget::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let b_vec = interp.stack.pop().unwrap();
            let a_vec = interp.stack.pop().unwrap();

            // TensorまたはVectorから数値を抽出（Tensor優先）
            let a_num = match &a_vec.val_type {
                ValueType::Tensor(t) if t.data().len() == 1 => &t.data()[0],
                ValueType::Vector(v) if v.len() == 1 => {
                    if let ValueType::Number(n) = &v[0].val_type {
                        n
                    } else {
                        interp.stack.push(a_vec);
                        interp.stack.push(b_vec);
                        return Err(AjisaiError::type_error("number", "other type"));
                    }
                },
                _ => {
                    interp.stack.push(a_vec);
                    interp.stack.push(b_vec);
                    return Err(AjisaiError::type_error("single-element vector or tensor", "other type"));
                }
            };

            let b_num = match &b_vec.val_type {
                ValueType::Tensor(t) if t.data().len() == 1 => &t.data()[0],
                ValueType::Vector(v) if v.len() == 1 => {
                    if let ValueType::Number(n) = &v[0].val_type {
                        n
                    } else {
                        interp.stack.push(a_vec);
                        interp.stack.push(b_vec);
                        return Err(AjisaiError::type_error("number", "other type"));
                    }
                },
                _ => {
                    interp.stack.push(a_vec);
                    interp.stack.push(b_vec);
                    return Err(AjisaiError::type_error("single-element vector or tensor", "other type"));
                }
            };

            let result = Value { val_type: ValueType::Boolean(op(a_num, b_num)) };
            interp.stack.push(wrap_result_value(result));
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
                // TensorまたはVectorから数値を抽出（Tensor優先）
                let a_num = match &items[i].val_type {
                    ValueType::Tensor(t) if t.data().len() == 1 => &t.data()[0],
                    ValueType::Vector(v) if v.len() == 1 => {
                        if let ValueType::Number(n) = &v[0].val_type {
                            n
                        } else {
                            interp.stack.extend(items);
                            interp.stack.push(count_val);
                            return Err(AjisaiError::type_error("number", "other type"));
                        }
                    },
                    _ => {
                        interp.stack.extend(items);
                        interp.stack.push(count_val);
                        return Err(AjisaiError::type_error("single-element vector or tensor", "other type"));
                    }
                };

                let b_num = match &items[i + 1].val_type {
                    ValueType::Tensor(t) if t.data().len() == 1 => &t.data()[0],
                    ValueType::Vector(v) if v.len() == 1 => {
                        if let ValueType::Number(n) = &v[0].val_type {
                            n
                        } else {
                            interp.stack.extend(items);
                            interp.stack.push(count_val);
                            return Err(AjisaiError::type_error("number", "other type"));
                        }
                    },
                    _ => {
                        interp.stack.extend(items);
                        interp.stack.push(count_val);
                        return Err(AjisaiError::type_error("single-element vector or tensor", "other type"));
                    }
                };

                if !op(a_num, b_num) {
                    all_true = false;
                    break;
                }
            }

            let result = Value { val_type: ValueType::Boolean(all_true) };
            interp.stack.push(wrap_result_value(result));
            Ok(())
        }
    }
}

// ============================================================================
// 比較演算子
// ============================================================================

/// < 演算子 - 小なり
///
/// 【責務】
/// - 2つの数値を比較し、左辺が右辺より小さいか判定
///
/// 【使用法】
/// - `[3] [5] <` → `[true]`
/// - `[5] [3] <` → `[false]`
/// - `[3] [3] <` → `[false]`
///
/// 【引数スタック】
/// - [b]: 右オペランド（単一要素ベクタの数値）
/// - [a]: 左オペランド（単一要素ベクタの数値）
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - オペランドが数値でない場合
/// - オペランドが単一要素ベクタでない場合
pub fn op_lt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.lt(b))
}

/// <= 演算子 - 小なりイコール
///
/// 【責務】
/// - 2つの数値を比較し、左辺が右辺以下か判定
///
/// 【使用法】
/// - `[3] [5] <=` → `[true]`
/// - `[5] [3] <=` → `[false]`
/// - `[3] [3] <=` → `[true]`
///
/// 【引数スタック】
/// - [b]: 右オペランド（単一要素ベクタの数値）
/// - [a]: 左オペランド（単一要素ベクタの数値）
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - オペランドが数値でない場合
/// - オペランドが単一要素ベクタでない場合
pub fn op_le(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.le(b))
}

/// > 演算子 - 大なり
///
/// 【責務】
/// - 2つの数値を比較し、左辺が右辺より大きいか判定
///
/// 【使用法】
/// - `[5] [3] >` → `[true]`
/// - `[3] [5] >` → `[false]`
/// - `[3] [3] >` → `[false]`
///
/// 【引数スタック】
/// - [b]: 右オペランド（単一要素ベクタの数値）
/// - [a]: 左オペランド（単一要素ベクタの数値）
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - オペランドが数値でない場合
/// - オペランドが単一要素ベクタでない場合
pub fn op_gt(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.gt(b))
}

/// >= 演算子 - 大なりイコール
///
/// 【責務】
/// - 2つの数値を比較し、左辺が右辺以上か判定
///
/// 【使用法】
/// - `[5] [3] >=` → `[true]`
/// - `[3] [5] >=` → `[false]`
/// - `[3] [3] >=` → `[true]`
///
/// 【引数スタック】
/// - [b]: 右オペランド（単一要素ベクタの数値）
/// - [a]: 左オペランド（単一要素ベクタの数値）
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - オペランドが数値でない場合
/// - オペランドが単一要素ベクタでない場合
pub fn op_ge(interp: &mut Interpreter) -> Result<()> {
    binary_comparison_op(interp, |a, b| a.ge(b))
}

/// = 演算子 - 等価比較
///
/// 【責務】
/// - StackTopモード: 2つの値を比較し、完全に等しいか判定
/// - Stackモード: N個の要素を順に比較し、全て等しいか判定
/// - あらゆる型の値を比較可能（Number、String、Boolean、Vector、Nil）
///
/// 【StackTopモードの使用法】
/// - `[3] [3] =` → `[true]`
/// - `[3] [5] =` → `[false]`
/// - `['hello'] ['hello'] =` → `[true]`
/// - `[a b] [a b] =` → `[true]`
///
/// 【Stackモードの使用法】
/// - `[3] [3] [3] [3] STACK =` → `[true]` (全て等しい)
/// - `[1] [2] [1] [3] STACK =` → `[false]` (1≠2)
///
/// 【引数スタック】
/// - StackTopモード: b, a (2つの値)
/// - Stackモード: count (要素数)
///
/// 【戻り値スタック】
/// - [result]: 比較結果（Boolean）
///
/// 【エラー】
/// - なし（すべての型で比較可能）
pub fn op_eq(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        // StackTopモード: 2つの値を比較
        OperationTarget::StackTop => {
            if interp.stack.len() < 2 {
                return Err(AjisaiError::StackUnderflow);
            }

            let b_vec = interp.stack.pop().unwrap();
            let a_vec = interp.stack.pop().unwrap();

            let result = Value { val_type: ValueType::Boolean(a_vec == b_vec) };
            interp.stack.push(wrap_result_value(result));
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
            let mut all_equal = true;
            for i in 0..items.len() - 1 {
                if items[i] != items[i + 1] {
                    all_equal = false;
                    break;
                }
            }

            let result = Value { val_type: ValueType::Boolean(all_equal) };
            interp.stack.push(wrap_result_value(result));
            Ok(())
        }
    }
}
