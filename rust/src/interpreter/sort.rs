use crate::error::{AjisaiError, Result};
use crate::interpreter::comparison_scalar::three_way_compare;
use crate::interpreter::Interpreter;
use crate::types::{Value, ValueData};
use std::cell::RefCell;

fn reorder_values_by_permutation(source: &[Value], perm: &[usize]) -> Vec<Value> {
    perm.iter()
        .map(|&orig_idx| source[orig_idx].clone())
        .collect::<Vec<Value>>()
}

/// The ascending, stable index permutation of `items` — `ORDER`'s answer, and
/// the permutation `SORT` applies to produce its own.
///
/// Shared so the two Words cannot disagree about an ordering: `xs ORDER` and
/// `xs SORT` are the same comparison sequence, read two ways. The exact
/// comparison (LANG.VALUES.EXACT) decides every pair; `Err(_)` is malformed
/// use (a structurally non-comparable element, LANG.FAILURE.ERROR).
pub(crate) fn order_indices(items: &[Value]) -> Result<Vec<usize>> {
    // Captured by the comparator: the first malformed error. When it is set
    // the produced permutation is discarded, so returning `Equal` from the
    // comparator in that case is harmless to correctness.
    let malformed: RefCell<Option<AjisaiError>> = RefCell::new(None);

    let mut perm: Vec<usize> = (0..items.len()).collect();
    perm.sort_by(|&i, &j| match compare_for_sort(&items[i], &items[j]) {
        Ok(ord) => ord,
        Err(e) => {
            let mut slot = malformed.borrow_mut();
            if slot.is_none() {
                *slot = Some(e);
            }
            std::cmp::Ordering::Equal
        }
    });

    match malformed.into_inner() {
        Some(e) => Err(e),
        None => Ok(perm),
    }
}

/// `three_way_compare`, with an operand the exact order is not defined on
/// raised as `nonNumeric`: the order is an order of Scalars, and every Word
/// that asks for one (LT/GT, MIN/MAX, SORT/ORDER/BSEARCH) names the fault the
/// same way.
pub(super) fn compare_for_sort(a: &Value, b: &Value) -> Result<std::cmp::Ordering> {
    three_way_compare(a, b).map_err(|e| {
        AjisaiError::declared(
            "nonNumeric",
            format!("expected Scalar elements, got {}", e.got),
        )
    })
}

/// Sort a flat pure-integer dense buffer by sorting its numerator column.
///
/// `Some(())` when this route ran and pushed the result; `None` when the value
/// is any other shape and the comparison sort below must handle it. An `Err` is
/// that sort's own error — the charge — raised before anything is consumed.
///
/// The comparison sort materializes a `Tensor` into one boxed `Value` per lane
/// and then orders a *permutation* of indices, calling the exact comparison
/// through two `Value` derefs per probe. None of that is needed to order
/// machine integers: nothing here is non-comparable, so the sort always
/// succeeds, and equal integers are
/// indistinguishable, so the stability the permutation sort provides is not
/// observable. Sorting 262,144 of them cost 86 ms.
///
/// Declines for anything but a flat, all-present, pure-integer buffer: a
/// rational lane still sorts by value rather than by numerator, an absent lane
/// raises the question of where NIL orders, rank above 1 sorts *rows*, and an
/// empty buffer must answer with the empty `Vector` the route below pushes.
fn dense_integer_sort(interp: &mut Interpreter, value: &Value) -> Result<Option<()>> {
    let ValueData::Tensor { data, shape } = &value.data else {
        return Ok(None);
    };
    if shape.len() != 1 || !data.is_pure_integer || !data.all_lanes_valid() || data.is_empty() {
        return Ok(None);
    }

    // Priced before the sort runs, in the same units and at the same point the
    // comparison route prices it — see `charge_comparison_sort_of`. Nothing has
    // been consumed yet, so a refusal leaves the caller's restore path intact.
    crate::interpreter::collection_meter::charge_comparison_sort_of(interp, value)?;

    let mut sorted: Vec<i64> = data.numerators.clone();
    sorted.sort_unstable();
    interp.stack.push(Value::from_int_tensor(sorted));
    Ok(Some(()))
}

pub fn op_sort(interp: &mut Interpreter) -> Result<()> {
    let val: Value = interp.stack.pop().ok_or(AjisaiError::stack_underflow())?;

    match dense_integer_sort(interp, &val) {
        Ok(Some(())) => return Ok(()),
        Ok(None) => {}
        Err(e) => {
            interp.stack.push(val);
            return Err(e);
        }
    }

    // VTU Phase III boundary helper: as_vector_view() borrows for
    // Vector/Record and materializes once for Tensor, collapsing the
    // old representation-juggling.
    let children = match val.as_vector_view() {
        Some(view) => view,
        None => {
            let got = val.domain_name();
            interp.stack.push(val);
            return Err(AjisaiError::declared(
                "nonVector",
                format!("expected a Vector, got {got}"),
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
        interp.stack.push(val);
        return Err(e);
    }

    match order_indices(&children) {
        Ok(perm) => {
            let sorted_v: Vec<Value> = reorder_values_by_permutation(&children, &perm);
            interp.stack.push(Value::from_vector(sorted_v));
            Ok(())
        }
        Err(e) => {
            interp.stack.push(val);
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
        order_indices(items).unwrap_or_else(|e| panic!("unexpected malformed: {e}"))
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
        assert!(order_indices(&items).is_err());
    }
}
