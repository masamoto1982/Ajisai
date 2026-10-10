//! The search Word `BSEARCH` (LANG.VALUES.VECTOR).
//!
//! `BSEARCH` answers a question a first match over `GET` and `EQ` answers one
//! probe at a time, and earns its slot on cost: it halves a range until it is
//! empty — a loop whose length depends on the data, which a language with no
//! unbounded loop cannot write at all.

use super::sort::compare_for_sort;
use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::collection_meter;
use crate::interpreter::value_extraction_helpers::extract_operands;
use crate::interpreter::Interpreter;
use crate::semantic::Recoverability;
use crate::types::{Value, ValueData};

fn non_vector(got: &Value) -> AjisaiError {
    AjisaiError::declared(
        "nonVector",
        format!("expected a Vector, got {}", got.domain_name()),
    )
}

/// The elements of a Vector operand, read one at a time.
///
/// A flat dense Tensor is read lane by lane, on demand. These Words used to
/// materialize every lane as a boxed `Value` before looking at the first one
/// (`as_vector_view`), so `BSEARCH` paid for a million boxes to probe twenty
/// of them. A lane read here is the lane the materialization would have built (`Value::from_dense_lane`, its one definition), so every
/// comparison sees the same value either way.
///
/// The price is the same too: [`collection_meter::element_cost`] reads a
/// flat dense Tensor's width in O(1) and reaches the units the per-lane
/// reading reaches
/// (`search_meter_parity_tests`), so which route ran stays unobservable
/// (LANG.AUTHORITY.FREEDOM). A higher-rank Tensor's element is a row, not a
/// lane, and keeps the materializing route.
enum Elements<'a> {
    Boxed(std::borrow::Cow<'a, [Value]>),
    Lanes(&'a Value, &'a crate::types::DenseTensor),
}

impl<'a> Elements<'a> {
    fn of(value: &'a Value) -> Option<Self> {
        match &value.data {
            ValueData::Tensor { data, shape } if shape.len() == 1 => {
                Some(Elements::Lanes(value, data))
            }
            _ => value.as_vector_view().map(Elements::Boxed),
        }
    }

    fn len(&self) -> usize {
        match self {
            Elements::Boxed(items) => items.len(),
            Elements::Lanes(_, data) => data.len(),
        }
    }

    fn get(&self, index: usize) -> std::borrow::Cow<'_, Value> {
        match self {
            Elements::Boxed(items) => std::borrow::Cow::Borrowed(&items[index]),
            Elements::Lanes(_, data) => {
                std::borrow::Cow::Owned(Value::from_dense_lane(data, index))
            }
        }
    }

    fn element_cost(&self) -> crate::interpreter::runtime_limits::ElementCost {
        match self {
            Elements::Boxed(items) => collection_meter::element_cost_of_slice(items),
            Elements::Lanes(value, _) => collection_meter::element_cost(value),
        }
    }

    /// Whether the elements ascend, compared as `SORT` compares them. A flat
    /// pure-integer Tensor compares its numerator column: every lane is an
    /// integer there, and `compare_for_sort` orders integers as integers, so
    /// the column answers what the pairwise comparison would. `None` when an
    /// element is `0/0`, which has no order to ascend in.
    fn check_ascending(&self) -> Result<Option<bool>> {
        if let Elements::Lanes(_, data) = self {
            if data.is_pure_integer {
                return Ok(Some(
                    data.numerators.windows(2).all(|pair| pair[0] <= pair[1]),
                ));
            }
        }
        for index in 0..self.len() {
            let element = self.get(index);
            let previous = if index == 0 {
                self.get(index)
            } else {
                self.get(index - 1)
            };
            let Some(ordering) = compare_for_sort(&previous, &element)? else {
                return Ok(None);
            };
            if ordering == std::cmp::Ordering::Greater {
                return Ok(Some(false));
            }
        }
        Ok(Some(true))
    }
}

/// What one binary search answered.
enum Found {
    At(usize),
    Absent,
    /// The key is `0/0`, which no order places (LANG.VALUES.EXACT).
    Unordered,
}

/// The first index in ascending `sorted` whose element equals `key`, by
/// halving. `compare_for_sort` decides; a structurally non-comparable key is
/// its `nonNumeric`, and a key with no order is `Unordered`.
fn lower_bound(sorted: &Elements<'_>, key: &Value) -> Result<Found> {
    let (mut lo, mut hi) = (0usize, sorted.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match compare_for_sort(&sorted.get(mid), key)? {
            Some(std::cmp::Ordering::Less) => lo = mid + 1,
            Some(_) => hi = mid,
            None => return Ok(Found::Unordered),
        }
    }
    if lo == sorted.len() {
        // An empty vector asks nothing of the key; a key with no order is
        // still answered as one, so `[ ] 0/0 BSEARCH` and `[ 1 ] 0/0 BSEARCH`
        // agree.
        return Ok(match compare_for_sort(key, key)? {
            Some(_) => Found::Absent,
            None => Found::Unordered,
        });
    }
    match compare_for_sort(&sorted.get(lo), key)? {
        Some(std::cmp::Ordering::Equal) => Ok(Found::At(lo)),
        Some(_) => Ok(Found::Absent),
        None => Ok(Found::Unordered),
    }
}

/// `BSEARCH ( [ sorted ] [ keys ] -> [ indices ] )`: the index of each key in
/// an ascending vector. The order is checked first — one pass — because a
/// binary search over unordered data would answer something rather than
/// nothing, and an unsorted operand is the program being wrong about its own
/// data (`unsortedInput`). A key that is not there is a `notFound` lane.
pub fn op_bsearch(interp: &mut Interpreter) -> Result<()> {
    let operands = extract_operands(interp, 2)?;
    let answer = bsearch_answer(interp, &operands[0], &operands[1]);
    match answer {
        Ok(answer) => {
            interp.stack.push(answer);
            Ok(())
        }
        Err(e) => {
            interp.stack.extend(operands);
            Err(e)
        }
    }
}

fn bsearch_answer(interp: &mut Interpreter, sorted: &Value, keys: &Value) -> Result<Value> {
    let Some(sorted) = Elements::of(sorted) else {
        return Err(non_vector(sorted));
    };

    // The order check walks the vector once; priced like a linear scan's miss.
    let units = sorted
        .element_cost()
        .probe()
        .saturating_mul(sorted.len() as u64);
    collection_meter::charge(interp, units)?;
    match sorted.check_ascending()? {
        Some(true) => {}
        Some(false) => {
            return Err(AjisaiError::declared(
                "unsortedInput",
                "expected an ascending Vector, got one that is not in order",
            ));
        }
        // A Vector holding `0/0` has no order, so nothing can be searched in
        // it: a well-formed operand outside the Word's domain.
        None => return Ok(crate::interpreter::comparison::unordered_projection()),
    }

    let lane = |found: Found| match found {
        Found::At(index) => Value::from_int(index as i64),
        Found::Absent => Value::nil_with_reason(NilReason::NotFound, Recoverability::Recoverable),
        Found::Unordered => crate::interpreter::comparison::unordered_projection(),
    };
    Ok(match keys.as_vector_view() {
        Some(keys) => {
            let mut lanes = Vec::with_capacity(keys.len());
            for key in keys.iter() {
                lanes.push(lane(lower_bound(&sorted, key)?));
            }
            Value::from_vector(lanes)
        }
        None => lane(lower_bound(&sorted, keys)?),
    })
}

#[cfg(test)]
mod tests {
    //! Behavioral probes for the search Words: the projections `BSEARCH` and
    //! `SEARCH` declare, and `BSEARCH`'s order check.

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
}
