//! Tests for pure HOF kernel memoization (`memo.rs`).
//!
//! The contract is: memoization must never change an observable MAP result —
//! only how many times the kernel runs. The differential tests run each program
//! with memo ON and OFF and require byte-identical stacks; the engagement tests
//! pin the hit/miss/store counters; the invalidation test proves a redefinition
//! is never served a stale cached result.
//!
//! Kernels here are deliberately *not* single fast-unary forms (`[ c ] +`,
//! `ABS`, ...), because those take MAP's bulk-tensor fast path and bypass the
//! per-element loop the memo lives in. A two-op block such as `{ [ 2 ] * [ 1 ] + }`
//! is pure and quantized but not bulk-eligible, so it exercises the memo path.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    fn run_with_memo(code: &str, memo: bool) -> Interpreter {
        let mut interp = Interpreter::new();
        interp.set_hof_memo_enabled(memo);
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            interp.execute(code).await.expect("code should execute");
        });
        interp
    }

    /// The core invariant: memo ON and memo OFF must leave identical stacks.
    fn assert_memo_invariant(code: &str) {
        let on = run_with_memo(code, true);
        let off = run_with_memo(code, false);
        assert_eq!(
            format!("{:?}", on.get_stack()),
            format!("{:?}", off.get_stack()),
            "MAP result diverged between memo ON and OFF for: {code}"
        );
    }

    #[test]
    fn invariant_repeated_elements() {
        assert_memo_invariant("[ 3 3 3 5 ] { [ 2 ] * [ 1 ] + } MAP");
    }

    #[test]
    fn invariant_distinct_elements() {
        assert_memo_invariant("[ 1 2 3 4 ] { [ 2 ] * [ 1 ] + } MAP");
    }

    #[test]
    fn invariant_user_word_kernel() {
        // A pure kernel that calls a user word, over repeated elements.
        assert_memo_invariant("{ [ 10 ] * } 'TENX' DEF\n[ 2 2 3 ] { TENX [ 1 ] + } MAP");
    }

    #[test]
    fn invariant_division_by_zero_bubble() {
        // elem / 0 projects to NIL for every element; the NIL result must be
        // reproduced identically whether or not it was memoized.
        assert_memo_invariant("[ 2 2 4 ] { [ 0 ] / [ 1 ] + } MAP");
    }

    #[test]
    fn invariant_non_rational_elements_fall_through() {
        // Vector-of-vectors: each element is a collection, not a rational
        // scalar, so the memo never engages — but the result must be unchanged.
        assert_memo_invariant("[ [ 1 2 ] [ 1 2 ] ] { [ 2 ] * } MAP");
    }

    #[test]
    fn repeated_elements_produce_cache_hits() {
        // Elements 3,3,3,5: 3 -> miss+store, 3 -> hit, 3 -> hit, 5 -> miss+store.
        let interp = run_with_memo("[ 3 3 3 5 ] { [ 2 ] * [ 1 ] + } MAP", true);
        let m = interp.runtime_metrics();
        assert_eq!(
            m.hof_memo_hit_count, 2,
            "two repeats of element 3 should hit"
        );
        assert_eq!(m.hof_memo_miss_count, 2, "distinct elements 3 and 5 miss");
        assert_eq!(m.hof_memo_store_count, 2, "one store per distinct element");
    }

    #[test]
    fn distinct_elements_never_hit() {
        let interp = run_with_memo("[ 1 2 3 4 ] { [ 2 ] * [ 1 ] + } MAP", true);
        let m = interp.runtime_metrics();
        assert_eq!(m.hof_memo_hit_count, 0, "all-distinct elements never hit");
        assert_eq!(m.hof_memo_store_count, 4, "one store per distinct element");
    }

    #[test]
    fn disabled_memo_does_no_cache_work() {
        let interp = run_with_memo("[ 3 3 3 5 ] { [ 2 ] * [ 1 ] + } MAP", false);
        let m = interp.runtime_metrics();
        assert_eq!(m.hof_memo_hit_count, 0);
        assert_eq!(m.hof_memo_miss_count, 0);
        assert_eq!(m.hof_memo_store_count, 0);
    }

    #[test]
    fn redefinition_is_not_served_stale() {
        // Populate the cache with TENX = *10, then redefine TENX = *100. The
        // dictionary epoch bump flushes the result cache, so the second MAP must
        // reflect the new definition, never the cached *10 result.
        let mut interp = Interpreter::new();
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            interp.execute("{ [ 10 ] * } 'TENX' DEF").await.unwrap();
            interp
                .execute("[ 2 2 ] { TENX [ 1 ] + } MAP")
                .await
                .unwrap();
            // 2*10+1 = 21 for each element.
            let first = format!("{:?}", interp.get_stack());
            assert!(first.contains("21"), "expected 21 from *10 kernel: {first}");

            interp.execute("{ [ 100 ] * } 'TENX' DEF").await.unwrap();
            interp
                .execute("[ 2 2 ] { TENX [ 1 ] + } MAP")
                .await
                .unwrap();
            // 2*100+1 = 201; a stale hit would wrongly reproduce 21.
            let second = format!("{:?}", interp.get_stack());
            assert!(
                second.contains("201"),
                "redefinition must not be served a stale memo result: {second}"
            );
        });
    }

    // ── Predicate family: FILTER / ANY / ALL / COUNT ──────────────────────
    // These share one memoization site in `execute_hedged_predicate_kernel`.
    // `{ [ 1 ] <= NOT }` (elem > 1) is pure and quantized but not a single
    // fast-unary predicate, so it takes the per-element loop the memo lives in.

    #[test]
    fn predicate_filter_invariant() {
        assert_memo_invariant("[ 1 2 2 3 3 3 ] { [ 1 ] <= NOT } FILTER");
    }

    #[test]
    fn predicate_count_invariant() {
        assert_memo_invariant("[ 1 2 2 3 3 3 ] { [ 1 ] <= NOT } COUNT");
    }

    #[test]
    fn predicate_any_invariant() {
        assert_memo_invariant("[ 2 2 2 2 ] { [ 5 ] = } ANY");
    }

    #[test]
    fn predicate_all_invariant() {
        assert_memo_invariant("[ 3 3 3 5 ] { [ 1 ] <= NOT } ALL");
    }

    #[test]
    fn predicate_filter_produces_cache_hits() {
        // FILTER evaluates every element (no short-circuit). Elements 3,3,3,5:
        // 3 -> miss+store, 3 -> hit, 3 -> hit, 5 -> miss+store.
        let interp = run_with_memo("[ 3 3 3 5 ] { [ 1 ] <= NOT } FILTER", true);
        let m = interp.runtime_metrics();
        assert_eq!(
            m.hof_memo_hit_count, 2,
            "two repeats of element 3 should hit"
        );
        assert_eq!(m.hof_memo_miss_count, 2, "distinct elements 3 and 5 miss");
        assert_eq!(m.hof_memo_store_count, 2, "one store per distinct element");
    }

    #[test]
    fn predicate_disabled_does_no_cache_work() {
        let interp = run_with_memo("[ 3 3 3 5 ] { [ 1 ] <= NOT } FILTER", false);
        let m = interp.runtime_metrics();
        assert_eq!(m.hof_memo_hit_count, 0);
        assert_eq!(m.hof_memo_miss_count, 0);
        assert_eq!(m.hof_memo_store_count, 0);
    }

    #[test]
    fn predicate_redefinition_is_not_served_stale() {
        // FILTER keeps elements where TEN_X exceeds the threshold; after TEN_X is
        // redefined the epoch-bump flush must prevent a stale predicate result.
        let mut interp = Interpreter::new();
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async {
            // TEN_X x = x*10; predicate keeps elements whose *10 is > 25.
            interp.execute("{ [ 10 ] * } 'TEN_X' DEF").await.unwrap();
            interp
                .execute("[ 2 2 ] { TEN_X [ 25 ] <= NOT } FILTER")
                .await
                .unwrap();
            // 2*10=20, not > 25 -> both dropped -> NIL. Inspect only the top of
            // stack: the first result stays on the stack under the second.
            let first = format!("{:?}", interp.get_stack().last());
            assert!(first.contains("Nil"), "expected NIL (20 <= 25): {first}");

            interp.execute("{ [ 100 ] * } 'TEN_X' DEF").await.unwrap();
            interp
                .execute("[ 2 2 ] { TEN_X [ 25 ] <= NOT } FILTER")
                .await
                .unwrap();
            // 2*100=200 > 25 -> both kept; a stale predicate would wrongly drop.
            let second = format!("{:?}", interp.get_stack().last());
            assert!(
                !second.contains("Nil"),
                "redefinition must not be served a stale predicate result: {second}"
            );
        });
    }
}
