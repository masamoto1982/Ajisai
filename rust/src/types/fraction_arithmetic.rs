//! `Fraction` arithmetic, and the one shared `gcd` for `BigInt`
//! (`balanced_bigint_gcd`), balanced before it reaches `num-bigint`'s binary
//! GCD. The gcd is used from `fraction.rs`, this file, and the Tier 1
//! algebraic normal form (`exact::basis`, `exact::algebraic`,
//! `exact::algebraic_field`) — a shared low-level primitive, kept beside the
//! arithmetic that calls it most.

use super::fraction::{compute_gcd_i64, Fraction, FractionRepr};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Zero};

impl Fraction {
    pub fn add(&self, other: &Fraction) -> Fraction {
        if self.is_nil() || other.is_nil() {
            return Self::nil();
        }

        if let (Some((a, b)), Some((c, d))) = (self.extract_i64_pair(), other.extract_i64_pair()) {
            if b == 1 && d == 1 {
                return Self::create_from_i128((a as i128) + (c as i128), 1);
            }
            if b == d {
                return Self::create_from_i128((a as i128) + (c as i128), b as i128);
            }
            if let Some(num) = (a as i128).checked_mul(d as i128).and_then(|ad| {
                (c as i128)
                    .checked_mul(b as i128)
                    .and_then(|cb| ad.checked_add(cb))
            }) {
                return Self::create_from_i128(num, (b as i128) * (d as i128));
            }
        }

        let (an, ad): (BigInt, BigInt) = self.to_bigint_pair();
        let (bn, bd): (BigInt, BigInt) = other.to_bigint_pair();

        if ad == bd {
            let sum: BigInt = &an + &bn;
            if sum.is_zero() {
                return Fraction::from_repr(FractionRepr::Small(0, 1));
            }
            let g: BigInt = balanced_bigint_gcd(&sum, &ad);
            if g.is_one() {
                return Self::create_already_reduced(sum, ad);
            }
            return Self::create_already_reduced(&sum / &g, &ad / &g);
        }

        Self::add_reduced_bigint(&an, &ad, &bn, &bd, false)
    }

    pub fn sub(&self, other: &Fraction) -> Fraction {
        if self.is_nil() || other.is_nil() {
            return Self::nil();
        }

        if let (Some((a, b)), Some((c, d))) = (self.extract_i64_pair(), other.extract_i64_pair()) {
            if b == 1 && d == 1 {
                return Self::create_from_i128((a as i128) - (c as i128), 1);
            }
            if b == d {
                return Self::create_from_i128((a as i128) - (c as i128), b as i128);
            }
            if let Some(num) = (a as i128).checked_mul(d as i128).and_then(|ad| {
                (c as i128)
                    .checked_mul(b as i128)
                    .and_then(|cb| ad.checked_sub(cb))
            }) {
                return Self::create_from_i128(num, (b as i128) * (d as i128));
            }
        }

        let (an, ad): (BigInt, BigInt) = self.to_bigint_pair();
        let (bn, bd): (BigInt, BigInt) = other.to_bigint_pair();

        if ad == bd {
            let diff: BigInt = &an - &bn;
            if diff.is_zero() {
                return Fraction::from_repr(FractionRepr::Small(0, 1));
            }
            let g: BigInt = balanced_bigint_gcd(&diff, &ad);
            if g.is_one() {
                return Self::create_already_reduced(diff, ad);
            }
            return Self::create_already_reduced(&diff / &g, &ad / &g);
        }

        Self::add_reduced_bigint(&an, &ad, &bn, &bd, true)
    }

    /// `a/b ± c/d` for two already-reduced fractions with positive
    /// denominators, by Henrici's algorithm (Knuth, TAOCP vol. 2, 4.5.1) —
    /// the one CPython's `fractions.Fraction` uses.
    ///
    /// The schoolbook form `(a·d ± c·b) / (b·d)` followed by one gcd hands
    /// that gcd two operands as wide as the *product* of the denominators.
    /// Henrici takes `g = gcd(b, d)` first: when it is 1 the sum is already
    /// in lowest terms and no wide gcd runs at all, and otherwise the only
    /// remaining gcd is `gcd(t, g)`, whose second operand is no wider than
    /// the narrower denominator. Accumulating a wide fraction with a narrow
    /// one — `1 1 n RANGE DIV 0 [ ADD ] FOLD`, a running total, any sum of
    /// many small-denominator terms — therefore never asks for a gcd of two
    /// wide numbers. Both gcds go through `balanced_bigint_gcd`, so the
    /// wide-against-narrow shape costs one division, not a binary GCD.
    /// Measured on the harmonic number H(5000): 810 ms → 7 ms natively.
    ///
    /// The result is in lowest terms, so it is built without the
    /// normalizing gcd `Fraction::new` would run again.
    fn add_reduced_bigint(
        an: &BigInt,
        ad: &BigInt,
        bn: &BigInt,
        bd: &BigInt,
        subtract: bool,
    ) -> Fraction {
        let g: BigInt = balanced_bigint_gcd(ad, bd);
        if g.is_one() {
            let cross = bn * ad;
            let num = if subtract {
                an * bd - cross
            } else {
                an * bd + cross
            };
            return Self::create_already_reduced(num, ad * bd);
        }
        let ad_g: BigInt = ad / &g;
        let cross = bn * &ad_g;
        let t: BigInt = if subtract {
            an * (bd / &g) - cross
        } else {
            an * (bd / &g) + cross
        };
        if t.is_zero() {
            return Fraction::from_repr(FractionRepr::Small(0, 1));
        }
        let g2: BigInt = balanced_bigint_gcd(&t, &g);
        if g2.is_one() {
            return Self::create_already_reduced(t, ad_g * bd);
        }
        Self::create_already_reduced(&t / &g2, ad_g * (bd / &g2))
    }

