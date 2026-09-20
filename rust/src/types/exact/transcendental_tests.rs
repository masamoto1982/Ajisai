//! Rigor probes for the Phase 7 numeric kernel: every enclosure contains the
//! true value (checked against 60-digit reference expansions computed
//! independently with Python's `decimal` module), nests, and shrinks; the
//! domain decisions are the ones the specification names.

use std::cmp::Ordering;

use num_bigint::BigInt;

use crate::types::exact::observation::RatInterval;
use crate::types::exact::series::{atan_bounds, cos_bounds, exp_bounds, ln_bounds, sin_bounds};
use crate::types::exact::{
    ExactCmp, ExactReal, PowOutcome, Transcendental, DEFAULT_COMPARISON_WATER,
};
use crate::types::fraction::Fraction;

fn frac(n: i64, d: i64) -> Fraction {
    Fraction::new(BigInt::from(n), BigInt::from(d))
}

/// A decimal literal as an exact rational.
fn dec(text: &str) -> Fraction {
    Fraction::from_str(text).expect("a decimal literal")
}

/// The reference is a decimal truncated at 64 places, so it sits within
/// `tolerance` of the true value rather than exactly on it: the enclosure,
/// whose width is far below that, must come within `tolerance` of it on
/// both sides, and be narrower than it.
fn encloses(iv: &RatInterval, truth: &Fraction, tolerance: &Fraction) {
    assert!(
        iv.lo.le(&truth.add(tolerance)) && truth.sub(tolerance).le(&iv.hi),
        "{truth} must lie within {tolerance} of [{}, {}]",
        iv.lo,
        iv.hi
    );
    assert!(iv.width().lt(tolerance), "width {} too wide", iv.width());
}

fn tiny() -> Fraction {
    Fraction::new(BigInt::from(1), BigInt::from(1) << 100usize)
}

#[test]
fn exp_bounds_enclose_known_values() {
    let e = dec("2.7182818284590452353602874713526624977572470936999595749669676277");
    let ulp = Fraction::new(BigInt::from(1), BigInt::from(10).pow(58));
    encloses(&exp_bounds(&frac(1, 1)), &e, &ulp);
    let e_inv = dec("0.3678794411714423215955237701614608674458111310317678345078368016");
    encloses(&exp_bounds(&frac(-1, 1)), &e_inv, &ulp);
    let e_ten = dec("22026.465794806716516957900645284244366353512618556781074235426355");
    encloses(&exp_bounds(&frac(10, 1)), &e_ten, &ulp);
    let e_half = dec("1.6487212707001281468486507878141635716537761007101480115750793116");
    encloses(&exp_bounds(&frac(1, 2)), &e_half, &ulp);
    assert!(exp_bounds(&frac(0, 1)).is_point());
}

#[test]
fn ln_bounds_enclose_known_values() {
    let ln2 = dec("0.6931471805599453094172321214581765680755001343602552541206800094");
    let ulp = Fraction::new(BigInt::from(1), BigInt::from(10).pow(58));
    encloses(&ln_bounds(&frac(2, 1)), &ln2, &ulp);
    let ln10 = dec("2.3025850929940456840179914546843642076011014886287729760333279009");
    encloses(&ln_bounds(&frac(10, 1)), &ln10, &ulp);
    let ln_half = dec("-0.693147180559945309417232121458176568075500134360255254120680009");
    encloses(&ln_bounds(&frac(1, 2)), &ln_half, &ulp);
    let ln3 = dec("1.0986122886681096913952452369225257046474905578227494517346943336");
    encloses(&ln_bounds(&frac(3, 1)), &ln3, &ulp);
    assert!(ln_bounds(&frac(1, 1)).is_point());
}

#[test]
fn atan_bounds_enclose_known_values() {
    let ulp = Fraction::new(BigInt::from(1), BigInt::from(10).pow(58));
    let pi_4 = dec("0.7853981633974483096156608458198757210492923498437764552437361480");
    encloses(&atan_bounds(&frac(1, 1)), &pi_4, &ulp);
    let atan_half = dec("0.4636476090008061162142562314612144020285370542861202638109330887");
    encloses(&atan_bounds(&frac(1, 2)), &atan_half, &ulp);
    let atan_2 = dec("1.1071487177940905030170654601785370400700476454014326466765392074");
    encloses(&atan_bounds(&frac(2, 1)), &atan_2, &ulp);
    let atan_neg_third = dec("-0.321750554396642193401404614358661319020755295557656191432803059");
    encloses(&atan_bounds(&frac(-1, 3)), &atan_neg_third, &ulp);
    let atan_thousand = dec("1.5697963271282297525647978820048308980869637651332848973960412479");
    encloses(&atan_bounds(&frac(1000, 1)), &atan_thousand, &ulp);
}

