//! Property tests for the Tier 1 algebraic normal form: ring axioms over
//! sampled values, √r·√r = r, eager demotion to Tier 0, semantic
//! normal-form uniqueness across construction histories, decidable
//! comparison, and the derived CF against known expansions.

use crate::types::exact::algebraic::{Algebraic, AlgebraicResult};
use crate::types::exact::observation::{Observation, Water};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use std::cmp::Ordering;

fn frac(n: i64, d: i64) -> Fraction {
    Fraction::new(BigInt::from(n), BigInt::from(d))
}

fn sqrt_irr(n: i64, d: i64) -> Algebraic {
    match Algebraic::sqrt_of_fraction(&frac(n, d)) {
        Some(AlgebraicResult::Irrational(a)) => a,
        other => panic!("√({n}/{d}) should be irrational, got {other:?}"),
    }
}

/// A small pool of Tier 1 values with varied bases and signs, for the
/// axiom checks below.
fn samples() -> Vec<Algebraic> {
    let sqrt2 = sqrt_irr(2, 1);
    let sqrt3 = sqrt_irr(3, 1);
    let sqrt_half = sqrt_irr(1, 2);
    let sum = match sqrt2.add(&sqrt3) {
        AlgebraicResult::Irrational(a) => a,
        other => panic!("√2+√3 is irrational, got {other:?}"),
    };
    let shifted = match sqrt2.add_fraction(&frac(-7, 3)) {
        AlgebraicResult::Irrational(a) => a,
        other => panic!("√2−7/3 is irrational, got {other:?}"),
    };
    vec![sqrt2, sqrt3, sqrt_half, sum, shifted]
}

fn as_result(a: &Algebraic) -> AlgebraicResult {
    AlgebraicResult::Irrational(a.clone())
}

fn results_equal(a: &AlgebraicResult, b: &AlgebraicResult) -> bool {
    match (a, b) {
        (AlgebraicResult::Rational(x), AlgebraicResult::Rational(y)) => x == y,
        (AlgebraicResult::Irrational(x), AlgebraicResult::Irrational(y)) => x == y,
        // Mixed shapes cannot be equal: demotion is eager, so a rational
        // never hides inside an `Irrational`.
        _ => false,
    }
}

fn add_results(a: &AlgebraicResult, b: &AlgebraicResult) -> AlgebraicResult {
    match (a, b) {
        (AlgebraicResult::Rational(x), AlgebraicResult::Rational(y)) => {
            AlgebraicResult::Rational(x.add(y))
        }
        (AlgebraicResult::Rational(x), AlgebraicResult::Irrational(y))
        | (AlgebraicResult::Irrational(y), AlgebraicResult::Rational(x)) => y.add_fraction(x),
        (AlgebraicResult::Irrational(x), AlgebraicResult::Irrational(y)) => x.add(y),
    }
}

fn mul_results(a: &AlgebraicResult, b: &AlgebraicResult) -> AlgebraicResult {
    match (a, b) {
        (AlgebraicResult::Rational(x), AlgebraicResult::Rational(y)) => {
            AlgebraicResult::Rational(x.mul(y))
        }
        (AlgebraicResult::Rational(x), AlgebraicResult::Irrational(y))
        | (AlgebraicResult::Irrational(y), AlgebraicResult::Rational(x)) => y.mul_fraction(x),
        (AlgebraicResult::Irrational(x), AlgebraicResult::Irrational(y)) => x.mul(y),
    }
}

#[test]
fn ring_axioms_hold_over_samples() {
    let pool = samples();
    for a in &pool {
        for b in &pool {
            // Commutativity.
            assert!(results_equal(&a.add(b), &b.add(a)), "a+b = b+a");
            assert!(results_equal(&a.mul(b), &b.mul(a)), "a·b = b·a");
            for c in &pool {
                // Associativity.
                let left = add_results(&a.add(b), &as_result(c));
                let right = add_results(&as_result(a), &b.add(c));
                assert!(results_equal(&left, &right), "(a+b)+c = a+(b+c)");
                let left = mul_results(&a.mul(b), &as_result(c));
                let right = mul_results(&as_result(a), &b.mul(c));
                assert!(results_equal(&left, &right), "(a·b)·c = a·(b·c)");
                // Distributivity.
                let left = mul_results(&as_result(a), &b.add(c));
                let right = add_results(&a.mul(b), &a.mul(c));
                assert!(results_equal(&left, &right), "a·(b+c) = a·b + a·c");
            }
        }
    }
}

