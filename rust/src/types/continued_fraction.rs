use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactReal {
    Rational(Fraction),
    /// √r for a non-negative rational `r` whose value is irrational
    /// (`r` is positive and either its numerator or denominator is not a
    /// perfect square). Constructed only via `from_sqrt_rational`, which
    /// projects perfect-square and zero radicands onto `Rational` so this
    /// variant always denotes a lazy continued fraction per SPEC §4.2.2.
    AlgebraicSqrt { radicand: Fraction },
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

    /// √`radicand` as an exact real. Returns `None` for nil or negative
    /// input; returns `Rational` for zero and for perfect-square
    /// rationals; otherwise returns `AlgebraicSqrt`.
    pub fn from_sqrt_rational(radicand: Fraction) -> Option<Self> {
        if radicand.is_nil() {
            return None;
        }
        let num = radicand.numerator();
        let den = radicand.denominator();
        if num.is_negative() {
            return None;
        }
        if num.is_zero() {
            return Some(Self::Rational(Fraction::new(
                BigInt::zero(),
                BigInt::one(),
            )));
        }
        let sn = num.sqrt();
        let sd = den.sqrt();
        if &sn * &sn == num && &sd * &sd == den {
            return Some(Self::Rational(Fraction::new(sn, sd)));
        }
        Some(Self::AlgebraicSqrt { radicand })
    }

    #[inline]
    pub fn as_rational(&self) -> Option<&Fraction> {
        match self {
            Self::Rational(f) => Some(f),
            Self::AlgebraicSqrt { .. } => None,
        }
    }

    #[inline]
    pub fn to_fraction(&self) -> Option<Fraction> {
        match self {
            Self::Rational(f) => Some(f.clone()),
            Self::AlgebraicSqrt { .. } => None,
        }
    }

    #[inline]
    pub fn sqrt_radicand(&self) -> Option<&Fraction> {
        match self {
            Self::Rational(_) => None,
            Self::AlgebraicSqrt { radicand } => Some(radicand),
        }
    }

    #[inline]
    pub fn is_rational(&self) -> bool {
        matches!(self, Self::Rational(_))
    }

    #[inline]
    pub fn is_algebraic_sqrt(&self) -> bool {
        matches!(self, Self::AlgebraicSqrt { .. })
    }

    #[inline]
    pub fn is_nil(&self) -> bool {
        match self {
            Self::Rational(f) => f.is_nil(),
            Self::AlgebraicSqrt { .. } => false,
        }
    }

    #[inline]
    pub fn is_integer(&self) -> bool {
        match self {
            Self::Rational(f) => f.is_integer(),
            Self::AlgebraicSqrt { .. } => false,
        }
    }

    /// Canonical partial quotients for finite (rational) values.
    /// Returns `None` for nil and for lazy representations such as
    /// `AlgebraicSqrt`; callers that want a bounded prefix of any value
    /// should use `partial_quotients_bounded`.
    pub fn partial_quotients(&self) -> Option<Vec<BigInt>> {
        match self {
            Self::Rational(f) => {
                if f.is_nil() {
                    return None;
                }
                Some(rational_partial_quotients(f.numerator(), f.denominator()))
            }
            Self::AlgebraicSqrt { .. } => None,
        }
    }

    /// Compute up to `budget` partial quotients. For finite (rational)
    /// values the returned sequence is the canonical CF, truncated to
    /// `budget` terms when shorter. For lazy values the result has
    /// exactly `budget` terms when `budget > 0`; the prefix is
    /// canonical (the leading term may be any integer; subsequent terms
    /// are strictly positive). The prefix does not enforce SPEC §4.2.1
    /// rule 3 — a truncated lazy prefix may end in a `1` even though
    /// the full canonical sequence does not.
    pub fn partial_quotients_bounded(&self, budget: usize) -> Vec<BigInt> {
        if budget == 0 {
            return Vec::new();
        }
        match self {
            Self::Rational(f) => {
                if f.is_nil() {
                    return Vec::new();
                }
                let mut canonical = rational_partial_quotients(f.numerator(), f.denominator());
                canonical.truncate(budget);
                canonical
            }
            Self::AlgebraicSqrt { radicand } => {
                sqrt_partial_quotients_bounded(radicand, budget)
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

/// Generate up to `budget` partial quotients of √(p/q) where the
/// caller guarantees `p > 0`, `q > 0`, `gcd(p, q) = 1`, and `p * q` is
/// not a perfect square. The expansion follows the classical quadratic
/// surd recurrence on the triple `(P, Q, D)`:
///
/// ```text
/// x_i = (P_i + √D) / Q_i
/// a_i = ⌊x_i⌋
/// P_{i+1} = a_i · Q_i − P_i
/// Q_{i+1} = (D − P_{i+1}²) / Q_i
/// ```
///
/// with `D = p · q`, `P_0 = 0`, `Q_0 = q`. The invariants `Q_i > 0`,
/// `P_i ≥ 0`, and `Q_i | (D − P_i²)` are preserved at every step.
fn sqrt_partial_quotients_bounded(radicand: &Fraction, budget: usize) -> Vec<BigInt> {
    debug_assert!(budget > 0);
    debug_assert!(!radicand.is_nil());
    let p = radicand.numerator();
    let q = radicand.denominator();
    debug_assert!(p.is_positive());
    debug_assert!(q.is_positive());

    let big_d: BigInt = &p * &q;
    let sqrt_floor: BigInt = big_d.sqrt();
    debug_assert!(
        &sqrt_floor * &sqrt_floor != big_d,
        "AlgebraicSqrt must not hold a perfect-square radicand"
    );

    let mut p_i = BigInt::zero();
    let mut q_i = q.clone();
    let mut terms = Vec::with_capacity(budget);
    for _ in 0..budget {
        let a_i: BigInt = (&p_i + &sqrt_floor).div_floor(&q_i);
        let next_p: BigInt = &a_i * &q_i - &p_i;
        let next_q: BigInt = (&big_d - &next_p * &next_p) / &q_i;
        terms.push(a_i);
        p_i = next_p;
        q_i = next_q;
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

    fn sqrt_of(num: i64, den: i64) -> ExactReal {
        ExactReal::from_sqrt_rational(Fraction::new(bi(num), bi(den)))
            .expect("non-negative radicand should construct")
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

    // --- Phase 4: AlgebraicSqrt construction --------------------------------

    #[test]
    fn from_sqrt_rational_zero_returns_rational_zero() {
        let value =
            ExactReal::from_sqrt_rational(Fraction::new(bi(0), bi(1))).expect("zero");
        assert_eq!(value, rational(0, 1));
        assert!(value.is_rational());
        assert!(!value.is_algebraic_sqrt());
    }

    #[test]
    fn from_sqrt_rational_perfect_square_integer_collapses_to_rational() {
        let value =
            ExactReal::from_sqrt_rational(Fraction::new(bi(9), bi(1))).expect("9");
        assert_eq!(value, rational(3, 1));
        assert!(value.is_integer());
    }

    #[test]
    fn from_sqrt_rational_perfect_square_rational_collapses_to_rational() {
        let value =
            ExactReal::from_sqrt_rational(Fraction::new(bi(9), bi(16))).expect("9/16");
        assert_eq!(value, rational(3, 4));
    }

    #[test]
    fn from_sqrt_rational_quarter_collapses_to_one_half() {
        let value =
            ExactReal::from_sqrt_rational(Fraction::new(bi(1), bi(4))).expect("1/4");
        assert_eq!(value, rational(1, 2));
    }

    #[test]
    fn from_sqrt_rational_negative_returns_none() {
        assert_eq!(
            ExactReal::from_sqrt_rational(Fraction::new(bi(-2), bi(1))),
            None
        );
        assert_eq!(
            ExactReal::from_sqrt_rational(Fraction::new(bi(-1), bi(4))),
            None
        );
    }

    #[test]
    fn from_sqrt_rational_nil_returns_none() {
        assert_eq!(ExactReal::from_sqrt_rational(Fraction::nil()), None);
    }

    #[test]
    fn from_sqrt_rational_non_square_integer_builds_algebraic_sqrt() {
        let value = sqrt_of(2, 1);
        assert!(value.is_algebraic_sqrt());
        assert!(!value.is_rational());
        assert!(!value.is_integer());
        assert!(!value.is_nil());
        let r = value.sqrt_radicand().expect("algebraic sqrt");
        assert_eq!(r.numerator(), bi(2));
        assert_eq!(r.denominator(), bi(1));
    }

    #[test]
    fn from_sqrt_rational_non_square_rational_builds_algebraic_sqrt() {
        let value = sqrt_of(2, 3);
        assert!(value.is_algebraic_sqrt());
        let r = value.sqrt_radicand().expect("algebraic sqrt");
        assert_eq!(r.numerator(), bi(2));
        assert_eq!(r.denominator(), bi(3));
    }

    #[test]
    fn from_sqrt_rational_square_numerator_only_is_lazy() {
        // sqrt(4/3) — 4 is square, 3 is not, value is irrational
        let value = sqrt_of(4, 3);
        assert!(value.is_algebraic_sqrt());
    }

    #[test]
    fn from_sqrt_rational_square_denominator_only_is_lazy() {
        // sqrt(2/9) — 2 is not square, 9 is, value is irrational
        let value = sqrt_of(2, 9);
        assert!(value.is_algebraic_sqrt());
    }

    #[test]
    fn algebraic_sqrt_partial_quotients_unbounded_returns_none() {
        assert_eq!(sqrt_of(2, 1).partial_quotients(), None);
        assert_eq!(sqrt_of(2, 3).partial_quotients(), None);
    }

    // --- Phase 4: bounded partial-quotient expansion -----------------------

    #[test]
    fn bounded_zero_budget_returns_empty_for_any_variant() {
        assert!(rational(355, 113).partial_quotients_bounded(0).is_empty());
        assert!(sqrt_of(2, 1).partial_quotients_bounded(0).is_empty());
    }

    #[test]
    fn bounded_rational_truncates_to_budget() {
        let value = rational(355, 113);
        assert_eq!(value.partial_quotients_bounded(1), terms(&[3]));
        assert_eq!(value.partial_quotients_bounded(2), terms(&[3, 7]));
        // canonical length is 3; larger budgets do not pad.
        assert_eq!(value.partial_quotients_bounded(3), terms(&[3, 7, 16]));
        assert_eq!(value.partial_quotients_bounded(99), terms(&[3, 7, 16]));
    }

    #[test]
    fn bounded_rational_nil_returns_empty() {
        let nil = ExactReal::Rational(Fraction::nil());
        assert!(nil.partial_quotients_bounded(8).is_empty());
    }

    #[test]
    fn sqrt_two_expands_to_one_two_two_two() {
        // √2 = [1; 2, 2, 2, ...]
        let value = sqrt_of(2, 1);
        assert_eq!(
            value.partial_quotients_bounded(6),
            terms(&[1, 2, 2, 2, 2, 2])
        );
    }

    #[test]
    fn sqrt_three_expands_to_period_one_two() {
        // √3 = [1; 1, 2, 1, 2, 1, 2, ...]
        let value = sqrt_of(3, 1);
        assert_eq!(
            value.partial_quotients_bounded(7),
            terms(&[1, 1, 2, 1, 2, 1, 2])
        );
    }

    #[test]
    fn sqrt_five_expands_to_period_four() {
        // √5 = [2; 4, 4, 4, ...]
        let value = sqrt_of(5, 1);
        assert_eq!(value.partial_quotients_bounded(5), terms(&[2, 4, 4, 4, 4]));
    }

    #[test]
    fn sqrt_seven_expands_to_period_one_one_one_four() {
        // √7 = [2; 1, 1, 1, 4, 1, 1, 1, 4, ...]
        let value = sqrt_of(7, 1);
        assert_eq!(
            value.partial_quotients_bounded(9),
            terms(&[2, 1, 1, 1, 4, 1, 1, 1, 4])
        );
    }

    #[test]
    fn sqrt_thirteen_expands_to_known_period() {
        // √13 = [3; 1, 1, 1, 1, 6, 1, 1, 1, 1, 6, ...]
        let value = sqrt_of(13, 1);
        assert_eq!(
            value.partial_quotients_bounded(11),
            terms(&[3, 1, 1, 1, 1, 6, 1, 1, 1, 1, 6])
        );
    }

    #[test]
    fn sqrt_one_half_expands_correctly() {
        // √(1/2) = √2 / 2 ≈ 0.7071 = [0; 1, 2, 2, 2, ...]
        let value = sqrt_of(1, 2);
        assert_eq!(
            value.partial_quotients_bounded(6),
            terms(&[0, 1, 2, 2, 2, 2])
        );
    }

    #[test]
    fn sqrt_two_thirds_expands_correctly() {
        // √(2/3) = [0; 1, 4, 2, 4, 2, 4, 2, ...]
        let value = sqrt_of(2, 3);
        assert_eq!(
            value.partial_quotients_bounded(7),
            terms(&[0, 1, 4, 2, 4, 2, 4])
        );
    }

    #[test]
    fn sqrt_two_ninths_expands_correctly() {
        // √(2/9) = √2 / 3 ≈ 0.4714 = [0; 2, 8, 4, 8, 4, ...]
        let value = sqrt_of(2, 9);
        assert_eq!(
            value.partial_quotients_bounded(7),
            terms(&[0, 2, 8, 4, 8, 4, 8])
        );
    }

    #[test]
    fn sqrt_four_thirds_expands_correctly() {
        // √(4/3) = 2/√3 ≈ 1.1547 = [1; 6, 2, 6, 2, ...]
        // Derivation: D=12, P_0=0, Q_0=3.
        //   a_0 = ⌊3/3⌋ = 1, P_1 = 3, Q_1 = (12-9)/3 = 1
        //   a_1 = ⌊(3+3)/1⌋ = 6, P_2 = 3, Q_2 = (12-9)/1 = 3
        //   a_2 = ⌊(3+3)/3⌋ = 2, P_3 = 3, Q_3 = (12-9)/3 = 1
        //   period [6, 2]
        let value = sqrt_of(4, 3);
        assert_eq!(
            value.partial_quotients_bounded(6),
            terms(&[1, 6, 2, 6, 2, 6])
        );
    }

    #[test]
    fn sqrt_two_is_not_equal_to_sqrt_three() {
        assert_ne!(sqrt_of(2, 1), sqrt_of(3, 1));
    }

    #[test]
    fn sqrt_two_is_equal_to_itself() {
        assert_eq!(sqrt_of(2, 1), sqrt_of(2, 1));
    }

    #[test]
    fn rational_and_algebraic_sqrt_are_not_structurally_equal() {
        // Note: this is *structural* equality on the ExactReal repr, not
        // value equality on the canonical CF. The latter is the subject
        // of Phase 5+ comparison work.
        assert_ne!(rational(1, 1), sqrt_of(2, 1));
    }

    #[test]
    fn sqrt_radicand_returns_none_for_rational() {
        assert!(rational(3, 2).sqrt_radicand().is_none());
    }

    #[test]
    fn bounded_budget_one_returns_first_term_only() {
        // Useful for AI-readable display truncation. Each variant
        // returns the integer part as its first bounded term.
        assert_eq!(sqrt_of(2, 1).partial_quotients_bounded(1), terms(&[1]));
        assert_eq!(sqrt_of(7, 1).partial_quotients_bounded(1), terms(&[2]));
        assert_eq!(sqrt_of(1, 2).partial_quotients_bounded(1), terms(&[0]));
    }
}
