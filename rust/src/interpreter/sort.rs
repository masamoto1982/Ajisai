// rust/src/interpreter/sort.rs
//
// 【責務】
// 高速ソートアルゴリズム（SORT）を実装する。
// Introsortアルゴリズムを使用し、分数比較には除算を避けて
// クロス乗算（a/b < c/d ⟺ a*d < b*c）を使用する。
//
// 統一Value宇宙アーキテクチャ版

use crate::interpreter::{Interpreter, OperationTargetMode};
use crate::error::{AjisaiError, Result};
use crate::types::{Value, ValueData};
use crate::types::fraction::Fraction;

// ============================================================================
// ヘルパー関数（統一Value宇宙アーキテクチャ用）
// ============================================================================

/// ベクタ値かどうかを判定
fn is_vector_value(val: &Value) -> bool {
    matches!(&val.data, ValueData::Vector(_))
}

/// 値から数値（Fraction）を抽出する
/// スカラー値の場合にFractionを返す
fn extract_fraction(val: &Value) -> Option<Fraction> {
    val.as_scalar().cloned()
}

/// ベクタの子要素を取得
fn get_vector_children(val: &Value) -> Option<&Vec<Value>> {
    if let ValueData::Vector(children) = &val.data {
        Some(children)
    } else {
        None
    }
}

/// Introsortによる分数ソート（昇順）
fn introsort_fractions(values: &mut [(usize, Fraction)]) {
    values.sort_unstable_by(|a, b| a.1.cmp(&b.1));
}

/// SORT - 高速ソート
pub fn op_sort(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

            if is_vector_value(&val) {
                if let Some(children) = get_vector_children(&val) {
                    if children.is_empty() {
                        // 空ベクタはNILとして返す
                        interp.stack.push(Value::nil());
                        return Ok(());
                    }

                    // 各要素からFractionを抽出
                    let mut indexed_fractions: Vec<(usize, Fraction)> = Vec::with_capacity(children.len());
                    for (i, elem) in children.iter().enumerate() {
                        match extract_fraction(elem) {
                            Some(f) => indexed_fractions.push((i, f)),
                            None => {
                                interp.stack.push(val);
                                return Err(AjisaiError::from(
                                    "SORT requires all elements to be numbers"
                                ));
                            }
                        }
                    }

                    // Introsortでソート
                    introsort_fractions(&mut indexed_fractions);

                    // ソート結果から新しいベクタを構築
                    let sorted_v: Vec<Value> = indexed_fractions
                        .iter()
                        .map(|(orig_idx, _)| children[*orig_idx].clone())
                        .collect();

                    // "No change is an error" チェック
                    if !interp.disable_no_change_check {
                        if children.len() < 2 {
                            interp.stack.push(Value::from_vector(sorted_v));
                            return Err(AjisaiError::from(
                                "SORT resulted in no change on a vector with less than 2 elements"
                            ));
                        }
                        if sorted_v == *children {
                            interp.stack.push(Value::from_vector(sorted_v));
                            return Err(AjisaiError::from(
                                "SORT resulted in no change (vector is already sorted)"
                            ));
                        }
                    }

                    interp.stack.push(Value::from_vector(sorted_v));
                    return Ok(());
                }
            }
            interp.stack.push(val);
            Err(AjisaiError::structure_error("vector", "other format"))
        }
        OperationTargetMode::Stack => {
            if interp.stack.is_empty() {
                return Ok(());
            }

            // スタックの全要素からFractionを抽出
            let mut indexed_fractions: Vec<(usize, Fraction)> = Vec::with_capacity(interp.stack.len());
            for (i, elem) in interp.stack.iter().enumerate() {
                match extract_fraction(elem) {
                    Some(f) => indexed_fractions.push((i, f)),
                    None => {
                        return Err(AjisaiError::from(
                            "SORT requires all stack elements to be numbers"
                        ));
                    }
                }
            }

            // Introsortでソート
            introsort_fractions(&mut indexed_fractions);

            // ソート結果からスタックを再構築
            let original_stack = interp.stack.clone();
            let sorted_stack: Vec<Value> = indexed_fractions
                .iter()
                .map(|(orig_idx, _)| original_stack[*orig_idx].clone())
                .collect();

            // "No change is an error" チェック
            if !interp.disable_no_change_check {
                if original_stack.len() < 2 {
                    return Err(AjisaiError::from(
                        "SORT resulted in no change on a stack with less than 2 elements"
                    ));
                }
                if sorted_stack == original_stack.as_slice() {
                    return Err(AjisaiError::from(
                        "SORT resulted in no change (stack is already sorted)"
                    ));
                }
            }

            interp.stack.clear();
            for val in sorted_stack {
                interp.stack.push(val);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;

    fn make_fraction(num: i64, den: i64) -> Fraction {
        Fraction::new(BigInt::from(num), BigInt::from(den))
    }

    #[test]
    fn test_fraction_comparison() {
        // 1/2 > 1/3 のテスト（クロス乗算: 1*3 > 2*1）
        let half = make_fraction(1, 2);
        let third = make_fraction(1, 3);
        assert!(half > third);

        // 2/3 > 1/2 のテスト
        let two_thirds = make_fraction(2, 3);
        assert!(two_thirds > half);
    }

    #[test]
    fn test_introsort_integers() {
        let mut values = vec![
            (0, make_fraction(32, 1)),
            (1, make_fraction(8, 1)),
            (2, make_fraction(2, 1)),
            (3, make_fraction(18, 1)),
        ];
        introsort_fractions(&mut values);

        // ソート後: 2, 8, 18, 32
        assert_eq!(values[0].1, make_fraction(2, 1));
        assert_eq!(values[1].1, make_fraction(8, 1));
        assert_eq!(values[2].1, make_fraction(18, 1));
        assert_eq!(values[3].1, make_fraction(32, 1));
    }

    #[test]
    fn test_introsort_fractions() {
        let mut values = vec![
            (0, make_fraction(1, 2)),
            (1, make_fraction(1, 3)),
            (2, make_fraction(2, 3)),
        ];
        introsort_fractions(&mut values);

        // ソート後: 1/3, 1/2, 2/3
        assert_eq!(values[0].1, make_fraction(1, 3));
        assert_eq!(values[1].1, make_fraction(1, 2));
        assert_eq!(values[2].1, make_fraction(2, 3));
    }

    #[test]
    fn test_introsort_mixed() {
        let mut values = vec![
            (0, make_fraction(3, 1)),   // 3
            (1, make_fraction(1, 2)),   // 0.5
            (2, make_fraction(2, 1)),   // 2
            (3, make_fraction(1, 4)),   // 0.25
        ];
        introsort_fractions(&mut values);

        // ソート後: 1/4, 1/2, 2, 3
        assert_eq!(values[0].1, make_fraction(1, 4));
        assert_eq!(values[1].1, make_fraction(1, 2));
        assert_eq!(values[2].1, make_fraction(2, 1));
        assert_eq!(values[3].1, make_fraction(3, 1));
    }
}
