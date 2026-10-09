//! Absence at the `Fraction` layer: a denominator of 0.
//!
//! A quotient by zero is absent *as a number*. `a/b ÷ 0` is `a/(b·0)`, so
//! the pair keeps the dividend's numerator over the zero that refused it:
//! `100 0 DIV` is `100/0`, `1/2 0 DIV` is `1/0`, and `0/0` is what `0 0 DIV`
//! alone leaves. The denominator is the whole of what makes the pair absent
//! ([`Fraction::is_nil`]); the numerator is what the machine keeps, never
//! what a program can read — an absent pair compares, hashes and displays
//! as any absence does. Arithmetic passes an absent operand through as it
//! is, so what a zero divisor refused to divide travels through the Words
//! after it unchanged.

use super::fraction::{Fraction, FractionRepr};
use num_bigint::BigInt;
use num_traits::Zero;

impl Fraction {
    /// `0/0`: the quotient of zero by zero, and the pair a test writes for an
    /// absent lane. Nothing else mints it — an absence that was never a
    /// division holds no pair at all (`ValueData::Nil`).
    #[inline]
    pub fn nil() -> Self {
        Fraction {
            repr: FractionRepr::Small(0, 0),
        }
    }

    /// Whether this number is absent: its denominator is 0, whatever the
    /// numerator holds.
    #[inline]
    pub fn is_nil(&self) -> bool {
        match &self.repr {
            FractionRepr::Small(_, d) => *d == 0,
            FractionRepr::Big(big) => big.denominator.is_zero(),
        }
    }

    /// `self` over a zero denominator: what dividing `self` by zero stores.
    /// An absent `self` is already over zero and is answered as it is.
    #[inline]
    pub fn over_zero(&self) -> Fraction {
        match &self.repr {
            FractionRepr::Small(n, _) => Fraction::from_repr(FractionRepr::Small(*n, 0)),
            FractionRepr::Big(big) => {
                Fraction::from_repr(FractionRepr::big(big.numerator.clone(), BigInt::zero()))
            }
        }
    }

    /// The absent operand a law passes through, leftmost first, as it is —
    /// pair and all. `None` when both operands are present.
    #[inline]
    pub(crate) fn absent_operand(&self, other: &Fraction) -> Option<Fraction> {
        if self.is_nil() {
            return Some(self.clone());
        }
        if other.is_nil() {
            return Some(other.clone());
        }
        None
    }
}
