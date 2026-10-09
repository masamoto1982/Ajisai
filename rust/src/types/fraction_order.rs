//! The order of the exact scalars (LANG.VALUES.EXACT): the field is totally
//! ordered, `-1/0` lies below it and `1/0` above, and `0/0` is ordered against
//! nothing — the one comparison the exact domain does not decide. `order`
//! answers `None` exactly there, and every Word that asks for an order projects
//! `domainMiss` on that answer.

use super::fraction::Fraction;
use num_bigint::BigInt;

impl Fraction {
    /// The order of two numbers, when both are ordered: `-1/0` is below
    /// every rational and `1/0` above, and `0/0` is ordered against nothing,
    /// not even itself — the one comparison the exact domain does not decide
    /// (LANG.VALUES.EXACT).
    pub fn order(&self, other: &Fraction) -> Option<std::cmp::Ordering> {
        if self.is_nullity() || other.is_nullity() {
            return None;
        }
        if !self.is_finite() || !other.is_finite() {
            // The sign over zero orders against anything: the sign pairs
            // `(±1, 0)` and `(s, 1)` compare as `±1` against `s`'s side of 0,
            // which is exactly what `cmp_finite` of the sign pairs reads.
            let (a, b) = (self.sign_lane(), other.sign_lane());
            return Some(a.cmp(&b));
        }
        Some(self.cmp_finite(other))
    }

    /// Where a number sits relative to the three points: -2 for `-1/0`,
    /// -1, 0 and 1 for the sign of a rational, and 2 for `1/0`.
    fn sign_lane(&self) -> i64 {
        let sign = match self.signum() {
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Less => -1,
        };
        if self.is_finite() {
            sign
        } else {
            sign * 2
        }
    }

    /// The order of two rationals. Both operands must be finite.
    pub(crate) fn cmp_finite(&self, other: &Fraction) -> std::cmp::Ordering {
        debug_assert!(self.is_finite() && other.is_finite());
        if let (Some((a, b)), Some((c, d))) = (self.extract_i64_pair(), other.extract_i64_pair()) {
            if b == d {
                return a.cmp(&c);
            }
            let lhs = (a as i128) * (d as i128);
            let rhs = (c as i128) * (b as i128);
            return lhs.cmp(&rhs);
        }
        let (an, ad): (BigInt, BigInt) = self.to_bigint_pair();
        let (bn, bd): (BigInt, BigInt) = other.to_bigint_pair();
        if ad == bd {
            return an.cmp(&bn);
        }
        let lhs: BigInt = an * &bd;
        let rhs: BigInt = bn * &ad;
        lhs.cmp(&rhs)
    }
}

impl PartialOrd for Fraction {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.order(other)
    }
}
