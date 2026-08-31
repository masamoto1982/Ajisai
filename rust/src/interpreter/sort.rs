use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::comparison::{three_way_compare, OrderOutcome};
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::semantic::Recoverability;
use crate::types::Value;
use std::cell::RefCell;

/// The logical Unknown as a plain NIL (SPEC §7.4.3): `SORT`/`ORDER`'s output
/// domain is a vector, not a truth value, so — unlike the comparison words'
/// `undecidable_truth_value` — this carries no `TruthValue` hint.
fn undecidable_nil() -> Value {
    Value::nil_with_reason(NilReason::Undecidable, Recoverability::Retryable)
}

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
    /// A required comparison exhausted its refinement budget (LANG.VALUES.EXACT):
    /// a Tier 2 pair (`PI`) that never separated. One undecidable pair leaves
    /// the whole order unestablished.
    Undecided,
    /// An element was structurally non-comparable (non-numeric) — malformed use
    /// (LANG.FAILURE.ERROR).
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
    if undecided.into_inner().is_some() {
        return SortAttempt::Undecided;
    }
    SortAttempt::Ordered(perm)
}

/// The ascending, stable index permutation of `items` — `ORDER`'s answer, and
/// the permutation `SORT` applies to produce its own.
///
/// Shared so the two Words cannot disagree about an ordering: `xs ORDER` and
/// `xs SORT` are the same comparison sequence, read two ways.
///
/// `Ok(None)` is the undecidable case: a required comparison exhausted its
/// budget, so no permutation exists to report. `Err(_)` stays reserved for
/// malformed use (a structurally non-comparable element).
pub(crate) fn order_indices(items: &[Value]) -> Result<Option<Vec<usize>>> {
    match try_sort_indices(items) {
        SortAttempt::Ordered(perm) => Ok(Some(perm)),
        SortAttempt::Undecided => Ok(None),
        SortAttempt::Malformed(e) => Err(e),
    }
}

pub fn op_sort(interp: &mut Interpreter) -> Result<()> {
    let is_keep_mode: bool = interp.consumption_mode == ConsumptionMode::Keep;

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
            // `expected` and `got` are the two halves of one sentence
            // ("Structure error: expected _, got _"), so each is a noun
            // phrase. A whole sentence here rendered as "expected SORT:
            // expected vector, got non-vector value, got other format" — the
            // Word's name belongs to the diagnosis locus, which already
            // carries it.
            return Err(AjisaiError::create_structure_error(
                "vector",
                "non-vector value",
            ));
        }
    };

    if children.is_empty() {
        interp.stack.push(Value::from_vector(Vec::new()));
        return Ok(());
    }

    // Priced before the sort runs: the comparison count is `n⌈log₂n⌉` at worst
    // and does not depend on the data, so this is a pre-charge in the same
    // sense the arithmetic meter's is.
    if let Err(e) = crate::interpreter::collection_meter::charge_comparison_sort(interp, &children)
    {
        if !is_keep_mode {
            interp.stack.push(val);
        }
        return Err(e);
    }

    match try_sort_indices(&children) {
        SortAttempt::Ordered(perm) => {
            let sorted_v: Vec<Value> = reorder_values_by_permutation(&children, &perm);
            interp.stack.push(Value::from_vector(sorted_v));
            Ok(())
        }
        SortAttempt::Undecided => {
            interp.stack.push(undecidable_nil());
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
            SortAttempt::Undecided => panic!("expected decidable sort"),
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
