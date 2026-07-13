//! Tests for the §4.2.7 multiquadratic normal-form comparison: the six
//! relations must be total and exact over the admitted domain \(D\), so
//! every case below must *decide* — a `None`/`Undecided` outcome is the
//! bug this module exists to remove.

use crate::types::continued_fraction::{CmpOutcome, ExactReal, DEFAULT_COMPARISON_BUDGET};
use crate::types::fraction::Fraction;
use crate::types::multiquadratic::{algebraic_cmp, algebraic_is_zero};
use num_bigint::BigInt;
use std::cmp::Ordering;

fn rational(num: i64, den: i64) -> ExactReal {
    ExactReal::Rational(Fraction::new(BigInt::from(num), BigInt::from(den)))
}

fn sqrt_of(num: i64, den: i64) -> ExactReal {
    ExactReal::from_sqrt_rational(Fraction::new(BigInt::from(num), BigInt::from(den))).unwrap()
}

/// √2 + √3 built through the bihomographic Gosper path — the value whose
/// comparison against its own commuted twin used to exhaust the budget.
fn sqrt2_plus_sqrt3() -> ExactReal {
    sqrt_of(2, 1).add(&sqrt_of(3, 1))
}

#[test]
fn commuted_sum_of_radicals_is_equal() {
    // √2 + √3 = √3 + √2, two distinct Gosper trees for one value.
    let a = sqrt2_plus_sqrt3();
    let b = sqrt_of(3, 1).add(&sqrt_of(2, 1));
    assert_eq!(algebraic_cmp(&a, &b), Some(Ordering::Equal));
    assert_eq!(
        a.cmp_with_budget(&b, DEFAULT_COMPARISON_BUDGET),
        Some(Ordering::Equal)
    );
}

#[test]
fn squared_sum_of_radicals_matches_expanded_form() {
    // (√2 + √3)² = 5 + 2√6 = 5 + √24.
    let squared = sqrt2_plus_sqrt3().mul(&sqrt2_plus_sqrt3());
    let expanded = rational(5, 1).add(&sqrt_of(24, 1));
    assert_eq!(algebraic_cmp(&squared, &expanded), Some(Ordering::Equal));
}

#[test]
fn sum_of_radicals_orders_below_sqrt_ten() {
    // (√2 + √3)² = 5 + 2√6 < 10, so √2 + √3 < √10. The two enclosures
    // overlap tightly (≈3.146 vs ≈3.162), forcing real interval
    // refinement rather than a first-cut separation.
    let a = sqrt2_plus_sqrt3();
    let b = sqrt_of(10, 1);
    assert_eq!(algebraic_cmp(&a, &b), Some(Ordering::Less));
    assert_eq!(algebraic_cmp(&b, &a), Some(Ordering::Greater));
    assert_eq!(a.lt_with_budget(&b, DEFAULT_COMPARISON_BUDGET), Some(true));
}

#[test]
fn sqrt_times_itself_is_the_radicand() {
    // √2 · √2 = 2 (regression: collapses in closed form pre-normal-form).
    let prod = sqrt_of(2, 1).mul(&sqrt_of(2, 1));
    assert_eq!(algebraic_cmp(&prod, &rational(2, 1)), Some(Ordering::Equal));
}

#[test]
fn sqrt_minus_itself_is_zero() {
    // √2 − √2 = 0 (SPEC §2.3.1.1's worked example).
    let diff = sqrt_of(2, 1).sub(&sqrt_of(2, 1));
    assert_eq!(algebraic_cmp(&diff, &rational(0, 1)), Some(Ordering::Equal));
}

#[test]
fn non_squarefree_radicands_share_a_basis() {
    // √2 + √8 = 3√2 = √18: the GCD-free basis must see through the
    // square parts of 8 and 18 without factoring into primes.
    let sum = sqrt_of(2, 1).add(&sqrt_of(8, 1));
    assert_eq!(algebraic_cmp(&sum, &sqrt_of(18, 1)), Some(Ordering::Equal));
}

#[test]
fn conjugate_product_is_rational() {
    // (√2 + 1)(√2 − 1) = 1.
    let plus = sqrt_of(2, 1).add(&rational(1, 1));
    let minus = sqrt_of(2, 1).sub(&rational(1, 1));
    assert_eq!(
        algebraic_cmp(&plus.mul(&minus), &rational(1, 1)),
        Some(Ordering::Equal)
    );
}

#[test]
fn division_rationalizes_the_denominator() {
    // 1 / (√2 + √3) = √3 − √2 … / 1 (since (√3+√2)(√3−√2) = 1), i.e.
    // multiplying back yields exactly 1 — exercises the recursive
    // conjugation inverse.
    let inv = rational(1, 1).div(&sqrt2_plus_sqrt3()).unwrap();
    let back = inv.mul(&sqrt2_plus_sqrt3());
    assert_eq!(algebraic_cmp(&back, &rational(1, 1)), Some(Ordering::Equal));
    // And 1/√2 = √2/2 in normal form.
    let a = rational(1, 1).div(&sqrt_of(2, 1)).unwrap();
    let b = sqrt_of(2, 1).div(&rational(2, 1)).unwrap();
    assert_eq!(algebraic_cmp(&a, &b), Some(Ordering::Equal));
}