    pub fn mul(&self, other: &Fraction) -> Fraction {
        if self.is_nil() || other.is_nil() {
            return Self::nil();
        }

        if let (Some((a, b)), Some((c, d))) = (self.extract_i64_pair(), other.extract_i64_pair()) {
            let g1 = compute_gcd_i64(a, d);
            let g2 = compute_gcd_i64(c, b);
            let a_r = (a / g1) as i128;
            let b_r = (b / g2) as i128;
            let c_r = (c / g2) as i128;
            let d_r = (d / g1) as i128;
            if let (Some(num), Some(den)) = (a_r.checked_mul(c_r), b_r.checked_mul(d_r)) {
                return Self::create_from_i128(num, den);
            }
        }

        let (an, ad): (BigInt, BigInt) = self.to_bigint_pair();
        let (bn, bd): (BigInt, BigInt) = other.to_bigint_pair();

        if ad.is_one() && bd.is_one() {
            return Self::from_bigint_pair(an * bn, BigInt::one());
        }

        if ad.is_one() {
            let g: BigInt = balanced_bigint_gcd(&an, &bd);
            let a_reduced: BigInt = &an / &g;
            let d_reduced: BigInt = &bd / &g;
            return Self::create_already_reduced(a_reduced * bn, d_reduced);
        }

        if bd.is_one() {
            let g: BigInt = balanced_bigint_gcd(&bn, &ad);
            let c_reduced: BigInt = &bn / &g;
            let b_reduced: BigInt = &ad / &g;
            return Self::create_already_reduced(an * c_reduced, b_reduced);
        }

        let g1: BigInt = balanced_bigint_gcd(&an, &bd);
        let g2: BigInt = balanced_bigint_gcd(&bn, &ad);

        let a_reduced: BigInt = &an / &g1;
        let d_reduced: BigInt = &bd / &g1;
        let c_reduced: BigInt = &bn / &g2;
        let b_reduced: BigInt = &ad / &g2;

        Self::create_already_reduced(a_reduced * c_reduced, b_reduced * d_reduced)
    }

    pub fn div(&self, other: &Fraction) -> Fraction {
        if self.is_nil() || other.is_nil() {
            return Self::nil();
        }
        if other.is_zero() {
            panic!("Division by zero");
        }

        if let (Some((a, b)), Some((c, d))) = (self.extract_i64_pair(), other.extract_i64_pair()) {
            let g1 = compute_gcd_i64(a, c);
            let g2 = compute_gcd_i64(d, b);
            let a_r = (a / g1) as i128;
            let b_r = (b / g2) as i128;
            let c_r = (c / g1) as i128;
            let d_r = (d / g2) as i128;
            if let (Some(num), Some(den)) = (a_r.checked_mul(d_r), b_r.checked_mul(c_r)) {
                return Self::create_from_i128(num, den);
            }
        }

        let (an, ad): (BigInt, BigInt) = self.to_bigint_pair();
        let (bn, bd): (BigInt, BigInt) = other.to_bigint_pair();

        if ad.is_one() && bd.is_one() {
            return Fraction::new(an, bn);
        }

        if ad.is_one() {
            let g: BigInt = balanced_bigint_gcd(&an, &bn);
            let a_reduced: BigInt = &an / &g;
            let c_reduced: BigInt = &bn / &g;
            return Self::create_already_reduced(a_reduced * bd, c_reduced);
        }

        if bd.is_one() {
            let g: BigInt = balanced_bigint_gcd(&an, &bn);
            let a_reduced: BigInt = &an / &g;
            let c_reduced: BigInt = &bn / &g;
            return Self::create_already_reduced(a_reduced, ad * c_reduced);
        }

        let g1: BigInt = balanced_bigint_gcd(&an, &bn);
        let g2: BigInt = balanced_bigint_gcd(&bd, &ad);

        let a_reduced: BigInt = &an / &g1;
        let c_reduced: BigInt = &bn / &g1;
        let d_reduced: BigInt = &bd / &g2;
        let b_reduced: BigInt = &ad / &g2;

        Self::create_already_reduced(a_reduced * d_reduced, b_reduced * c_reduced)
    }

