//! Tier ≤ 1 decidability (SPEC §4.2.2, §7.4.1). The Tier 2 vocabulary
//! (`MATH@PI`, Phase 7) can now reach the `Starved` comparison outcome and the
//! logical `UNKNOWN`, but the *Tier ≤ 1* words must not: their comparisons stay
//! decidable regardless of budget. These tests pin that boundary — a sweep of
//! `SQRT`-based numeric programs stays free of Tier 2 values and U, the one
//! irrational constructor stays within Tier 1, and the type-level starvation
//! witness projects `Starved` onto U through the same comparison router.
//! The positive Tier 2 reach (`MATH@PI` → U) is covered in
//! `tier2_vocabulary_tests`.

use crate::interpreter::Interpreter;
use crate::types::exact::{Computable, ExactCmp, ExactReal, Water};
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

/// Whether a value is, or structurally contains, a Tier 2 payload.
fn contains_tier2(value: &Value) -> bool {
    match &value.data {
        ValueData::ExactScalar(ExactReal::Computable(_)) => true,
        ValueData::Vector(items) | ValueData::Record { pairs: items, .. } => {
            items.iter().any(contains_tier2)
        }
        _ => false,
    }
}

/// Numeric programs spanning the Tier ≤ 1 exact-real vocabulary: SQRT
/// construction, field arithmetic (including values the Gosper era could not
/// decide), rounding, and every comparison word under small explicit budgets.
/// None construct a Tier 2 value, so all stay decidable.
const VOCABULARY_SWEEP: &[&str] = &[
    "'math' IMPORT 2 SQRT",
    "'math' IMPORT 2 SQRT 3 SQRT ADD 5 SQRT MUL",
    "'math' IMPORT 1 2 SQRT ADD 2 SQRT 1 SUB MUL",
    "'math' IMPORT 2 SQRT 1 ADD 2 SQRT 1 ADD SUB 0 EQ",
    "'math' IMPORT 2 SQRT 1 ADD 2 SQRT 1 ADD 1 COMPARE-WITHIN",
    "'math' IMPORT 8 SQRT 2 SQRT 2 MUL 1 COMPARE-WITHIN",
    "'math' IMPORT 2 SQRT 3 SQRT 1 COMPARE-WITHIN",
    "'math' IMPORT 2 SQRT 2 LT",
    "'math' IMPORT 2 SQRT 2 SQRT EQ",
    "'math' IMPORT 2 SQRT FLOOR",
    "'math' IMPORT 2 SQRT NEG ROUND",
    "'math' IMPORT 2 SQRT 0 MAX",
    "'math' IMPORT 2 SQRT 3 SQRT MIN",
    "'algo' IMPORT 'math' IMPORT [ 3 1 2 ] SORT",
];

#[tokio::test]
async fn tier1_vocabulary_stays_decidable() {
    for src in VOCABULARY_SWEEP {
        let mut interp = Interpreter::new();
        interp
            .execute(src)
            .await
            .unwrap_or_else(|e| panic!("`{src}` unexpectedly errored: {e}"));
        for value in interp.get_stack() {
            assert!(
                !contains_tier2(value),
                "`{src}` must not produce a Tier 2 value, got {value:?}"
            );
            assert!(
                !value.is_unknown(),
                "`{src}` must not produce the logical UNKNOWN, got {value:?}"
            );
        }
    }
}

#[test]
fn sqrt_constructor_stays_within_tier1() {
    // The one irrational-producing constructor projects onto Tier 0/1 only.
    for n in [0i64, 1, 2, 3, 4, 8, 9, 12, 49, 50] {
        match ExactReal::from_sqrt_rational(Fraction::from(n)) {
            Some(ExactReal::Computable(_)) => {
                panic!("from_sqrt_rational must never build a Tier 2 value")
            }
            Some(_) => {}
            None => panic!("√{n} is well-defined for non-negative n"),
        }
    }
}

#[test]
fn starved_projection_remains_wired_for_tier2() {
    // The counterpart guarantee at the type level: the Starved arm reports its
    // consumed water and a separable Tier 2 value still decides. This is the
    // router the `MATH@PI` vocabulary reaches through.
    let witness = ExactReal::Computable(Computable::vanishing());
    let zero = ExactReal::from_fraction(Fraction::from(0i64));
    assert_eq!(
        witness.cmp_within(&zero, Water(8)),
        ExactCmp::Starved { steps: 8 }
    );
    // A separable Tier 2 value still decides: the witness is below 1.
    let one = ExactReal::from_fraction(Fraction::from(1i64));
    assert_eq!(
        witness.cmp_within(&one, Water(8)),
        ExactCmp::Decided(std::cmp::Ordering::Less)
    );
}