#[test]
fn distinct_radical_sums_are_unequal() {
    // √2 + √3 ≠ √5 — distinct coefficient maps, decided structurally;
    // the order (√2+√3 ≈ 3.15 > √5 ≈ 2.24) comes from sign refinement.
    let sum = sqrt2_plus_sqrt3();
    assert_eq!(algebraic_cmp(&sum, &sqrt_of(5, 1)), Some(Ordering::Greater));
}

#[test]
fn rational_only_fields_compare_exactly() {
    // Gosper trees over rational leaves (e.g. built via mixed lazy
    // arithmetic) still normalize: (1 + √2) − √2 = 1.
    let v = rational(1, 1).add(&sqrt_of(2, 1)).sub(&sqrt_of(2, 1));
    assert_eq!(algebraic_cmp(&v, &rational(1, 1)), Some(Ordering::Equal));
}

#[test]
fn lazy_zero_divisor_is_detected() {
    // (√2 + √3) − (√3 + √2) is a Gosper tree denoting exactly 0: DIV
    // must refuse it (→ divisionByZero at the language boundary,
    // SPEC §4.2.7) instead of building a degenerate transform.
    let lazy_zero = sqrt2_plus_sqrt3().sub(&sqrt_of(3, 1).add(&sqrt_of(2, 1)));
    assert_eq!(algebraic_is_zero(&lazy_zero), Some(true));
    assert!(rational(10, 1).div(&lazy_zero).is_none());
    assert!(lazy_zero.reciprocal().is_none());
    // A non-zero lazy divisor still divides.
    assert!(rational(10, 1).div(&sqrt2_plus_sqrt3()).is_some());
}

#[test]
fn division_by_lazy_zero_inside_operand_falls_back_to_streamed() {
    // A degenerate transform recording a division by an exact lazy zero
    // (constructed below at the type level, bypassing `div`'s guard) is
    // outside the normal form's reach: `algebraic_cmp` must decline
    // (never wrongly decide), leaving the budgeted path's Undecided.
    use crate::types::continued_fraction::Gosper;
    use num_traits::{One, Zero};
    use std::sync::Arc;
    let lazy_zero = sqrt2_plus_sqrt3().sub(&sqrt_of(3, 1).add(&sqrt_of(2, 1)));
    // 1/x as a raw Möbius over the exact lazy zero.
    let degenerate = ExactReal::Gosper(Arc::new(Gosper::Mobius {
        a: BigInt::zero(),
        b: BigInt::one(),
        c: BigInt::one(),
        d: BigInt::zero(),
        x: lazy_zero,
    }));
    assert_eq!(algebraic_cmp(&degenerate, &rational(1, 1)), None);
    assert!(matches!(
        degenerate.cmp_with_budget_tracked(&rational(1, 1), DEFAULT_COMPARISON_BUDGET),
        CmpOutcome::Undecided { .. }
    ));
}

#[test]
fn nil_operand_declines() {
    let nil = ExactReal::Rational(Fraction::nil());
    assert_eq!(algebraic_cmp(&nil, &rational(0, 1)), None);
    assert_eq!(algebraic_is_zero(&nil), None);
}

#[test]
fn fractional_radicands_normalize() {
    // √(1/2) = √2 / 2, i.e. √(1/2) · 2 = √2.
    let half_root = sqrt_of(1, 2);
    let doubled = half_root.mul(&rational(2, 1));
    assert_eq!(
        algebraic_cmp(&doubled, &sqrt_of(2, 1)),
        Some(Ordering::Equal)
    );
}

#[test]
fn deep_mixed_expression_decides() {
    // ((√2+√3)·(√2−√3) + 1) = (2 − 3) + 1 = 0.
    let v = sqrt2_plus_sqrt3()
        .mul(&sqrt_of(2, 1).sub(&sqrt_of(3, 1)))
        .add(&rational(1, 1));
    assert_eq!(algebraic_cmp(&v, &rational(0, 1)), Some(Ordering::Equal));
    // And through the language-level default budget path:
    assert_eq!(
        v.eq_with_budget(&rational(0, 1), DEFAULT_COMPARISON_BUDGET),
        Some(true)
    );
}

#[test]
fn close_rational_approximations_still_order() {
    // 665857/470832 is a continued-fraction convergent of √2 that agrees
    // to ~12 digits (665857² − 2·470832² = 1, so it lies just *above*
    // √2); the sign refinement must still separate them.
    let approx = rational(665_857, 470_832);
    // Build √2 as a Gosper so the multiquadratic path (not the O(1)
    // sqrt-vs-rational shortcut) is the code under test.
    let sqrt2_gosper = sqrt_of(2, 1).add(&rational(1, 1)).sub(&rational(1, 1));
    assert_eq!(algebraic_cmp(&sqrt2_gosper, &approx), Some(Ordering::Less));
}