    #[inline]
    pub fn abs(&self) -> Fraction {
        if self.is_nil() {
            return self.clone();
        }
        match &self.repr {
            // `i64::MIN` has no positive i64 counterpart, so `i64::abs()`
            // overflows on it (a panic in debug, the same negative value back
            // in release). `checked_abs` declines instead, and that one
            // operand widens to `Big`, as `round` and `compute_gcd_i64` do.
            FractionRepr::Small(n, d) => match n.checked_abs() {
                Some(magnitude) => Fraction::from_repr(FractionRepr::Small(magnitude, *d)),
                None => Fraction::from_repr(FractionRepr::Big {
                    numerator: -BigInt::from(*n),
                    denominator: BigInt::from(*d),
                }),
            },
            FractionRepr::Big {
                numerator,
                denominator,
            } => Fraction::from_repr(FractionRepr::Big {
                numerator: if *numerator < BigInt::zero() {
                    -numerator.clone()
                } else {
                    numerator.clone()
                },
                denominator: denominator.clone(),
            }),
        }
    }

    pub fn floor(&self) -> Fraction {
        // Absence propagates, as it does through `add`, `sub`, `mul` and
        // `div`: `Fraction::nil` is `0/0`, so without this an absent lane
        // reached the division below and `[ NIL 1 ] FLOOR` aborted the process.
        if self.is_nil() {
            return Self::nil();
        }
        if self.is_integer() {
            return Fraction::from_repr(self.repr.clone());
        }

        match &self.repr {
            FractionRepr::Small(n, d) => {
                let q = n / d;
                let r = n % d;
                let floored = if *n < 0 && r != 0 { q - 1 } else { q };
                Fraction::from_repr(FractionRepr::Small(floored, 1))
            }
            FractionRepr::Big {
                numerator,
                denominator,
            } => {
                let q = numerator / denominator;
                let r = numerator % denominator;
                let floored = if *numerator < BigInt::zero() && !r.is_zero() {
                    q - BigInt::one()
                } else {
                    q
                };
                Self::from_bigint_pair(floored, BigInt::one())
            }
        }
    }

    pub fn round(&self) -> Fraction {
        // As `floor`: an absent lane is `0/0` and must not reach the division.
        if self.is_nil() {
            return Self::nil();
        }
        if self.is_integer() {
            return Fraction::from_repr(self.repr.clone());
        }

        if self.is_zero() {
            return Fraction::from_repr(FractionRepr::Small(0, 1));
        }

        match &self.repr {
            FractionRepr::Small(n, d) => {
                let is_negative = *n < 0;
                // Widen to i128 *before* taking the absolute value: `i64::MIN`
                // has no positive i64 counterpart, so `i64::abs()` overflows and
                // panics in debug (reachable from a `-9223372036854775808/d`
                // operand). i128 holds it exactly.
                let abs_n = (*n as i128).abs();
                let d128 = *d as i128;
                let result = ((2 * abs_n + d128) / (2 * d128)) as i64;
                Fraction::from_repr(FractionRepr::Small(
                    if is_negative { -result } else { result },
                    1,
                ))
            }
            FractionRepr::Big {
                numerator,
                denominator,
            } => {
                let is_negative = *numerator < BigInt::zero();
                let abs_num = if is_negative {
                    -numerator.clone()
                } else {
                    numerator.clone()
                };
                let two = BigInt::from(2);
                let two_abs_num = &abs_num * &two;
                let result = (&two_abs_num + denominator) / (&two * denominator);
                Self::from_bigint_pair(if is_negative { -result } else { result }, BigInt::one())
            }
        }
    }
}

