//! Tier 2 (`PI`) comparison-budget exhaustion (LANG.VALUES.EXACT), the first
//! source-reachable witness of a genuinely undecidable comparison. Split out
//! from `nil_conformance_tests` to stay under the §14.1 file-size budget.
//!
//! `PI PI EQ`/`PI PI LT` (etc.) never separate: two independently-constructed
//! computable reals with numerically-identical enclosures run the full
//! refinement budget without deciding an order, deterministically starving —
//! no hand-computed digits required. This is the one witness every Word in
//! `PROJECTING_WORDS` gained through Tier 2's arrival (`nil_conformance_tests`).

use crate::interpreter::Interpreter;
use crate::types::Value;

async fn run_ok(code: &str) -> Vec<Value> {
    let mut interp = Interpreter::new();
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` unexpectedly errored: {e}"));
    interp.get_stack().to_vec()
}

/// `PI` itself: a Tier 2 computable real, decisive against a clearly-separated
/// rational on both sides of the relation.
#[tokio::test]
async fn pi_is_a_computable_real_decisive_against_separated_rationals() {
    for (code, expect) in [
        ("PI 3 GT", true),
        ("3 PI LT", true),
        ("PI 4 LT", true),
        ("4 PI GT", true),
        ("PI 3 EQ", false),
    ] {
        let stack = run_ok(code).await;
        assert_eq!(stack.len(), 1, "`{code}` must leave exactly one value");
        assert_eq!(stack[0].as_truth(), Some(expect), "`{code}` must decide");
    }
}

/// `EQ`/`NEQ`/`LT`/`LTE`/`GT`/`GTE`: a `PI PI` pair never separates, so the
/// comparison's budget exhausts and the result is the logical Unknown (U) —
/// a NIL tagged `TruthValue` so `truthValue()` reports `unknown`, not an
/// ordinary absence and never an error.
#[tokio::test]
async fn comparison_family_projects_undecidable_pi_pair_to_unknown() {
    for name in ["EQ", "NEQ", "LT", "LTE", "GT", "GTE"] {
        let code = format!("PI PI {name}");
        let stack = run_ok(&code).await;
        assert_eq!(stack.len(), 1, "`{code}` must leave exactly one value");
        assert!(stack[0].is_nil(), "`{code}` must produce NIL (UNKNOWN)");
        assert_eq!(
            stack[0].truth_value(),
            Some("unknown"),
            "`{code}`'s NIL result must observe as truthValue `unknown`"
        );
    }
}

/// `MIN`/`MAX`/`ABS`: their output domain is numeric, not truth, so an
/// undecidable `PI PI` pair projects to a plain NIL — no `TruthValue` hint.
#[tokio::test]
async fn selecting_words_project_undecidable_pi_pair_to_plain_nil() {
    for code in ["PI PI MIN", "PI PI MAX"] {
        let stack = run_ok(code).await;
        assert_eq!(stack.len(), 1, "`{code}` must leave exactly one value");
        assert!(stack[0].is_nil(), "`{code}` must produce NIL");
        assert!(
            !stack[0].is_truth_value(),
            "`{code}`'s NIL must not carry the TruthValue role"
        );
    }

    // MIN/MAX still decide over a decisive pair.
    let stack = run_ok("PI 4 MIN").await;
    assert!(!stack[0].is_nil(), "`PI 4 MIN` must decide");
}

/// `SORT`/`ORDER`: one undecidable pair anywhere in the vector leaves the
/// whole order unestablished, so both answer a plain NIL rather than a
/// partially-sorted vector or permutation.
#[tokio::test]
async fn ordering_words_project_undecidable_pi_pair_to_plain_nil() {
    for code in ["PI PI 2 COLLECT SORT", "PI PI 2 COLLECT ORDER"] {
        let stack = run_ok(code).await;
        assert_eq!(stack.len(), 1, "`{code}` must leave exactly one value");
        assert!(stack[0].is_nil(), "`{code}` must produce NIL");
        assert!(
            !stack[0].is_truth_value(),
            "`{code}`'s NIL must not carry the TruthValue role"
        );
    }

    // A decisive (Tier 0/1) vector still sorts normally.
    let stack = run_ok("3 1 2 3 COLLECT SORT").await;
    assert!(!stack[0].is_nil(), "a decisive vector must sort");
}

/// Allocation identity is not a semantic discriminant (LANG.AUTHORITY.FREEDOM),
/// and construction history cannot be read back out of a value
/// (LANG.VALUES.DENOTATION). One π bound and used twice is the same two π as
/// two freshly built ones, so every relation must answer them alike.
///
/// It did not: `Computable`'s `PartialEq` is pointer identity, so the shared
/// pair took `pairwise_eq`'s structural shortcut and `X X EQ` answered TRUE
/// while `X X GTE` — which equality entails — still answered UNKNOWN.
#[tokio::test]
async fn equality_reads_the_value_not_the_allocation() {
    for op in ["EQ", "NEQ", "LT", "LTE", "GT", "GTE"] {
        let fresh = run_ok(&format!("PI PI {op}")).await;
        let shared = run_ok(&format!("PI 'X' BIND X X {op}")).await;
        assert!(
            fresh[0].is_nil() && shared[0].is_nil(),
            "`{op}` must answer UNKNOWN for both a fresh and a shared π pair, \
             got fresh={:?} shared={:?}",
            fresh[0],
            shared[0]
        );
        assert_eq!(
            fresh[0].truth_value(),
            shared[0].truth_value(),
            "`{op}` must not split on how the two π were made"
        );
    }

    // The same holds one level down: a Vector carrying a Tier 2 element
    // cannot decide structurally either, and used to answer a flat FALSE.
    for code in [
        "PI 1 COLLECT PI 1 COLLECT EQ",
        "PI 'X' BIND X 1 COLLECT X 1 COLLECT EQ",
    ] {
        let stack = run_ok(code).await;
        assert!(stack[0].is_nil(), "`{code}` must answer UNKNOWN");
    }
}

/// Deferring to the budgeted comparison costs no precision where the pair
/// genuinely decides: only a pair the refinement cannot separate is UNKNOWN.
#[tokio::test]
async fn tier2_equality_still_decides_what_it_can() {
    for (code, expect) in [
        ("PI 3 EQ", false),
        ("PI 3 NEQ", true),
        // Disjoint domains are unequal whatever they carry.
        ("PI 'a' EQ", false),
        // A length difference settles a Vector pair without an element.
        ("PI 1 COLLECT PI 1 2 COLLECT EQ", false),
        // One decidedly-unequal element settles the Kleene conjunction.
        ("PI 1 COLLECT 5 1 COLLECT EQ", false),
    ] {
        let stack = run_ok(code).await;
        assert_eq!(stack[0].as_truth(), Some(expect), "`{code}` must decide");
    }
}