#[test]
fn trig_bounds_enclose_known_values() {
    let ulp = Fraction::new(BigInt::from(1), BigInt::from(10).pow(58));
    let sin1 = dec("0.8414709848078965066525023216302989996225630607983710656727517099");
    encloses(&sin_bounds(&frac(1, 1)), &sin1, &ulp);
    let cos1 = dec("0.5403023058681397174009366074429766037323104206179222276700972553");
    encloses(&cos_bounds(&frac(1, 1)), &cos1, &ulp);
    let sin100 = dec("-0.506365641109758793656557610459785432065032721290657323443392473");
    encloses(&sin_bounds(&frac(100, 1)), &sin100, &ulp);
    let cos_neg_7 = dec("0.7539022543433046381411975217191820122183133914601268395436138808");
    encloses(&cos_bounds(&frac(-7, 1)), &cos_neg_7, &ulp);
    let sin_neg_half = dec("-0.479425538604203000273287935215571388081803367940600675188616613");
    encloses(&sin_bounds(&frac(-1, 2)), &sin_neg_half, &ulp);
}

fn decided(x: &ExactReal, y: &ExactReal) -> Ordering {
    match x.cmp_within(y, DEFAULT_COMPARISON_WATER) {
        ExactCmp::Decided(o) => o,
        other => panic!("expected a decision, got {other:?}"),
    }
}

fn value(t: Transcendental) -> ExactReal {
    match t {
        Transcendental::Value(v) => v,
        other => panic!("expected a value, got {other:?}"),
    }
}

fn rational(n: i64, d: i64) -> ExactReal {
    ExactReal::from_fraction(frac(n, d))
}

#[test]
fn tier2_values_nest_shrink_and_decide_against_rationals() {
    let e = value(rational(1, 1).exp());
    let mut prev = e.enclosure_at(0);
    for step in 1..200 {
        let now = e.enclosure_at(step);
        assert!(now.is_within(&prev), "step {step} must nest");
        assert!(!prev.width().lt(&now.width()), "width must not grow");
        prev = now;
    }
    assert!(prev.width().lt(&tiny()));
    assert_eq!(decided(&e, &rational(27182, 10000)), Ordering::Greater);
    assert_eq!(decided(&e, &rational(27183, 10000)), Ordering::Less);
    // e = exp(1) and exp(½)² agree to the plateau, so they starve honestly.
    let half = value(rational(1, 2).exp());
    let squared = half.mul(&half);
    assert!(matches!(
        e.cmp_within(&squared, DEFAULT_COMPARISON_WATER),
        ExactCmp::Starved { .. }
    ));
}

#[test]
fn transcendentals_compose_over_every_tier() {
    // ln(exp(3)) encloses 3 within the plateau.
    let x = value(value(rational(3, 1).exp()).ln());
    let iv = x.enclosure_at(600);
    assert!(iv.lo.le(&frac(3, 1)) && frac(3, 1).le(&iv.hi));
    assert!(iv.width().lt(&tiny()));
    // sin over an algebraic argument: sin(√2) ∈ (0.987, 0.988).
    let root2 = ExactReal::from_sqrt_rational(frac(2, 1)).unwrap();
    let s = value(root2.sin());
    assert_eq!(decided(&s, &rational(987, 1000)), Ordering::Greater);
    assert_eq!(decided(&s, &rational(988, 1000)), Ordering::Less);
    // cos(π) encloses −1: a Tier 2 argument through a Lipschitz map.
    let pi = ExactReal::Computable(crate::types::exact::pi::pi());
    let c = value(pi.cos());
    let iv = c.enclosure_at(600);
    assert!(iv.lo.le(&frac(-1, 1)) && frac(-1, 1).le(&iv.hi));
    assert!(iv.width().lt(&tiny()));
    // atan is total; 4·atan(1) encloses π.
    let four_atan_one = value(rational(1, 1).atan()).mul(&rational(4, 1));
    assert!(matches!(
        four_atan_one.cmp_within(&pi, DEFAULT_COMPARISON_WATER),
        ExactCmp::Starved { .. }
    ));
}