#[test]
fn additive_and_multiplicative_inverses_cancel() {
    for a in &samples() {
        // a + (−a) = 0, demoted to the Tier 0 zero.
        match a.add(&a.neg()) {
            AlgebraicResult::Rational(f) => assert!(f.is_zero()),
            other => panic!("a + (−a) should demote to 0, got {other:?}"),
        }
        // a · a⁻¹ = 1.
        let product = match a.reciprocal() {
            AlgebraicResult::Rational(f) => a.mul_fraction(&f),
            AlgebraicResult::Irrational(inv) => a.mul(&inv),
        };
        match product {
            AlgebraicResult::Rational(f) => assert_eq!(f, frac(1, 1)),
            other => panic!("a · a⁻¹ should demote to 1, got {other:?}"),
        }
    }
}

#[test]
fn sqrt_times_itself_recovers_radicand() {
    for (n, d) in [(2, 1), (3, 1), (8, 1), (1, 2), (5, 7)] {
        let r = sqrt_irr(n, d);
        match r.mul(&r) {
            AlgebraicResult::Rational(f) => assert_eq!(f, frac(n, d), "√r·√r = r for {n}/{d}"),
            other => panic!("√r·√r should demote to the rational r, got {other:?}"),
        }
    }
}

#[test]
fn sqrt_normalizes_like_the_historical_constructor() {
    // Perfect squares and zero demote to Tier 0.
    assert_eq!(
        Algebraic::sqrt_of_fraction(&frac(9, 4)),
        Some(AlgebraicResult::Rational(frac(3, 2)))
    );
    assert_eq!(
        Algebraic::sqrt_of_fraction(&frac(0, 1)),
        Some(AlgebraicResult::Rational(frac(0, 1)))
    );
    // Negative and nil inputs are rejected.
    assert_eq!(Algebraic::sqrt_of_fraction(&frac(-2, 1)), None);
    assert_eq!(Algebraic::sqrt_of_fraction(&Fraction::nil()), None);
}

#[test]
fn demotion_is_eager_famous_identity() {
    // (1+√2)(√2−1) = 1: the Gosper-era value that could not even show
    // its leading CF term now demotes to the Tier 0 rational 1/1.
    let sqrt2 = sqrt_irr(2, 1);
    let one_plus = match sqrt2.add_fraction(&frac(1, 1)) {
        AlgebraicResult::Irrational(a) => a,
        other => panic!("1+√2 is irrational, got {other:?}"),
    };
    let minus_one = match sqrt2.add_fraction(&frac(-1, 1)) {
        AlgebraicResult::Irrational(a) => a,
        other => panic!("√2−1 is irrational, got {other:?}"),
    };
    match one_plus.mul(&minus_one) {
        AlgebraicResult::Rational(f) => assert_eq!(f, frac(1, 1)),
        other => panic!("(1+√2)(√2−1) should demote to 1/1, got {other:?}"),
    }
}

#[test]
fn normal_form_identity_survives_construction_history() {
    // √8 built directly (coarse basis {8}) equals 2·√2 (basis {2}) —
    // uniqueness is semantic, guaranteed by rebasing, not structural.
    let sqrt8 = sqrt_irr(8, 1);
    let two_sqrt2 = match sqrt_irr(2, 1).mul_fraction(&frac(2, 1)) {
        AlgebraicResult::Irrational(a) => a,
        other => panic!("2√2 is irrational, got {other:?}"),
    };
    assert_eq!(sqrt8, two_sqrt2);
    assert_eq!(sqrt8.cmp(&two_sqrt2), Ordering::Equal);
    // And √12 vs 2·√3 through an additive detour.
    let sqrt12 = sqrt_irr(12, 1);
    let detour = match sqrt_irr(3, 1).add(&sqrt_irr(3, 1)) {
        AlgebraicResult::Irrational(a) => a,
        other => panic!("√3+√3 is irrational, got {other:?}"),
    };
    assert_eq!(sqrt12, detour);
}

