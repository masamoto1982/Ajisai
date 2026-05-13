use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactReal {
    Rational(Fraction),
}

impl ExactReal {
    #[inline]
    pub fn from_fraction(f: Fraction) -> Self {
        Self::Rational(f)
    }

    #[inline]
    pub fn from_integer(n: i64) -> Self {
        Self::Rational(Fraction::new(BigInt::from(n), BigInt::one()))
    }

    #[inline]
    pub fn from_bigint(n: BigInt) -> Self {
        Self::Rational(Fraction::new(n, BigInt::one()))
    }

    #[inline]
    pub fn as_rational(&self) -> Option<&Fraction> {
        match self {
            Self::Rational(f) => Some(f),
        }
    }

    #[inline]
    pub fn to_fraction(&self) -> Option<Fraction> {
        match self {
            Self::Rational(f) => Some(f.clone()),
        }
    }

    #[inline]
    pub fn is_rational(&self) -> bool {
        matches!(self, Self::Rational(_))
    }

    #[inline]
    pub fn is_nil(&self) -> bool {
        match self {
            Self::Rational(f) => f.is_nil(),
        }
    }

    #[inline]
    pub fn is_integer(&self) -> bool {
        match self {
            Self::Rational(f) => f.is_integer(),
        }
    }

    pub fn partial_quotients(&self) -> Option<Vec<BigInt>> {
        match self {
            Self::Rational(f) => {
                if f.is_nil() {
                    return None;
                }
                Some(rational_partial_quotients(f.numerator(), f.denominator()))
            }
        }
    }

    pub fn from_partial_quotients(terms: &[BigInt]) -> Option<Self> {
        if terms.is_empty() {
            return None;
        }
        for term in terms.iter().skip(1) {
            if !term.is_positive() {
                return None;
            }
        }

        let mut iter = terms.iter().rev();
        let last = iter.next().expect("non-empty by length check above");
        let mut num: BigInt = last.clone();
        let mut den: BigInt = BigInt::one();
        for a in iter {
            let new_num = a * &num + &den;
            let new_den = num;
            num = new_num;
            den = new_den;
        }
        Some(Self::Rational(Fraction::new(num, den)))
    }
}

