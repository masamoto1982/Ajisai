// rust/src/interpreter/sort.rs
//
// 【責務】
// 高速ソートアルゴリズム（SORT）を実装する。
// Introsortアルゴリズムを使用し、分数比較には除算を避けて
// クロス乗算（a/b < c/d ⟺ a*d < b*c）を使用する。
//
// 統一Value宇宙アーキテクチャ版

use crate::error::{AjisaiError, Result};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

fn sort_fractions_by_introsort(values: &mut [(usize, Fraction)]) {
    values.sort_unstable_by(|a, b| a.1.cmp(&b.1));
}

fn reorder_values_by_permutation(source: &[Value], perm: &[usize]) -> Vec<Value> {
    perm.iter()
        .map(|&orig_idx| source[orig_idx].clone())
        .collect()
}

pub fn op_sort(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val = if is_keep_mode {
                interp
                    .stack
                    .last()
                    .cloned()
                    .ok_or(AjisaiError::StackUnderflow)?
            } else {
                interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
            };

            let children = match &val.data {
                ValueData::Vector(children) => children,
                ValueData::Record {
                    pairs: children, ..
                } => children,
                _ => {
                    if !is_keep_mode {
                        interp.stack.push(val);
                    }
                    return Err(AjisaiError::create_structure_error("vector", "other format"));
                }
            };

            if children.is_empty() {
                interp.stack.push(Value::nil());
                return Ok(());
            }

            let mut indexed_fractions: Vec<(usize, Fraction)> = Vec::with_capacity(children.len());
            for (i, elem) in children.iter().enumerate() {
                match elem.as_scalar() {
                    Some(f) => indexed_fractions.push((i, f.clone())),
                    None => {
                        if !is_keep_mode {
                            interp.stack.push(val);
                        }
                        return Err(AjisaiError::from(
                            "SORT requires all elements to be numbers",
                        ));
                    }
                }
            }

            sort_fractions_by_introsort(&mut indexed_fractions);

            let perm: Vec<usize> = indexed_fractions
                .iter()
                .map(|(orig_idx, _)| *orig_idx)
                .collect();
            let sorted_v = reorder_values_by_permutation(children, &perm);

            if !interp.disable_no_change_check {
                if children.len() < 2 {
                    if !is_keep_mode {
                        interp.stack.push(Value::from_vector(sorted_v));
                    }
                    return Err(AjisaiError::NoChange {
                        word: "SORT".into(),
                    });
                }
                if sorted_v == **children {
                    if !is_keep_mode {
                        interp.stack.push(Value::from_vector(sorted_v));
                    }
                    return Err(AjisaiError::NoChange {
                        word: "SORT".into(),
                    });
                }
            }

            interp.stack.push(Value::from_vector(sorted_v));
            Ok(())
        }
        OperationTargetMode::Stack => {
            if interp.stack.is_empty() {
                return Ok(());
            }

            let mut indexed_fractions: Vec<(usize, Fraction)> =
                Vec::with_capacity(interp.stack.len());
            for (i, elem) in interp.stack.iter().enumerate() {
                match elem.as_scalar() {
                    Some(f) => indexed_fractions.push((i, f.clone())),
                    None => {
                        return Err(AjisaiError::from(
                            "SORT requires all stack elements to be numbers",
                        ));
                    }
                }
            }

            sort_fractions_by_introsort(&mut indexed_fractions);

            let is_identity = indexed_fractions
                .iter()
                .enumerate()
                .all(|(i, (orig, _))| *orig == i);
            if !interp.disable_no_change_check {
                if interp.stack.len() < 2 || is_identity {
                    return Err(AjisaiError::NoChange {
                        word: "SORT".into(),
                    });
                }
            }

            let perm: Vec<usize> = indexed_fractions.iter().map(|(orig, _)| *orig).collect();
            let sorted_stack = reorder_values_by_permutation(&interp.stack, &perm);
            if is_keep_mode {
                interp.stack.extend(sorted_stack);
            } else {
                interp.stack = sorted_stack;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;

    fn create_fraction(num: i64, den: i64) -> Fraction {
        Fraction::new(BigInt::from(num), BigInt::from(den))
    }

    #[test]
    fn test_fraction_comparison() {
        let half = create_fraction(1, 2);
        let third = create_fraction(1, 3);
        assert!(half > third);

        let two_thirds = create_fraction(2, 3);
        assert!(two_thirds > half);
    }

    #[test]
    fn test_introsort_integers() {
        let mut values = vec![
            (0, create_fraction(32, 1)),
            (1, create_fraction(8, 1)),
            (2, create_fraction(2, 1)),
            (3, create_fraction(18, 1)),
        ];
        sort_fractions_by_introsort(&mut values);

        assert_eq!(values[0].1, create_fraction(2, 1));
        assert_eq!(values[1].1, create_fraction(8, 1));
        assert_eq!(values[2].1, create_fraction(18, 1));
        assert_eq!(values[3].1, create_fraction(32, 1));
    }

    #[test]
    fn test_sort_fractions_by_introsort() {
        let mut values = vec![
            (0, create_fraction(1, 2)),
            (1, create_fraction(1, 3)),
            (2, create_fraction(2, 3)),
        ];
        sort_fractions_by_introsort(&mut values);

        assert_eq!(values[0].1, create_fraction(1, 3));
        assert_eq!(values[1].1, create_fraction(1, 2));
        assert_eq!(values[2].1, create_fraction(2, 3));
    }

    #[test]
    fn test_introsort_mixed() {
        let mut values = vec![
            (0, create_fraction(3, 1)),
            (1, create_fraction(1, 2)),
            (2, create_fraction(2, 1)),
            (3, create_fraction(1, 4)),
        ];
        sort_fractions_by_introsort(&mut values);

        assert_eq!(values[0].1, create_fraction(1, 4));
        assert_eq!(values[1].1, create_fraction(1, 2));
        assert_eq!(values[2].1, create_fraction(2, 1));
        assert_eq!(values[3].1, create_fraction(3, 1));
    }
}
