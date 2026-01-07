// rust/src/interpreter/sort.rs
//
// 【責務】
// 高速ソートアルゴリズム（SORT）を実装する。
// Introsortアルゴリズムを使用し、分数比較には除算を避けて
// クロス乗算（a/b < c/d ⟺ a*d < b*c）を使用する。

use crate::interpreter::{Interpreter, OperationTarget};
use crate::error::{AjisaiError, Result};
use crate::types::{Value, ValueType};
use crate::types::fraction::Fraction;

/// 値から数値（Fraction）を抽出する
/// 単一要素ベクタの場合は中の値を取り出す
fn extract_fraction(val: &Value) -> Option<Fraction> {
    match val.val_type() {
        ValueType::Number(f) => Some(f.clone()),
        ValueType::Vector(v) if v.len() == 1 => {
            match v[0].val_type() {
                ValueType::Number(f) => Some(f.clone()),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Introsortによる分数ソート（昇順）
///
/// Introsortは3つのアルゴリズムを組み合わせた高速ソート：
/// - QuickSort: 通常の高速処理
/// - HeapSort: 最悪ケースの保険（深さ制限超過時）
/// - InsertionSort: 小さな配列の最適化
///
/// Rustの標準sort_unstableはpdqsort（Pattern-defeating Quicksort）を使用しており、
/// これはIntrosortの発展形である。
fn introsort_fractions(values: &mut [(usize, Fraction)]) {
    // Rustのsort_unstable_byは内部的にIntrosort系（pdqsort）を使用
    // 安定ソートが不要な場合、より高速
    values.sort_unstable_by(|a, b| a.1.cmp(&b.1));
}

/// SORT - 高速ソート
///
/// 【責務】
/// - ベクタまたはスタック内の数値を昇順にソート
/// - 分数比較にはクロス乗算（a/b < c/d ⟺ a*d < b*c）を使用
/// - Introsortアルゴリズムで高速かつ安定した性能を提供
///
/// 【使用法】
/// - StackTopモード: `[ 32 8 2 18 ] SORT` → `[ 2 8 18 32 ]`
/// - StackTopモード: `[ 1/2 1/3 2/3 ] SORT` → `[ 1/3 1/2 2/3 ]`
/// - Stackモード: `64 25 12 22 11 .. SORT` → スタック全体が昇順に
///
/// 【動作原理】
/// 1. 分数の比較: a/b と c/d を比較する際、a×d と b×c を比較
///    これにより除算を避け、精度を保ちながら高速に比較
/// 2. Introsortアルゴリズム: QuickSort + HeapSort + InsertionSort
///    データの状態に応じて自動的に最適なアルゴリズムを選択
///
/// 【エラー】
/// - 対象に数値以外の要素が含まれる場合
/// - 対象がベクタでない場合（StackTopモード）
pub fn op_sort(interp: &mut Interpreter) -> Result<()> {
    match interp.operation_target {
        OperationTarget::StackTop => {
            let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            match val.val_type() {
                ValueType::Vector(v) => {
                    let v = v.clone();
                    if v.is_empty() {
                        // 空ベクタはそのまま返す
                        interp.stack.push(Value::from_vector(v));
                        return Ok(());
                    }

                    // 各要素からFractionを抽出
                    let mut indexed_fractions: Vec<(usize, Fraction)> = Vec::with_capacity(v.len());
                    for (i, elem) in v.iter().enumerate() {
                        match extract_fraction(elem) {
                            Some(f) => indexed_fractions.push((i, f)),
                            None => {
                                interp.stack.push(Value::from_vector(v));
                                return Err(AjisaiError::from(
                                    "SORT requires all elements to be numbers"
                                ));
                            }
                        }
                    }

                    // Introsortでソート
                    introsort_fractions(&mut indexed_fractions);

                    // ソート結果から新しいベクタを構築
                    // 元の値の構造（[n]か単なるnか）を維持
                    let sorted_v: Vec<Value> = indexed_fractions
                        .iter()
                        .map(|(orig_idx, _)| v[*orig_idx].clone())
                        .collect();

                    // "No change is an error" チェック
                    if !interp.disable_no_change_check {
                        if v.len() < 2 {
                            interp.stack.push(Value::from_vector(sorted_v));
                            return Err(AjisaiError::from(
                                "SORT resulted in no change on a vector with less than 2 elements"
                            ));
                        }
                        if sorted_v == v {
                            interp.stack.push(Value::from_vector(sorted_v));
                            return Err(AjisaiError::from(
                                "SORT resulted in no change (vector is already sorted)"
                            ));
                        }
                    }

                    interp.stack.push(Value::from_vector(sorted_v));
                    Ok(())
                },
                _ => {
                    interp.stack.push(val);
                    Err(AjisaiError::type_error("vector", "other type"))
                }
            }
        }
        OperationTarget::Stack => {
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