fn rational_partial_quotients(mut num: BigInt, mut den: BigInt) -> Vec<BigInt> {
    debug_assert!(!den.is_zero());

    if den.is_negative() {
        num = -num;
        den = -den;
    }

    let mut terms: Vec<BigInt> = Vec::new();

    loop {
        let (q, r) = num.div_mod_floor(&den);
        terms.push(q);
        if r.is_zero() {
            break;
        }
        num = den;
        den = r;
    }

    if terms.len() >= 2 && terms.last().expect("non-empty").is_one() {
        let popped = terms.pop().expect("just checked length >= 2");
        *terms.last_mut().expect("length >= 1 after pop") += popped;
    }

    terms
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bi(n: i64) -> BigInt {
        BigInt::from(n)
    }

    fn terms(seq: &[i64]) -> Vec<BigInt> {
        seq.iter().map(|&n| BigInt::from(n)).collect()
    }

    fn rational(num: i64, den: i64) -> ExactReal {
        ExactReal::Rational(Fraction::new(bi(num), bi(den)))
    }

    #[test]
    fn integer_three_expands_to_single_term() {
        assert_eq!(rational(3, 1).partial_quotients(), Some(terms(&[3])));
    }

    #[test]
    fn three_halves_expands_to_one_two() {
        assert_eq!(rational(3, 2).partial_quotients(), Some(terms(&[1, 2])));
    }

    #[test]
    fn one_half_expands_to_zero_two() {
        assert_eq!(rational(1, 2).partial_quotients(), Some(terms(&[0, 2])));
    }

    #[test]
    fn negative_three_halves_expands_via_floor_division() {
        assert_eq!(rational(-3, 2).partial_quotients(), Some(terms(&[-2, 2])));
    }

    #[test]
    fn negative_one_half_expands_via_floor_division() {
        assert_eq!(rational(-1, 2).partial_quotients(), Some(terms(&[-1, 2])));
    }

    #[test]
    fn three_seven_sixteen_expands_for_355_113() {
        assert_eq!(
            rational(355, 113).partial_quotients(),
            Some(terms(&[3, 7, 16]))
        );
    }

    #[test]
    fn zero_expands_to_single_zero() {
        assert_eq!(rational(0, 1).partial_quotients(), Some(terms(&[0])));
    }

    #[test]
    fn negative_integer_expands_to_single_term() {
        assert_eq!(rational(-5, 1).partial_quotients(), Some(terms(&[-5])));
    }

    #[test]
    fn negative_denominator_is_normalized_before_expansion() {
        assert_eq!(rational(3, -2).partial_quotients(), Some(terms(&[-2, 2])));
    }

    #[test]
    fn trailing_one_is_folded_into_previous_term() {
        let f = ExactReal::Rational(Fraction::new(bi(2), bi(1)));
        assert_eq!(f.partial_quotients(), Some(terms(&[2])));
    }

    #[test]
    fn from_partial_quotients_evaluates_canonical_sequence() {
        let reconstructed = ExactReal::from_partial_quotients(&terms(&[3, 7, 16]));
        assert_eq!(reconstructed, Some(rational(355, 113)));
    }

    #[test]
    fn from_partial_quotients_evaluates_single_term() {
        assert_eq!(
            ExactReal::from_partial_quotients(&terms(&[3])),
            Some(rational(3, 1))
        );
    }

    #[test]
    fn from_partial_quotients_evaluates_negative_leading_term() {
        assert_eq!(
            ExactReal::from_partial_quotients(&terms(&[-2, 2])),
            Some(rational(-3, 2))
        );
    }

    #[test]
    fn from_partial_quotients_accepts_non_canonical_trailing_one() {
        assert_eq!(
            ExactReal::from_partial_quotients(&terms(&[1, 1])),
            Some(rational(2, 1))
        );
    }

    #[test]
    fn from_partial_quotients_rejects_empty_input() {
        assert_eq!(ExactReal::from_partial_quotients(&[]), None);
    }

    #[test]
    fn from_partial_quotients_rejects_non_positive_after_first() {
        assert_eq!(
            ExactReal::from_partial_quotients(&terms(&[1, 0])),
            None
        );
        assert_eq!(
            ExactReal::from_partial_quotients(&terms(&[1, -2])),
            None
        );
    }

    #[test]
    fn round_trip_canonical_sequence_is_idempotent() {
        let value = rational(355, 113);
        let canonical = value.partial_quotients().expect("rational");
        let reconstructed =
            ExactReal::from_partial_quotients(&canonical).expect("valid sequence");
        let canonical_again = reconstructed.partial_quotients().expect("rational");
        assert_eq!(canonical, canonical_again);
    }

    #[test]
    fn round_trip_negative_value_is_idempotent() {
        let value = rational(-22, 7);
        let canonical = value.partial_quotients().expect("rational");
        let reconstructed =
            ExactReal::from_partial_quotients(&canonical).expect("valid sequence");
        let canonical_again = reconstructed.partial_quotients().expect("rational");
        assert_eq!(canonical, canonical_again);
    }

    #[test]
    fn nil_fraction_has_no_partial_quotients() {
        let nil = ExactReal::Rational(Fraction::nil());
        assert!(nil.is_nil());
        assert_eq!(nil.partial_quotients(), None);
    }

    #[test]
    fn from_integer_round_trips_through_fraction() {
        let value = ExactReal::from_integer(42);
        assert!(value.is_integer());
        assert!(!value.is_nil());
        let f = value.to_fraction().expect("rational");
        assert_eq!(f.numerator(), bi(42));
        assert_eq!(f.denominator(), bi(1));
    }
}
