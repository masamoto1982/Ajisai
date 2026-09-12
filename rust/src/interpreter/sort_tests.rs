//! What `SORT` answers, and in what representation.
//!
//! A flat pure-integer dense buffer sorts by sorting its numerator column: every
//! comparison decides (nothing there is a Tier 2 real that could exhaust its
//! refinement budget, and nothing is non-comparable), and equal integers are
//! indistinguishable, so the stability of the permutation sort the comparison
//! route runs is not observable. Everything else keeps that route, and the tests
//! below pin both halves of that split — the fast one for its answers, the slow
//! one for the behaviour it must not lose.
//!
//! The pricing half of the same change lives in `collection_meter_tests`, whose
//! subject is a charge that must not turn on a representation decision.

#[cfg(test)]
mod sort_tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    async fn run(source: &str) -> Interpreter {
        let mut interp = Interpreter::new();
        let mut limits = *interp.runtime_limits();
        limits.max_materialized_elements = 10_000_000;
        interp.set_runtime_limits(limits);
        interp
            .execute(source)
            .await
            .unwrap_or_else(|e| panic!("`{source}` must compute, got: {e:?}"));
        interp
    }

    async fn top_is_dense(source: &str) -> bool {
        let interp = run(source).await;
        matches!(
            interp.get_stack().as_slice().last().map(|v| &v.data),
            Some(ValueData::Tensor { .. })
        )
    }

    async fn equals(left: &str, right: &str) -> Option<bool> {
        let interp = run(&format!("{left} {right} EQ")).await;
        interp.get_stack().as_slice().last()?.as_truth()
    }

    #[tokio::test]
    async fn a_dense_integer_sort_stays_dense() {
        for source in [
            "[ 5 3 9 1 7 2 8 4 ] SORT",
            "[ 0 7 ] RANGE REVERSE SORT",
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
            ("[ 0 7 ] RANGE REVERSE SORT", "[ 0 7 ] RANGE"),
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
    /// integers held nested — `CONCAT` does not promote, so the right-hand side
    /// takes the route the dense one declines.
    #[tokio::test]
    async fn the_two_routes_answer_alike() {
        assert_eq!(
            equals(
                "[ 0 99 ] RANGE REVERSE SORT",
                "[ 0 49 ] RANGE [ 50 99 ] RANGE CONCAT SORT"
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
                "[ 1 4 ] RANGE [ 3 DIV ] MAP REVERSE SORT",
                "[ 1 4 ] RANGE [ 3 DIV ] MAP"
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
                format!("{error:?}").contains("nonComparableElement"),
                "`{source}` must be refused as nonComparableElement, got: {error:?}"
            );
        }
    }
}
