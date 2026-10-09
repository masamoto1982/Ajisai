use crate::error::{AjisaiError, Result};
use crate::interpreter::comparison::three_way_compare;
use crate::interpreter::Interpreter;
use crate::types::{Value, ValueData};

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
/// comparison (LANG.VALUES.EXACT) decides every pair of Scalars but one:
/// `0/0` has no order, so a Vector holding it has no sorted form, and the
/// answer is `None` — the projection `SORT` and `ORDER` make for it.
/// `Err(_)` is malformed use (a structurally non-comparable element,
/// LANG.FAILURE.ERROR).
pub(crate) fn order_indices(items: &[Value]) -> Result<Option<Vec<usize>>> {
    // Whether a pair orders depends only on whether each of its elements is an
    // orderable Scalar, so one pass comparing each element with itself finds
    // the first malformed one, and the first `0/0`, before the sort runs. The
    // sort's comparator must be a total order — the standard sort panics on
    // one that reports `Equal` for a pair it could not order — and after this
    // pass it cannot fail.
    for item in items {
        if compare_for_sort(item, item)?.is_none() {
            return Ok(None);
        }
    }

    let mut perm: Vec<usize> = (0..items.len()).collect();
    perm.sort_by(|&i, &j| {
        compare_for_sort(&items[i], &items[j])
            .ok()
            .flatten()
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(Some(perm))
}

/// `three_way_compare`, with an operand the exact order is not defined on
/// raised as `nonNumeric`: the order is an order of Scalars, and every Word
/// that asks for one (LT/GT, MIN/MAX, SORT/ORDER/BSEARCH) names the fault the
/// same way. `None` for a pair holding `0/0`, which the Word projects.
pub(super) fn compare_for_sort(a: &Value, b: &Value) -> Result<Option<std::cmp::Ordering>> {
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
/// Declines for anything but a flat, pure-integer buffer: a rational lane
/// still sorts by value rather than by numerator, rank above 1 sorts *rows*,
/// and an empty buffer must answer with the empty `Vector` the route below
/// pushes.
fn dense_integer_sort(interp: &mut Interpreter, value: &Value) -> Result<Option<()>> {
    let ValueData::Tensor { data, shape } = &value.data else {
        return Ok(None);
    };
    if shape.len() != 1 || !data.is_pure_integer || data.is_empty() {
        return Ok(None);
    }

    // Priced before the sort runs, in the same units and at the same point the
    // comparison route prices it — see `charge_comparison_sort_of`. Nothing has
    // been consumed yet, so a refusal leaves the caller's restore path intact.
    crate::interpreter::collection_meter::charge_comparison_sort_of(interp, value)?;

    let mut sorted: Vec<i64> = data.numerators.to_vec();
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
        Ok(Some(perm)) => {
            let sorted_v: Vec<Value> = reorder_values_by_permutation(&children, &perm);
            interp.stack.push(Value::from_vector(sorted_v));
            Ok(())
        }
        Ok(None) => {
            interp
                .stack
                .push(crate::interpreter::comparison::unordered_projection());
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
        order_indices(items)
            .unwrap_or_else(|e| panic!("unexpected malformed: {e}"))
            .expect("an orderable vector")
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

#[cfg(test)]
mod sort_word_tests {
    //! What `SORT` answers, and in what representation.
    //!
    //! A flat pure-integer dense buffer sorts by sorting its numerator column: every
    //! comparison decides (nothing there is non-comparable), and equal integers are
    //! indistinguishable, so the stability of the permutation sort the comparison
    //! route runs is not observable. Everything else keeps that route, and the tests
    //! below pin both halves of that split — the fast one for its answers, the slow
    //! one for the behaviour it must not lose.
    //!
    //! The pricing half of the same change lives in `collection_meter_tests`, whose
    //! subject is a charge that must not turn on a representation decision.

    use crate::interpreter::Interpreter;
    use crate::test_support::{equals, top_is_dense};

    #[tokio::test]
    async fn a_dense_integer_sort_stays_dense() {
        for source in [
            "[ 5 3 9 1 7 2 8 4 ] SORT",
            "0 7 RANGE REVERSE SORT",
            "[ 7 ] SORT",
        ] {
            assert!(
                top_is_dense(source).await,
                "`{source}` must keep its result in columns"
            );
        }
    }

    #[tokio::test]
    async fn a_dense_integer_sort_orders_ascending() {
        for (sorted, expected) in [
            ("[ 5 3 9 1 7 2 8 4 ] SORT", "[ 1 2 3 4 5 7 8 9 ]"),
            ("0 7 RANGE REVERSE SORT", "0 7 RANGE"),
            // Negatives: sorting the numerator column must respect sign, not
            // magnitude.
            ("[ 3 -1 0 -5 2 ] SORT", "[ -5 -1 0 2 3 ]"),
            // Duplicates are kept, not collapsed — that is `UNIQUE`'s job.
            ("[ 3 1 3 1 2 ] SORT", "[ 1 1 2 3 3 ]"),
            ("[ 7 ] SORT", "[ 7 ]"),
            // Already ascending, and fully descending: the two ends of the
            // input space a sorted-run detector treats differently.
            ("[ 1 2 3 4 ] SORT", "[ 1 2 3 4 ]"),
            ("[ 4 3 2 1 ] SORT", "[ 1 2 3 4 ]"),
        ] {
            assert_eq!(
                equals(sorted, expected).await,
                Some(true),
                "`{sorted}` must equal `{expected}`"
            );
        }
    }

    /// Sorting is idempotent, and agrees with the comparison route on the same
    /// integers held nested — `CONCAT` with a boxed operand (the empty literal)
    /// does not promote, so the right-hand side takes the route the dense one
    /// declines.
    #[tokio::test]
    async fn the_two_routes_answer_alike() {
        assert_eq!(
            equals(
                "0 99 RANGE REVERSE SORT",
                "0 49 RANGE 50 99 RANGE CONCAT [ ] CONCAT SORT"
            )
            .await,
            Some(true),
            "a dense sort and a nested sort of the same integers must agree"
        );
        assert_eq!(
            equals("[ 5 3 9 1 ] SORT SORT", "[ 5 3 9 1 ] SORT").await,
            Some(true),
            "sorting a sorted buffer must change nothing"
        );
    }

    /// A rational lane sorts by *value*, not by numerator, so it must not reach
    /// the column sort. `1/3 2/3 1 4/3` descending back to ascending is the case
    /// that would break if numerators were compared directly: `4 3 2 1` as
    /// numerators is descending while the values ascend.
    #[tokio::test]
    async fn rational_lanes_keep_the_comparison_route_and_sort_by_value() {
        assert_eq!(
            equals(
                "1 4 RANGE [ 3 DIV ] MAP REVERSE SORT",
                "1 4 RANGE [ 3 DIV ] MAP"
            )
            .await,
            Some(true),
            "rationals must sort by value"
        );
    }

    /// The inputs the comparison route refuses must still be refused, with the
    /// declared condition `SORT` names in `spec/words.json` — an absent lane and
    /// a non-numeric element are not orderable, and rank-2 rows are not scalars.
    #[tokio::test]
    async fn non_orderable_inputs_are_still_refused_as_declared() {
        for source in [
            "[ 3 NIL 1 ] SORT",
            "[ [ 3 4 ] [ 1 2 ] ] SORT",
            "[ 'b' 'a' ] SORT",
        ] {
            let mut interp = Interpreter::new();
            let result = interp.execute(source).await;
            let error = result.expect_err(&format!("`{source}` must be refused"));
            assert!(
                format!("{error:?}").contains("nonNumeric"),
                "`{source}` must be refused as nonNumeric, got: {error:?}"
            );
        }
    }

    /// A non-orderable element among enough others to reach the sort's
    /// non-trivial paths must still be refused as declared, with the operand
    /// restored: a comparator that reports `Equal` for a pair it cannot order
    /// is not a total order, and the standard sort panics on one rather than
    /// returning.
    #[tokio::test]
    async fn a_non_orderable_element_in_a_long_vector_is_refused_not_a_panic() {
        let ints = "582 867 821 782 64 261 507 779 460 483 667 388 214 96 499 29 914 855 443 622";
        for (word, odd) in [
            ("SORT", "'a'"),
            ("ORDER", "NIL"),
            ("SORT", "NIL"),
            ("ORDER", "'a'"),
        ] {
            let items: Vec<&str> = ints
                .split(' ')
                .enumerate()
                .flat_map(|(i, n)| if i % 3 == 0 { vec![odd, n] } else { vec![n] })
                .collect();
            let source = format!("[ {} ] {word}", items.join(" "));
            let mut interp = Interpreter::new();
            let error = interp
                .execute(&source)
                .await
                .expect_err(&format!("`{source}` must be refused"));
            assert!(
                format!("{error:?}").contains("nonNumeric"),
                "`{source}` must be refused as nonNumeric, got: {error:?}"
            );
            assert_eq!(
                interp.get_stack().len(),
                1,
                "`{source}` must restore its operand"
            );
        }
    }
}
