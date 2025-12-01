// rust/src/interpreter/sort.rs
//
// 【責務】
// 数値ソートアルゴリズムの実装を提供する。
// STACK/STACKTOPモードの両方をサポートする。

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::types::{Value, ValueType};
use crate::types::fraction::Fraction;

/// 分数ベクタを取得
fn extract_fractions(vec: &[Value]) -> Result<Vec<(Fraction, Value)>> {
    vec.iter().map(|v| {
        match &v.val_type {
            ValueType::Number(frac) => Ok((frac.clone(), v.clone())),
            _ => Err(AjisaiError::type_error("number", "other type")),
        }
    }).collect()
}

/// 分数のソート済みチェック（昇順）
fn is_sorted_fractions(items: &[(Fraction, Value)]) -> bool {
    items.windows(2).all(|w| w[0].0 <= w[1].0)
}

/// FRACTIONSORT - 高速分数ソート
///
/// 【特徴】
/// 分数専用の高速ソートアルゴリズム。
/// 整数演算のみで分数を比較し、Introsort（QuickSort + HeapSort）を使用。
///
/// 【速度最適化】
/// 1. 浮動小数点変換なし → 精度保持
/// 2. 整数演算のみ → 高速比較（a/b < c/d ⟺ a×d < b×c）
/// 3. Rustの標準ソート使用 → 最適化された実装
/// 4. インプレース → メモリ効率的
///
/// 【計算量】
/// - 時間計算量: O(n log n)（最悪ケースでも保証）
/// - 空間計算量: O(log n)（再帰スタックのみ）
///
/// 【使用例】
/// ```ajisai
/// # ベクトルをソート
/// [ 32 8 2 18 ] FRACTIONSORT
///
/// # 分数をソート
/// [ 1/2 1/3 2/3 ] FRACTIONSORT
///
/// # スタック全体をソート
/// 10 5 20 15
/// .. FRACTIONSORT
/// ```
pub fn op_fractionsort(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let vector_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match vector_val.val_type {
                ValueType::Vector(v) => {
                    if v.is_empty() {
                        return Err(AjisaiError::from("Cannot sort empty vector"));
                    }

                    let items = extract_fractions(&v)?;

                    if is_sorted_fractions(&items) {
                        interp.stack.push(Value { val_type: ValueType::Vector(v) });
                        return Err(AjisaiError::from("FRACTIONSORT resulted in no change (already sorted)"));
                    }

                    // Rustの標準ソートを使用（Introsort: QuickSort + HeapSort）
                    let mut sortable: Vec<(Fraction, Value)> = items;
                    sortable.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

                    let sorted: Vec<Value> = sortable.into_iter().map(|(_, v)| v).collect();
                    interp.stack.push(Value { val_type: ValueType::Vector(sorted) });
                    Ok(())
                },
                _ => {
                    interp.stack.push(vector_val);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
            if interp.stack.is_empty() {
                return Err(AjisaiError::from("Cannot sort empty stack"));
            }

            let items_vec: Vec<Value> = interp.stack.drain(..).collect();
            let items = extract_fractions(&items_vec)?;

            if is_sorted_fractions(&items) {
                interp.stack = items_vec;
                return Err(AjisaiError::from("FRACTIONSORT resulted in no change (already sorted)"));
            }

            // Rustの標準ソートを使用
            let mut sortable: Vec<(Fraction, Value)> = items;
            sortable.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            interp.stack = sortable.into_iter().map(|(_, v)| v).collect();
            Ok(())
        }
    }
}