/// `gcd(a, b)` for `BigInt`, balanced before handing off to `num-bigint`'s
/// binary GCD.
///
/// `num_integer::Integer::gcd` on `BigInt` is quadratic in the *difference*
/// between the operands' bit widths, not their size: `gcd(4096-digit, 1)`
/// measured 612 µs on one container, `gcd(4096-digit, 7)` 356 µs, but
/// `gcd(4096-digit, 4096-digit)` 1.2 µs. Every wide rational whose
/// denominator is 1 — i.e. every wide *integer*-valued `Fraction`, which is
/// the common case, not an edge one — hits the slow side of that on every
/// normalization, every `+`/`-`/`*`/`/`, and the equality hash
/// (`Fraction`'s `Hash` impl); a `UNIQUE` over a vector of wide integers was
/// measured charging the collection meter for roughly 1/470th of the time
/// it actually took, because the pricing had no way to see this and priced
/// the value's declared width instead.
///
/// `gcd(a, b) == gcd(b, a mod b)`: one Euclidean division collapses the
/// wider operand down to at most the narrower operand's width — cheap,
/// since a `BigInt` remainder is not the binary GCD's pathological case —
/// and the binary GCD that follows then runs on two comparably-sized
/// operands, its fast case. Verified against `Integer::gcd` directly (not
/// merely against this function's own logic) below: a property test
/// compares the two on random pairs across a spread of relative widths,
/// plus zero and negative operands, and every call site's existing
/// arithmetic/hash tests exercise it in place. Measured 494x faster at
/// `gcd(4096-digit, 1)` and within noise of the direct call once the
/// operands are already balanced, where the extra division is pure
/// overhead.
#[inline]
pub(crate) fn balanced_bigint_gcd(a: &BigInt, b: &BigInt) -> BigInt {
    if a.is_zero() {
        return b.gcd(a);
    }
    if b.is_zero() {
        return a.gcd(b);
    }
    if a.bits() >= b.bits() {
        b.gcd(&(a % b))
    } else {
        a.gcd(&(b % a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Hand-picked pairs covering the shapes the doc comment's measurements
    /// used: a wide operand against 0, against a small denominator (1, 7),
    /// and against another wide operand of comparable size.
    #[test]
    fn matches_direct_gcd_on_the_measured_shapes() {
        let wide: BigInt = "9".repeat(4096).parse().unwrap();
        let wide_other: BigInt = "7".repeat(4096).parse().unwrap();
        for (a, b) in [
            (wide.clone(), BigInt::from(0)),
            (BigInt::from(0), wide.clone()),
            (wide.clone(), BigInt::from(1)),
            (wide.clone(), BigInt::from(7)),
            (wide.clone(), wide_other.clone()),
            (wide_other, wide),
            (BigInt::from(0), BigInt::from(0)),
            (BigInt::from(-462), BigInt::from(1071)),
            (BigInt::from(17), BigInt::from(17)),
        ] {
            assert_eq!(
                balanced_bigint_gcd(&a, &b),
                a.gcd(&b),
                "balanced_bigint_gcd({a}, {b}) must equal Integer::gcd"
            );
        }
    }

    proptest! {
        /// Random pairs across a spread of magnitudes, including the
        /// deliberately imbalanced widths (a `u32` against digit strings up
        /// to 512 digits) that are exactly the shape the balancing exists
        /// for. `Integer::gcd` is the oracle; this only has to agree with it
        /// everywhere, not be independently correct.
        #[test]
        fn agrees_with_integer_gcd(
            a in any::<i64>(),
            b_digits in "[0-9]{0,512}",
            b_sign in any::<bool>(),
        ) {
            let a = BigInt::from(a);
            let mut b: BigInt = if b_digits.is_empty() { BigInt::from(0) } else { b_digits.parse().unwrap() };
            if b_sign {
                b = -b;
            }
            prop_assert_eq!(balanced_bigint_gcd(&a, &b), a.gcd(&b));
        }

        /// Henrici's `add`/`sub` against the schoolbook oracle
        /// `Fraction::new(a·d ± c·b, b·d)` on wide operands, including a
        /// denominator shared by construction (`shared` multiplies both) so
        /// the `gcd(b, d) != 1` branch and its second reduction both run.
        /// The results must be equal *and* identically represented: both
        /// sides are in lowest terms with a positive denominator.
        #[test]
        fn henrici_add_sub_match_schoolbook(
            an in "-?[1-9][0-9]{0,40}",
            ad in "[1-9][0-9]{18,40}",
            bn in "-?[1-9][0-9]{0,40}",
            bd in "[1-9][0-9]{0,40}",
            shared in 1u64..1_000_000,
        ) {
            let parse = |s: &str| s.parse::<BigInt>().unwrap();
            let shared = BigInt::from(shared);
            let x = Fraction::new(parse(&an), parse(&ad) * &shared);
            let y = Fraction::new(parse(&bn), parse(&bd) * &shared);
            let (xn, xd) = x.to_bigint_pair();
            let (yn, yd) = y.to_bigint_pair();
            let sum = Fraction::new(&xn * &yd + &yn * &xd, &xd * &yd);
            let diff = Fraction::new(&xn * &yd - &yn * &xd, &xd * &yd);
            prop_assert_eq!(x.add(&y).to_bigint_pair(), sum.to_bigint_pair());
            prop_assert_eq!(x.sub(&y).to_bigint_pair(), diff.to_bigint_pair());
        }
    }
}
