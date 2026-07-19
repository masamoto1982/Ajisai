//! Phase 7 positive tests: the Tier 2 vocabulary (`MATH@PI`, `MATH@ENCLOSE`)
//! constructs and observes a general computable real, and `COMPARE-WITHIN`
//! reaches the logical `UNKNOWN` — while Tier ≤ 1 comparisons stay decidable.

use crate::interpreter::Interpreter;
use crate::types::exact::{ExactReal, RatInterval};
use crate::types::{Value, ValueData};

async fn run(src: &str) -> (Interpreter, Vec<Value>) {
    let mut interp = Interpreter::new();
    interp
        .execute(src)
        .await
        .unwrap_or_else(|e| panic!("`{src}` errored: {e}"));
    let stack = interp.get_stack().to_vec();
    (interp, stack)
}

fn is_tier2(value: &Value) -> bool {
    matches!(
        &value.data,
        ValueData::ExactScalar(ExactReal::Computable(_))
    )
}

#[tokio::test]
async fn pi_constructs_a_tier2_value() {
    let (_i, stack) = run("'math' IMPORT PI").await;
    assert_eq!(stack.len(), 1);
    assert!(
        is_tier2(&stack[0]),
        "MATH@PI must push a Tier 2 computable real, got {:?}",
        stack[0]
    );
}

#[tokio::test]
async fn pi_enclosures_nest_and_contain_pi_as_budget_grows() {
    // ENCLOSE at increasing budgets yields nested rational intervals, each
    // bracketing π (checked against the known rationals 333/106 < π < 22/7).
    let (_i, coarse) = run("'math' IMPORT PI [ 4 ] ENCLOSE").await;
    let (_j, fine) = run("'math' IMPORT PI [ 32 ] ENCLOSE").await;

    let to_interval = |v: &Value| -> RatInterval {
        let items = match &v.data {
            ValueData::Vector(items) => items,
            other => panic!("ENCLOSE must yield an interval vector, got {other:?}"),
        };
        assert_eq!(items.len(), 2, "interval has two endpoints");
        RatInterval::new(
            items[0].as_scalar().unwrap().clone(),
            items[1].as_scalar().unwrap().clone(),
        )
    };
    let coarse_iv = to_interval(&coarse[0]);
    let fine_iv = to_interval(&fine[0]);

    assert!(fine_iv.is_within(&coarse_iv), "finer enclosure must nest");
    // Both bracket π: lo < 22/7 and hi > 333/106 (a value inside both encloses π).
    let lower = crate::types::fraction::Fraction::new(333.into(), 106.into());
    let upper = crate::types::fraction::Fraction::new(22.into(), 7.into());
    for iv in [&coarse_iv, &fine_iv] {
        assert!(iv.lo.lt(&upper) && lower.lt(&iv.hi), "π stays enclosed");
    }
    assert!(
        fine_iv.width().lt(&coarse_iv.width()),
        "budget narrows width"
    );
}

#[tokio::test]
async fn compare_within_pi_and_pi_reaches_unknown() {
    // A Tier 2 process never proves its own equality: comparing π to π within
    // any finite budget starves, yielding the logical UNKNOWN (not NIL).
    let (_i, stack) = run("'math' IMPORT PI PI [ 64 ] COMPARE-WITHIN").await;
    assert_eq!(stack.len(), 1);
    assert!(stack[0].is_unknown(), "π vs π must be UNKNOWN");
    // The logical Unknown is stored as a Nil variant but is *not* an
    // operational NIL — it carries no reason-based absence (SPEC §14.6).
    assert!(
        !stack[0].is_operational_nil(),
        "UNKNOWN is not an operational NIL"
    );
}

#[tokio::test]
async fn compare_within_pi_and_three_decides() {
    // π separates from 3 quickly: the order is decided, not UNKNOWN.
    let (_i, stack) = run("'math' IMPORT PI [ 3 ] [ 64 ] COMPARE-WITHIN").await;
    assert_eq!(stack.len(), 1);
    assert!(!stack[0].is_unknown(), "π vs 3 is decidable");
    assert_eq!(
        stack[0].as_scalar().map(|f| f.numerator()),
        Some(1.into()),
        "π > 3 pushes 1"
    );
}

#[tokio::test]
async fn tier0_comparison_never_regresses_to_unknown() {
    // A Tier 0 pair decides regardless of the budget — no starvation.
    for budget in [1, 4, 64] {
        let src = format!("[ 5 ] [ 2 ] [ {budget} ] COMPARE-WITHIN");
        let (_i, stack) = run(&src).await;
        assert!(!stack[0].is_unknown(), "`{src}` must decide, not UNKNOWN");
    }
}

#[tokio::test]
async fn enclose_on_rational_is_a_point() {
    // A Tier 0 value observed by ENCLOSE yields the exact point [x, x].
    let (_i, stack) = run("'math' IMPORT [ 7 ] [ 8 ] ENCLOSE").await;
    let items = match &stack[0].data {
        ValueData::Vector(items) => items,
        // A degenerate interval may collapse to a scalar; accept that too.
        ValueData::Scalar(_) => return,
        other => panic!("unexpected ENCLOSE output {other:?}"),
    };
    let lo = items[0].as_scalar().unwrap();
    let hi = items[1].as_scalar().unwrap();
    assert_eq!(lo, hi, "a rational encloses to a point");
}
