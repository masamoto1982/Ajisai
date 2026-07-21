use crate::error::{AjisaiError, Result};
use crate::interpreter::comparison::{three_way_compare, OrderOutcome};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::Stack;
use crate::types::Value;
use std::cell::RefCell;

fn reorder_values_by_permutation(source: &[Value], perm: &[usize]) -> Vec<Value> {
    perm.iter()
        .map(|&orig_idx| source[orig_idx].clone())
        .collect::<Vec<Value>>()
}

/// Outcome of attempting to sort a slice of values under the SPEC §7.4.3
/// budgeted comparison.
enum SortAttempt {
    /// Every required comparison decided; `perm` is the ascending permutation
    /// of the original indices.
    Ordered(Vec<usize>),
    /// At least one required comparison was undecidable within the budget; the
    /// sorted order as a whole is not established (SPEC §7.4.3: a partial order
    /// is not a sort). Carries the agreed-prefix of the first undecidable pair.
    Undecided(usize),
    /// An element was structurally non-comparable (non-numeric) — malformed use
    /// (SPEC §11.2), distinct from the logical Unknown.
    Malformed(AjisaiError),
}

/// Sort the indices `0..items.len()` by the values' ascending order under the
/// budgeted continued-fraction comparison (SPEC §7.4.1). A single undecidable
/// pair makes the whole order unestablished — reported as `Undecided` with the
/// first such pair's agreed-prefix — and `SORT` then yields the logical
/// `Unknown` rather than a partially-sorted vector. A non-comparable element
/// is reported as `Malformed`.
fn try_sort_indices(items: &[Value]) -> SortAttempt {
    // Captured by the comparator: the first malformed error and the first
    // undecidable agreed-prefix. When either is set the produced permutation
    // is discarded, so returning `Equal` from the comparator in those cases is
    // harmless to correctness.
    let malformed: RefCell<Option<AjisaiError>> = RefCell::new(None);
    let undecided: RefCell<Option<usize>> = RefCell::new(None);

    let mut perm: Vec<usize> = (0..items.len()).collect();
    perm.sort_by(|&i, &j| match three_way_compare(&items[i], &items[j]) {
        Ok(OrderOutcome::Decided(ord)) => ord,
        Ok(OrderOutcome::Undecided(prefix)) => {
            let mut slot = undecided.borrow_mut();
            if slot.is_none() {
                *slot = Some(prefix);
            }
            std::cmp::Ordering::Equal
        }
        Err(e) => {
            let mut slot = malformed.borrow_mut();
            if slot.is_none() {
                *slot = Some(e);
            }
            std::cmp::Ordering::Equal
        }
    });

    if let Some(e) = malformed.into_inner() {
        return SortAttempt::Malformed(e);
    }
    if let Some(prefix) = undecided.into_inner() {
        return SortAttempt::Undecided(prefix);
    }
    SortAttempt::Ordered(perm)
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

            // VTU Phase III boundary helper: as_vector_view() borrows for
            // Vector/Record and materializes once for Tensor, collapsing the
            // old representation-juggling.
            let children = match val.as_vector_view() {
                Some(view) => view,
                None => {
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

            match try_sort_indices(&children) {
                SortAttempt::Ordered(perm) => {
                    let sorted_v: Vec<Value> = reorder_values_by_permutation(&children, &perm);
                    interp.stack.push(Value::from_vector(sorted_v));
                    Ok(())
                }
                SortAttempt::Undecided(agreed_prefix) => {
                    // SPEC §7.4.3: a single undecidable pair makes the whole
                    // order unestablished — yield the logical Unknown (U),
                    // never a partially-sorted vector.
                    crate::interpreter::comparison::push_comparison_unknown(interp, agreed_prefix);
                    Ok(())
                }
                SortAttempt::Malformed(e) => {
                    if !is_keep_mode {
                        interp.stack.push(val);
                    }
                    Err(e)
                }
            }
        }
        OperationTargetMode::Stack => {
            if interp.stack.is_empty() {
                return Ok(());
            }

            let items: Vec<Value> = interp.stack.to_vec();
            match try_sort_indices(&items) {
                SortAttempt::Ordered(perm) => {
                    let sorted_stack: Vec<Value> = reorder_values_by_permutation(&items, &perm);
                    if is_keep_mode {
                        interp.stack.extend(sorted_stack);
                    } else {
                        interp.stack = Stack::from_values(sorted_stack);
                    }
                    Ok(())
                }
                SortAttempt::Undecided(agreed_prefix) => {
                    // SPEC §7.4.3: the whole sorted order is unestablished;
                    // leave the operands in place and push the logical Unknown.
                    crate::interpreter::comparison::push_comparison_unknown(interp, agreed_prefix);
                    Ok(())
                }
                SortAttempt::Malformed(e) => Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::fraction::Fraction;
    use num_bigint::BigInt;

    fn scalar(num: i64, den: i64) -> Value {
        Value::from_fraction(Fraction::new(BigInt::from(num), BigInt::from(den)))
    }

    fn ordered(items: &[Value]) -> Vec<usize> {
        match try_sort_indices(items) {
            SortAttempt::Ordered(perm) => perm,
            SortAttempt::Undecided(_) => panic!("expected decidable sort"),
            SortAttempt::Malformed(e) => panic!("unexpected malformed: {e}"),
        }
    }

    #[test]
    fn try_sort_orders_integers_ascending() {
        let items = vec![scalar(32, 1), scalar(8, 1), scalar(2, 1), scalar(18, 1)];
        let perm = ordered(&items);
        // ascending: 2(idx2), 8(idx1), 18(idx3), 32(idx0)
        assert_eq!(perm, vec![2, 1, 3, 0]);
    }

    #[test]
    fn try_sort_orders_fractions_ascending() {
        let items = vec![scalar(1, 2), scalar(1, 3), scalar(2, 3)];
        let perm = ordered(&items);
        // ascending: 1/3(idx1), 1/2(idx0), 2/3(idx2)
        assert_eq!(perm, vec![1, 0, 2]);
    }

    #[test]
    fn try_sort_reports_malformed_on_non_numeric() {
        // A multi-element vector is not a comparable scalar (a singleton
        // vector would project to its sole scalar, so use two elements).
        let non_numeric = Value::from_vector(vec![scalar(1, 1), scalar(2, 1)]);
        let items = vec![scalar(1, 1), non_numeric];
        assert!(matches!(
            try_sort_indices(&items),
            SortAttempt::Malformed(_)
        ));
    }
}
