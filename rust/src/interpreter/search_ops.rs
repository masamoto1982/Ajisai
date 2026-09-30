//! Search Words: `INDEX-OF`, `MEMBER` and `BSEARCH` (LANG.VALUES.VECTOR).
//!
//! `MEMBER` and `BSEARCH` answer a question `INDEX-OF` already answers one
//! probe at a time, and both earn their slot on cost (docs/dev/vocabulary-100-work-order-2026-09.md
//! §1): `MEMBER` indexes the vector once instead of scanning it once per probe,
//! and `BSEARCH` halves a range until it is empty — a loop whose length depends
//! on the data, which a language with no unbounded loop cannot write at all.

use super::sort::compare_for_sort;
use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::collection_meter::{self, ScanMeter};
use crate::interpreter::value_extraction_helpers::extract_operands;
use crate::interpreter::Interpreter;
use crate::semantic::Recoverability;
use crate::types::Value;

fn non_vector(got: &Value) -> AjisaiError {
    AjisaiError::declared(
        "nonVector",
        format!("expected a Vector, got {}", got.domain_name()),
    )
}

/// `MEMBER? ( [ vec ] [ x ] -> [ TRUE | FALSE ] )`: whether `x` occurs in the
/// vector, by the value equality `UNIQUE` and `INDEX-OF` use. The needle is
/// one value compared, not read (an `element` operand), so a Vector needle is
/// looked for as an element rather than taken as several needles.
pub fn op_member(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let Some(elements) = operands[0].as_vector_view().map(|view| view.into_owned()) else {
        let err = non_vector(&operands[0]);
        interp.stack.extend(operands);
        return Err(err);
    };

    let meter = ScanMeter::new(&elements);
    for (completed, item) in elements.iter().enumerate() {
        if let Err(e) = meter.charge_scan_of(interp, completed) {
            interp.stack.extend(operands);
            return Err(e);
        }
        if *item == operands[1] {
            interp.stack.push(Value::from_bool(true));
            return Ok(());
        }
    }
    interp.stack.push(Value::from_bool(false));
    Ok(())
}

/// What one binary search answered.
enum Found {
    At(usize),
    Absent,
}

/// The first index in ascending `sorted` whose element equals `key`, by
/// halving. `compare_for_sort` decides; a structurally non-comparable key is
/// its `nonNumeric`.
fn lower_bound(sorted: &[Value], key: &Value) -> Result<Found> {
    let (mut lo, mut hi) = (0usize, sorted.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match compare_for_sort(&sorted[mid], key)? {
            std::cmp::Ordering::Less => lo = mid + 1,
            _ => hi = mid,
        }
    }
    if lo == sorted.len() {
        return Ok(Found::Absent);
    }
    match compare_for_sort(&sorted[lo], key)? {
        std::cmp::Ordering::Equal => Ok(Found::At(lo)),
        _ => Ok(Found::Absent),
    }
}

/// `BSEARCH ( [ sorted ] [ keys ] -> [ indices ] )`: the index of each key in
/// an ascending vector. The order is checked first — one pass — because a
/// binary search over unordered data would answer something rather than
/// nothing, and an unsorted operand is the program being wrong about its own
/// data (`unsortedInput`). A key that is not there is a `notFound` lane.
pub fn op_bsearch(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let Some(sorted) = operands[0].as_vector_view().map(|view| view.into_owned()) else {
        let err = non_vector(&operands[0]);
        interp.stack.extend(operands);
        return Err(err);
    };

    // The order check walks the vector once; priced like INDEX-OF's miss.
    let units = collection_meter::element_cost_of_slice(&sorted)
        .probe()
        .saturating_mul(sorted.len() as u64);
    if let Err(e) = collection_meter::charge(interp, units) {
        interp.stack.extend(operands);
        return Err(e);
    }
    for pair in sorted.windows(2) {
        match compare_for_sort(&pair[0], &pair[1]) {
            Ok(std::cmp::Ordering::Greater) => {
                interp.stack.extend(operands);
                return Err(AjisaiError::declared(
                    "unsortedInput",
                    "expected an ascending Vector, got one that is not in order",
                ));
            }
            Ok(_) => {}
            Err(e) => {
                interp.stack.extend(operands);
                return Err(e);
            }
        }
    }

    let lane = |found: Found| match found {
        Found::At(index) => Value::from_int(index as i64),
        Found::Absent => Value::nil_with_reason(NilReason::NotFound, Recoverability::Recoverable),
    };
    let answer = match operands[1].as_vector_view() {
        Some(keys) => {
            let mut lanes = Vec::with_capacity(keys.len());
            for key in keys.iter() {
                match lower_bound(&sorted, key) {
                    Ok(found) => lanes.push(lane(found)),
                    Err(e) => {
                        interp.stack.extend(operands);
                        return Err(e);
                    }
                }
            }
            Value::from_vector(lanes)
        }
        None => match lower_bound(&sorted, &operands[1]) {
            Ok(found) => lane(found),
            Err(e) => {
                interp.stack.extend(operands);
                return Err(e);
            }
        },
    };
    interp.stack.push(answer);
    Ok(())
}