#[test]
fn domain_decisions_are_the_specified_ones() {
    assert!(matches!(rational(0, 1).ln(), Transcendental::DomainMiss));
    assert!(matches!(rational(-2, 1).ln(), Transcendental::DomainMiss));
    assert!(matches!(
        rational(1, 1).ln(),
        Transcendental::Value(ExactReal::Rational(_))
    ));
    let pi = ExactReal::Computable(crate::types::exact::pi::pi());
    assert!(matches!(pi.sub(&pi).ln(), Transcendental::Undecidable));
    assert!(matches!(pi.neg().ln(), Transcendental::DomainMiss));
    assert!(matches!(
        ExactReal::from_fraction(frac(1 << 40, 1)).exp(),
        Transcendental::SpaceExhausted
    ));
    assert!(matches!(
        ExactReal::from_fraction(Fraction::new(BigInt::from(1) << 70usize, BigInt::from(1))).sin(),
        Transcendental::SpaceExhausted
    ));
}

fn pow_value(x: &ExactReal, y: &ExactReal) -> ExactReal {
    match x.pow(y) {
        PowOutcome::Value(v) => v,
        other => panic!("expected a value, got {other:?}"),
    }
}

#[test]
fn pow_stays_in_the_cheapest_tier_that_holds_it() {
    assert_eq!(
        pow_value(&rational(2, 1), &rational(10, 1)),
        rational(1024, 1)
    );
    assert_eq!(pow_value(&rational(2, 1), &rational(-2, 1)), rational(1, 4));
    assert_eq!(
        pow_value(&rational(-3, 1), &rational(3, 1)),
        rational(-27, 1)
    );
    assert_eq!(pow_value(&rational(0, 1), &rational(0, 1)), rational(1, 1));
    assert_eq!(pow_value(&rational(8, 1), &rational(1, 3)), rational(2, 1));
    assert_eq!(
        pow_value(&rational(27, 8), &rational(-2, 3)),
        rational(4, 9)
    );
    // x^(p/2) is (√x)^p, in the field.
    let root2 = ExactReal::from_sqrt_rational(frac(2, 1)).unwrap();
    assert_eq!(pow_value(&rational(2, 1), &rational(1, 2)), root2);
    assert_eq!(
        pow_value(&rational(2, 1), &rational(3, 2)),
        root2.mul(&rational(2, 1))
    );
    // An algebraic base under an integer exponent stays algebraic.
    assert_eq!(pow_value(&root2, &rational(2, 1)), rational(2, 1));
    assert_eq!(pow_value(&root2, &rational(-2, 1)), rational(1, 2));
    // A cube root that is not exact is a computable real near the truth.
    let cbrt2 = pow_value(&rational(2, 1), &rational(1, 3));
    assert_eq!(decided(&cbrt2, &rational(12599, 10000)), Ordering::Greater);
    assert_eq!(decided(&cbrt2, &rational(12600, 10000)), Ordering::Less);
    // π² through interval multiplication decides against 9.8696.
    let pi = ExactReal::Computable(crate::types::exact::pi::pi());
    let pi2 = pow_value(&pi, &rational(2, 1));
    assert_eq!(decided(&pi2, &rational(98696, 10000)), Ordering::Greater);
    assert_eq!(decided(&pi2, &rational(98697, 10000)), Ordering::Less);
    let pi_inv = pow_value(&pi, &rational(-1, 1));
    assert_eq!(decided(&pi_inv, &rational(3183, 10000)), Ordering::Greater);
    // 2^π ∈ (8.8249, 8.8250).
    let two_pi = pow_value(&rational(2, 1), &pi);
    assert_eq!(decided(&two_pi, &rational(88249, 10000)), Ordering::Greater);
    assert_eq!(decided(&two_pi, &rational(88250, 10000)), Ordering::Less);
}

#[test]
fn pow_refuses_what_has_no_real_value() {
    assert!(matches!(
        rational(0, 1).pow(&rational(-1, 1)),
        PowOutcome::DivisionByZero
    ));
    assert!(matches!(
        rational(0, 1).pow(&rational(-1, 2)),
        PowOutcome::DivisionByZero
    ));
    assert!(matches!(
        rational(-8, 1).pow(&rational(1, 3)),
        PowOutcome::DomainMiss
    ));
    assert!(matches!(
        rational(-2, 1).pow(&rational(1, 2)),
        PowOutcome::DomainMiss
    ));
    assert!(matches!(
        rational(0, 1).pow(&rational(1, 2)),
        PowOutcome::Value(_)
    ));
    let pi = ExactReal::Computable(crate::types::exact::pi::pi());
    assert!(matches!(
        pi.sub(&pi).pow(&rational(-1, 1)),
        PowOutcome::Undecidable
    ));
    assert!(matches!(
        pi.sub(&pi).pow(&rational(1, 3)),
        PowOutcome::Undecidable
    ));
    assert!(matches!(
        rational(2, 1).pow(&rational(1 << 30, 1)),
        PowOutcome::SpaceExhausted
    ));
}