#[test]
fn comparison_is_total_and_budget_free() {
    let sqrt2 = sqrt_irr(2, 1);
    let sqrt3 = sqrt_irr(3, 1);
    assert_eq!(sqrt2.cmp(&sqrt3), Ordering::Less);
    assert_eq!(sqrt3.cmp(&sqrt2), Ordering::Greater);
    assert_eq!(sqrt2.cmp(&sqrt2), Ordering::Equal);
    // Against rationals, including the √2 < 2 acceptance criterion.
    assert_eq!(sqrt2.cmp_fraction(&frac(2, 1)), Ordering::Less);
    assert_eq!(sqrt2.cmp_fraction(&frac(1, 1)), Ordering::Greater);
    // A pair whose CF streams agree for many terms still decides:
    // √8 vs √2+√2 (equal values through different histories).
    let sqrt8 = sqrt_irr(8, 1);
    let doubled = match sqrt_irr(2, 1).add(&sqrt_irr(2, 1)) {
        AlgebraicResult::Irrational(a) => a,
        other => panic!("√2+√2 is irrational, got {other:?}"),
    };
    assert_eq!(sqrt8.cmp(&doubled), Ordering::Equal);
    // Sign of a multi-term difference: √2+√3 − 3 < 0 < √2+√3 − 3.14…?
    let sum = match sqrt2.add(&sqrt3) {
        AlgebraicResult::Irrational(a) => a,
        other => panic!("√2+√3 is irrational, got {other:?}"),
    };
    assert_eq!(sum.cmp_fraction(&frac(3, 1)), Ordering::Greater);
    assert_eq!(sum.cmp_fraction(&frac(315, 100)), Ordering::Less);
}

#[test]
fn floor_ceil_round_are_exact() {
    let sqrt2 = sqrt_irr(2, 1);
    assert_eq!(sqrt2.floor_int(), BigInt::from(1));
    assert_eq!(sqrt2.ceil_int(), BigInt::from(2));
    assert_eq!(sqrt2.round_int(), BigInt::from(1));
    let neg = sqrt2.neg();
    assert_eq!(neg.floor_int(), BigInt::from(-2));
    assert_eq!(neg.ceil_int(), BigInt::from(-1));
    assert_eq!(neg.round_int(), BigInt::from(-1));
    // √3 ≈ 1.732 rounds up.
    assert_eq!(sqrt_irr(3, 1).round_int(), BigInt::from(2));
}

#[test]
fn derived_cf_matches_known_expansions() {
    // √2 = [1; 2, 2, 2, …].
    let cf = sqrt_irr(2, 1).cf_prefix(8);
    let expected: Vec<BigInt> = [1, 2, 2, 2, 2, 2, 2, 2].iter().map(|n| BigInt::from(*n)).collect();
    assert_eq!(cf, expected);
    // √3 = [1; 1, 2, 1, 2, …].
    let cf = sqrt_irr(3, 1).cf_prefix(7);
    let expected: Vec<BigInt> = [1, 1, 2, 1, 2, 1, 2].iter().map(|n| BigInt::from(*n)).collect();
    assert_eq!(cf, expected);
    // −√2 = [−2; 1, 1, 2, 2, 2, …] (floor convention).
    let cf = sqrt_irr(2, 1).neg().cf_prefix(6);
    let expected: Vec<BigInt> = [-2, 1, 1, 2, 2, 2].iter().map(|n| BigInt::from(*n)).collect();
    assert_eq!(cf, expected);
    // √(1/2) = [0; 1, 2, 2, 2, …].
    let cf = sqrt_irr(1, 2).cf_prefix(6);
    let expected: Vec<BigInt> = [0, 1, 2, 2, 2, 2].iter().map(|n| BigInt::from(*n)).collect();
    assert_eq!(cf, expected);
}

#[test]
fn best_rational_approximation_returns_principal_convergents() {
    let sqrt2 = sqrt_irr(2, 1);
    // Convergents of √2: 1, 3/2, 7/5, 17/12, 41/29, 99/70, …
    assert_eq!(
        sqrt2.best_rational_approximation(&BigInt::from(1)),
        Some(frac(1, 1))
    );
    assert_eq!(
        sqrt2.best_rational_approximation(&BigInt::from(12)),
        Some(frac(17, 12))
    );
    assert_eq!(
        sqrt2.best_rational_approximation(&BigInt::from(70)),
        Some(frac(99, 70))
    );
    assert_eq!(sqrt2.best_rational_approximation(&BigInt::from(0)), None);
}

#[test]
fn observation_adapter_narrows_monotonically() {
    let sqrt2 = sqrt_irr(2, 1);
    let mut obs = sqrt2.observe();
    let first = obs.current_interval().expect("Tier 1 always encloses");
    assert!(first.lo.lt(&first.hi), "irrational enclosure is not a point");
    assert_eq!(obs.refine(Water(24)), crate::types::exact::Refine::Narrower);
    let second = obs.current_interval().expect("still enclosed");
    assert!(second.is_within(&first), "refinement is monotone");
    assert!(second.width().lt(&first.width()), "refinement narrows");
    // The enclosure straddles the true value: lo < √2 < hi ⇔ lo² < 2 < hi².
    assert!(second.lo.mul(&second.lo).lt(&frac(2, 1)));
    assert!(second.hi.mul(&second.hi).gt(&frac(2, 1)));
}