fn pop_vector_and_target(interp: &mut Interpreter, _word: &str) -> Result<(Vec<Value>, Value)> {
    let operands = extract_operands(interp, 2)?;
    match operands[0].as_vector_view() {
        Some(view) => {
            let vector = view.into_owned();
            Ok((vector, operands[1].clone()))
        }
        None => {
            let got = operands[0].domain_name();
            interp.stack.extend(operands);
            // A noun phrase, not a sentence: the template around it already
            // says "expected _, got _", and the failing Word's name is the
            // diagnosis locus rather than part of the message.
            Err(AjisaiError::declared(
                "nonVector",
                format!("expected a Vector, got {got}"),
            ))
        }
    }
}

/// `vector value -- index`. Index of the first element equal to the target.
/// A well-formed miss (value absent from a valid vector) projects to
/// NIL with `reason = notFound` per the NIL Projection Rule.
pub fn op_index_of(interp: &mut Interpreter) -> Result<()> {
    let (vector, target) = pop_vector_and_target(interp, "INDEX-OF")?;
    // A linear search, priced at its worst case — the miss, which is the only
    // outcome that has to walk the whole vector. The count is known before the
    // scan starts, unlike the distinct-value scans, so this is a pre-charge.
    let units = crate::interpreter::collection_meter::element_cost_of_slice(&vector)
        .probe()
        .saturating_mul(vector.len() as u64);
    if let Err(e) = crate::interpreter::collection_meter::charge(interp, units) {
        interp.stack.extend([Value::from_vector(vector), target]);
        return Err(e);
    }
    match vector.iter().position(|elem| elem == &target) {
        Some(index) => {
            interp.stack.push(Value::from_int(index as i64));
        }
        None => {
            interp.stack.push(Value::nil_with_reason(
                NilReason::NotFound,
                Recoverability::Recoverable,
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Behavioral probes for the search Words: `INDEX-OF`'s position and its
    //! miss, the projections `BSEARCH` and `SEARCH` declare, `BSEARCH`'s order
    //! check, and `MEMBER` over every domain.

    use crate::interpreter::Interpreter;
    use crate::test_support::{reason, top};

    async fn raises(code: &str, naming: &str) {
        let mut interp = Interpreter::new();
        let message = interp
            .execute(code)
            .await
            .expect_err(&format!("`{code}` must raise"))
            .to_string();
        assert!(
            message.contains(naming),
            "`{code}` must raise naming `{naming}`, got: {message}"
        );
    }

    #[tokio::test]
    async fn member_answers_lane_for_lane_over_any_domain() {
        for (code, want) in [
            ("[ 1 2 3 ] 2 MEMBER?", "TRUE"),
            ("[ 1 2 3 ] 5 MEMBER?", "FALSE"),
            ("[ 'a' 'b' ] 'b' MEMBER?", "TRUE"),
            ("[ [ 1 2 ] [ 3 ] ] [ 3 ] MEMBER?", "TRUE"),
            ("[ [ 1 2 ] [ 3 ] ] [ 1 ] MEMBER?", "FALSE"),
            ("[ ] 1 MEMBER?", "FALSE"),
            ("[ 1 2 ] [ ] MEMBER?", "FALSE"),
        ] {
            assert_eq!(top(code).await, want, "`{code}`");
        }
    }

    #[tokio::test]
    async fn bsearch_answers_the_first_index_and_projects_an_absent_key() {
        for (code, want) in [
            ("[ 1 3 5 7 ] [ 5 ] BSEARCH", "[ 2/1 ]"),
            ("[ 1 3 5 7 ] 5 BSEARCH", "2/1"),
            ("[ 1 3 3 3 7 ] 3 BSEARCH", "1/1"),
            ("[ 1 3 5 7 ] [ 1 7 ] BSEARCH", "[ 0/1 3/1 ]"),
            ("[ 1/2 3/2 ] 3/2 BSEARCH", "1/1"),
        ] {
            assert_eq!(top(code).await, want, "`{code}`");
        }
        assert_eq!(
            reason("[ 1 3 5 7 ] 4 BSEARCH").await.as_deref(),
            Some("notFound")
        );
        assert_eq!(
            top("[ 1 3 5 7 ] [ 4 5 ] BSEARCH 0 GET NIL-REASON").await,
            "'notFound'"
        );
        assert_eq!(reason("[ ] 4 BSEARCH").await.as_deref(), Some("notFound"));
    }

    /// The order is checked before the search: an unsorted operand is the
    /// program being wrong.
    #[tokio::test]
    async fn bsearch_checks_the_order_first() {
        raises("[ 3 1 2 ] [ 2 ] BSEARCH", "BSEARCH").await;
        raises(
            "[ 1 'a' ] 1 BSEARCH",
            "expected Scalar elements, got String",
        )
        .await;
        raises(
            "[ 1 2 3 ] 'a' BSEARCH",
            "expected Scalar elements, got String",
        )
        .await;
    }

    #[tokio::test]
    async fn search_counts_characters_and_projects_an_absent_needle() {
        for (code, want) in [
            ("'hello world' 'world' SEARCH", "6/1"),
            ("'hello' 'l' SEARCH", "2/1"),
            ("'hello' '' SEARCH", "0/1"),
            ("'こんにちは' 'ち' SEARCH", "3/1"),
        ] {
            assert_eq!(top(code).await, want, "`{code}`");
        }
        assert_eq!(
            reason("'hello' 'z' SEARCH").await.as_deref(),
            Some("notFound")
        );
        raises("'hello' 1 SEARCH", "SEARCH").await;
    }

    #[tokio::test]
    async fn replace_substitutes_every_occurrence_without_overlap() {
        for (code, want) in [
            ("'a-b-c' '-' '+' REPLACE", "'a+b+c'"),
            ("'aaa' 'aa' 'b' REPLACE", "'ba'"),
            ("'hello' 'z' 'y' REPLACE", "'hello'"),
            ("'hello' '' 'x' REPLACE", "'hello'"),
            ("'hello' 'l' '' REPLACE", "'heo'"),
        ] {
            assert_eq!(top(code).await, want, "`{code}`");
        }
        raises("'a' 'b' 3 REPLACE", "REPLACE").await;
    }

    #[tokio::test]
    async fn index_of_returns_position() {
        let mut interp = Interpreter::new();
        interp
            .execute("[ 10 20 30 ] 20 INDEX-OF")
            .await
            .expect("should succeed");
        assert_eq!(interp.stack[0].as_scalar().unwrap().to_i64().unwrap(), 1);
    }

    #[tokio::test]
    async fn index_of_missing_projects_to_nil() {
        let mut interp = Interpreter::new();
        interp
            .execute("[ 10 20 30 ] 99 INDEX-OF")
            .await
            .expect("a search miss projects to NIL, not an error");
        assert_eq!(interp.stack.len(), 1);
        assert!(interp.stack[0].is_nil());
    }
}
