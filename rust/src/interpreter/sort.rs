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
        .collect::<Vec<Value>>()
}

pub fn op_sort(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode: bool = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val: Value = if is_keep_mode {
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
                ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => {
                    if !is_keep_mode {
                        interp.stack.push(val);
                    }
                    return Err(AjisaiError::create_structure_error(
                        "SORT: expected vector, got non-vector value",
                        "other format",
                    ));
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
                            "SORT: expected all elements to be numbers, got non-number element",
                        ));
                    }
                }
            }

            sort_fractions_by_introsort(&mut indexed_fractions);

            let perm: Vec<usize> = indexed_fractions
                .iter()
                .map(|(orig_idx, _)| *orig_idx)
                .collect::<Vec<usize>>();
            let sorted_v: Vec<Value> = reorder_values_by_permutation(children, &perm);
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
                            "SORT: expected all stack elements to be numbers, got non-number element",
                        ));
                    }
                }
            }

            sort_fractions_by_introsort(&mut indexed_fractions);
            let perm: Vec<usize> = indexed_fractions
                .iter()
                .map(|(orig, _)| *orig)
                .collect::<Vec<usize>>();
            let sorted_stack: Vec<Value> = reorder_values_by_permutation(&interp.stack, &perm);
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
        let half: Fraction = create_fraction(1, 2);
        let third: Fraction = create_fraction(1, 3);
        assert!(half > third);

        let two_thirds: Fraction = create_fraction(2, 3);
        assert!(two_thirds > half);
    }

    #[test]
    fn test_introsort_integers() {
        let mut values: Vec<(usize, Fraction)> = vec![
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
        let mut values: Vec<(usize, Fraction)> = vec![
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
        let mut values: Vec<(usize, Fraction)> = vec![
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
