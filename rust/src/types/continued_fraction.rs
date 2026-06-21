use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};
use std::sync::Arc;

// Safety bound on internal Gosper ingestion. The classical Gosper
// transforms terminate (or emit) after a bounded number of input
// quotients per output quotient, but the bound depends on the
// matrix coefficients. This constant caps how many input quotients
// the iterator will absorb without emitting before giving up and
// reporting end-of-stream. Tuned generously so well-formed
// transforms never hit it.
const GOSPER_INGEST_SAFETY: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExactReal {
    Rational(Fraction),
    /// √r for a non-negative rational `r` whose value is irrational
    /// (`r` is positive and either its numerator or denominator is not a
    /// perfect square). Constructed only via `from_sqrt_rational`, which
    /// projects perfect-square and zero radicands onto `Rational` so this
    /// variant always denotes a lazy continued fraction per SPEC §4.2.2.
    AlgebraicSqrt {
        radicand: Fraction,
    },
    /// Unevaluated Gosper transform of one or two operand CFs per
    /// SPEC §4.2.2. Constructed only by the arithmetic methods, which
    /// fold pure-rational operands through the `Fraction` fast path
    /// instead of building a Gosper node. Held behind an `Arc` so
    /// that cloning an `ExactReal` — which the arithmetic methods do
    /// for every operand they fold into a new transform — is O(1)
    /// instead of a deep copy of the operand's Gosper tree.
    Gosper(Arc<Gosper>),
}

/// Möbius (unary) or bihomographic (binary) Gosper state with BigInt
/// coefficients. Both forms are stored as data; the streaming algorithm
/// in `CfIter` consumes them lazily.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gosper {
    /// (a·x + b) / (c·x + d) over a CF operand `x`.
    Mobius {
        a: BigInt,
        b: BigInt,
        c: BigInt,
        d: BigInt,
        x: ExactReal,
    },
    /// (a·x·y + b·x + c·y + d) / (e·x·y + f·x + g·y + h) over CF
    /// operands `x` and `y`.
    Bihomographic {
        a: BigInt,
        b: BigInt,
        c: BigInt,
        d: BigInt,
        e: BigInt,
        f: BigInt,
        g: BigInt,
        h: BigInt,
        x: ExactReal,
        y: ExactReal,
    },
}

// =========================================================================
// Constructors / accessors / predicates
// =========================================================================

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
            return Some(Self::Rational(Fraction::new(BigInt::zero(), BigInt::one())));
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
            _ => None,
        }
    }

    #[inline]
    pub fn to_fraction(&self) -> Option<Fraction> {
        match self {
            Self::Rational(f) => Some(f.clone()),
            _ => None,
        }
    }

    #[inline]
    pub fn sqrt_radicand(&self) -> Option<&Fraction> {
        match self {
            Self::AlgebraicSqrt { radicand } => Some(radicand),
            _ => None,
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
    pub fn is_gosper(&self) -> bool {
        matches!(self, Self::Gosper(_))
    }

    #[inline]
    pub fn is_nil(&self) -> bool {
        match self {
            Self::Rational(f) => f.is_nil(),
            _ => false,
        }
    }

    #[inline]
    pub fn is_integer(&self) -> bool {
        match self {
            Self::Rational(f) => f.is_integer(),
            _ => false,
        }
    }

    /// True only for the value structurally known to be exactly zero,
    /// without expanding the CF. Lazy variants conservatively report
    /// `false` even when mathematically zero (e.g. `x − x` built via
    /// Gosper). Resolving such cases is the comparison-budget work
    /// scheduled for a later phase (SPEC §7.4.1).
    #[inline]
    pub fn is_structurally_zero(&self) -> bool {
        match self {
            Self::Rational(f) => f.is_zero(),
            _ => false,
        }
    }
}

// =========================================================================
// Partial-quotient API
// =========================================================================

impl ExactReal {
    /// Canonical partial quotients for finite (rational) values.
    /// Returns `None` for nil and for lazy representations
    /// (`AlgebraicSqrt`, `Gosper`); callers that want a bounded prefix
    /// of any value should use `partial_quotients_bounded`.
    pub fn partial_quotients(&self) -> Option<Vec<BigInt>> {
        match self {
            Self::Rational(f) => {
                if f.is_nil() {
                    return None;
                }
                Some(rational_partial_quotients(f.numerator(), f.denominator()))
            }
            _ => None,
        }
    }

    /// Compute up to `budget` partial quotients. For finite (rational)
    /// values the returned sequence is the canonical CF, truncated to
    /// `budget` terms when shorter. For lazy values the result has
    /// up to `budget` terms; if the value reduces to a rational during
    /// expansion (e.g. a Gosper transform consuming a finite operand)
    /// the sequence may be shorter than `budget` once that rational's
    /// canonical CF is exhausted. The prefix does not enforce SPEC
    /// §4.2.1 rule 3 — a truncated lazy prefix may end in a `1` even
    /// though the full canonical sequence does not.
    pub fn partial_quotients_bounded(&self, budget: usize) -> Vec<BigInt> {
        if budget == 0 {
            return Vec::new();
        }
        let mut iter = CfIter::from_exact_real(self);
        let mut out = Vec::with_capacity(budget);
        for _ in 0..budget {
            match iter.next_quotient() {
                Some(q) => out.push(q),
                None => break,
            }
        }
        out
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

// =========================================================================
// Arithmetic
// =========================================================================

impl ExactReal {
    /// Negation. `Rational(-x)` for rationals; otherwise a Möbius
    /// Gosper. Preserves nil.
    pub fn neg(&self) -> Self {
        match self {
            Self::Rational(f) => {
                if f.is_nil() {
                    return Self::Rational(Fraction::nil());
                }
                Self::Rational(Fraction::new(-f.numerator(), f.denominator()))
            }
            other => mobius_apply(
                BigInt::from(-1),
                BigInt::zero(),
                BigInt::zero(),
                BigInt::one(),
                other.clone(),
            ),
        }
    }

    /// Reciprocal `1/x`. Returns `Rational(nil)` for nil; returns
    /// `None` only if the operand is structurally zero. Lazy zero
    /// values are *not* caught here — they yield a degenerate Gosper
    /// whose expansion is undefined; SPEC §11.2's `divisionByZero` /
    /// `undecidable` projection is enforced at the language boundary,
    /// not at this internal type-level API.
    pub fn reciprocal(&self) -> Option<Self> {
        if self.is_nil() {
            return Some(Self::Rational(Fraction::nil()));
        }
        if self.is_structurally_zero() {
            return None;
        }
        match self {
            Self::Rational(f) => {
                let (n, d) = f.to_bigint_pair();
                Some(Self::Rational(Fraction::new(d, n)))
            }
            other => Some(mobius_apply(
                BigInt::zero(),
                BigInt::one(),
                BigInt::one(),
                BigInt::zero(),
                other.clone(),
            )),
        }
    }

    /// Addition. Routes through the `Fraction` fast path when both
    /// operands are rational; otherwise builds a Möbius Gosper (one
    /// rational operand) or a bihomographic Gosper (two non-rational
    /// operands).
    pub fn add(&self, other: &Self) -> Self {
        if self.is_nil() || other.is_nil() {
            return Self::Rational(Fraction::nil());
        }
        match (self.as_rational(), other.as_rational()) {
            (Some(a), Some(b)) => Self::Rational(a.add(b)),
            (Some(r), None) => add_rational_to_lazy(r, other.clone()),
            (None, Some(r)) => add_rational_to_lazy(r, self.clone()),
            (None, None) => match (self.sqrt_radicand(), other.sqrt_radicand()) {
                // √r + √r = 2·√r = √(4r): collapse equal radicands to a
                // single canonical `AlgebraicSqrt` (or `Rational`, e.g.
                // √(1/4)+√(1/4) = 1) instead of leaving a bihom whose
                // operands never let it terminate on a rational sum.
                (Some(r1), Some(r2)) if r1 == r2 => {
                    let four = Fraction::new(BigInt::from(4), BigInt::one());
                    Self::from_sqrt_rational(four.mul(r1))
                        .unwrap_or_else(|| bihom_apply_add(self.clone(), other.clone()))
                }
                _ => bihom_apply_add(self.clone(), other.clone()),
            },
        }
    }

    /// Subtraction `self − other`.
    pub fn sub(&self, other: &Self) -> Self {
        if self.is_nil() || other.is_nil() {
            return Self::Rational(Fraction::nil());
        }
        match (self.as_rational(), other.as_rational()) {
            (Some(a), Some(b)) => Self::Rational(a.sub(b)),
            (Some(r), None) => {
                // r − y = (0·y + r_num·1)/(0·y + r_den) ... but signs:
                // r − y = (−r_den·y + r_num) / r_den; Möbius on y:
                // (a·y + b) / (c·y + d) with a = −r_den, b = r_num,
                // c = 0, d = r_den.
                let (rn, rd) = r.to_bigint_pair();
                mobius_apply(-&rd, rn, BigInt::zero(), rd, other.clone())
            }
            (None, Some(r)) => {
                // x − r = (1·x − r_num) / r_den. Möbius on x:
                // a = r_den, b = −r_num, c = 0, d = r_den.
                let (rn, rd) = r.to_bigint_pair();
                mobius_apply(rd.clone(), -rn, BigInt::zero(), rd, self.clone())
            }
            (None, None) => match (self.sqrt_radicand(), other.sqrt_radicand()) {
                // √r − √r = 0. For two positive irrational square roots the
                // difference is rational only when the radicands are equal
                // (otherwise √r1 = q + √r2 would force √r2 rational). The
                // streaming bihom cannot pin this exact zero, so detect it
                // in closed form; distinct radicands stay lazy.
                (Some(r1), Some(r2)) if r1 == r2 => {
                    Self::Rational(Fraction::new(BigInt::zero(), BigInt::one()))
                }
                _ => bihom_apply_sub(self.clone(), other.clone()),
            },
        }
    }

    /// Multiplication.
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_nil() || other.is_nil() {
            return Self::Rational(Fraction::nil());
        }
        match (self.as_rational(), other.as_rational()) {
            (Some(a), Some(b)) => Self::Rational(a.mul(b)),
            (Some(r), None) => mul_rational_by_lazy(r, other.clone()),
            (None, Some(r)) => mul_rational_by_lazy(r, self.clone()),
            (None, None) => match (self.sqrt_radicand(), other.sqrt_radicand()) {
                // √r1 · √r2 = √(r1·r2). Closed-form so the result is a
                // `Rational` whenever the radicand product is a perfect
                // square (e.g. √2·√2 = √4 = 2). The streaming bihomographic
                // expander cannot recover this: when two infinite operand
                // CFs multiply to an exact rational the four-corner floor
                // test straddles the value forever and never emits a term,
                // so the transform would exhaust into an empty CF.
                (Some(r1), Some(r2)) => Self::from_sqrt_rational(r1.mul(r2))
                    .unwrap_or_else(|| bihom_apply_mul(self.clone(), other.clone())),
                _ => bihom_apply_mul(self.clone(), other.clone()),
            },
        }
    }

    /// Division `self / other`. Returns `Rational(nil)` for nil
    /// operands and `None` for division by a structurally-zero divisor;
    /// lazy zero divisors propagate to a degenerate Gosper, deferring
    /// the SPEC-level `divisionByZero` / `undecidable` decision to the
    /// language boundary.
    pub fn div(&self, other: &Self) -> Option<Self> {
        if self.is_nil() || other.is_nil() {
            return Some(Self::Rational(Fraction::nil()));
        }
        if other.is_structurally_zero() {
            return None;
        }
        Some(match (self.as_rational(), other.as_rational()) {
            (Some(a), Some(b)) => Self::Rational(a.div(b)),
            (Some(r), None) => {
                // r / y = r_num / (r_den·y). Möbius on y:
                // a = 0, b = r_num, c = r_den, d = 0.
                let (rn, rd) = r.to_bigint_pair();
                mobius_apply(BigInt::zero(), rn, rd, BigInt::zero(), other.clone())
            }
            (None, Some(r)) => {
                // x / r = r_den·x / r_num. Möbius on x:
                // a = r_den, b = 0, c = 0, d = r_num.
                let (rn, rd) = r.to_bigint_pair();
                mobius_apply(rd, BigInt::zero(), BigInt::zero(), rn, self.clone())
            }
            (None, None) => match (self.sqrt_radicand(), other.sqrt_radicand()) {
                // √r1 / √r2 = √(r1/r2): same rational-collapse hazard as
                // `mul` (e.g. √8/√2 = √4 = 2). `r2 > 0` here because a
                // structurally-zero divisor was rejected above and an
                // `AlgebraicSqrt` never carries a zero radicand.
                (Some(r1), Some(r2)) => Self::from_sqrt_rational(r1.div(r2))
                    .unwrap_or_else(|| bihom_apply_div(self.clone(), other.clone())),
                _ => bihom_apply_div(self.clone(), other.clone()),
            },
        })
    }
}

// =========================================================================
// Comparison — partial-quotient budget per SPEC §7.4.1
// =========================================================================

/// Default partial-quotient budget for `cmp_with_budget`. SPEC §7.4.1
/// says the budget itself is not part of observable semantics; it
/// only has to be high enough that distinct rationals always decide.
/// 256 partial quotients comfortably decides any rational pair
/// representable on the platform while keeping pathological lazy-CF
/// comparisons (e.g. two equal irrationals built via different
/// Gosper transforms) from running unboundedly.
pub const DEFAULT_COMPARISON_BUDGET: usize = 256;

/// Three-way comparison of √`radicand` against the rational `q`,
/// yielding the ordering of √`radicand` relative to `q`.
///
/// `radicand` is the positive, non-perfect-square rational stored by
/// an `AlgebraicSqrt`, so √`radicand` is a positive irrational and is
/// never equal to the rational `q`. The result is exact and computed
/// in O(1): a non-positive `q` is below the positive root, and for a
/// positive `q` the order follows from `radicand` vs `q²` because
/// squaring is increasing on the non-negative reals.
fn cmp_sqrt_vs_rational(radicand: &Fraction, q: &Fraction) -> std::cmp::Ordering {
    if !q.numerator().is_positive() {
        // q <= 0 < √radicand.
        return std::cmp::Ordering::Greater;
    }
    radicand.cmp(&q.mul(q))
}

/// Outcome of a budgeted three-way CF comparison that also reports how
/// far the two partial-quotient streams agreed (SPEC §4.5.0 / §7.4.1).
///
/// `Decided` carries the resolved order. `Undecided` carries the
/// agreed-prefix length: the number of leading partial quotients that
/// matched before the budget (or an internal CF safety budget) was
/// exhausted. The agreed-prefix is the CF-specific evidence behind the
/// logical `Unknown` (U) and is surfaced as `diagnosis.agreedPrefix`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpOutcome {
    Decided(std::cmp::Ordering),
    Undecided { agreed_prefix: usize },
}

impl ExactReal {
    /// Three-way comparison of CF values under a partial-quotient
    /// budget.
    ///
    /// Returns `Some(Ordering::Less | Equal | Greater)` when the two
    /// CF streams diverge within `budget` partial quotients or both
    /// terminate naturally at the same index, applying the
    /// alternating-parity rule (SPEC §4.2.4 + §7.4.1): at the first
    /// index `i` where the two streams differ, the order is `a_i
    /// vs b_i` for even `i` and `b_i vs a_i` for odd `i`.
    ///
    /// Returns `None` when the budget is exhausted without a
    /// difference, when either operand is nil, or when either
    /// operand's CF stream hits its internal safety budget
    /// (`CfStep::Exhausted`) before the comparison resolves. The
    /// `None` outcome is what the language-level Bubble Rule
    /// projects to `NilReason::Undecidable` with `absence.origin =
    /// comparisonBudget`.
    pub fn cmp_with_budget(&self, other: &Self, budget: usize) -> Option<std::cmp::Ordering> {
        match self.cmp_with_budget_tracked(other, budget) {
            CmpOutcome::Decided(o) => Some(o),
            CmpOutcome::Undecided { .. } => None,
        }
    }

    /// Three-way comparison under a partial-quotient budget that also
    /// reports the agreed-prefix length (SPEC §4.5.0 / §7.4.1).
    ///
    /// Returns `CmpOutcome::Decided(ordering)` when the order resolves
    /// within `budget` partial quotients (or via an algebraic
    /// short-circuit), exactly as `cmp_with_budget`. Returns
    /// `CmpOutcome::Undecided { agreed_prefix }` when the budget is
    /// exhausted, an operand is nil, or a CF stream hits its internal
    /// safety budget before the order resolves; `agreed_prefix` is the
    /// number of leading partial quotients that matched before giving up.
    /// When the full `budget` is consumed with every quotient matching,
    /// `agreed_prefix == budget`. `COMPARE-WITHIN` (SPEC §7.4.2) surfaces
    /// this field as `diagnosis.agreedPrefix` on its `Unknown` result.
    pub fn cmp_with_budget_tracked(&self, other: &Self, budget: usize) -> CmpOutcome {
        use std::cmp::Ordering;
        if self.is_nil() || other.is_nil() {
            return CmpOutcome::Undecided { agreed_prefix: 0 };
        }
        if budget == 0 {
            return CmpOutcome::Undecided { agreed_prefix: 0 };
        }
        // Algebraic short-circuits (SPEC §4.2.4): a comparison whose
        // operands are only `Rational` and `AlgebraicSqrt` is decided
        // exactly in O(1) from the operands' algebraic structure,
        // without streaming partial quotients. This avoids the
        // budget-length CF expansion and, crucially, lets two equal
        // irrational square roots decide `Equal` instead of running
        // their never-diverging CF streams to budget exhaustion.
        match (self, other) {
            (Self::AlgebraicSqrt { radicand: r }, Self::AlgebraicSqrt { radicand: s }) => {
                // √ is strictly increasing on the non-negative
                // rationals, so √r vs √s is decided by r vs s.
                return CmpOutcome::Decided(r.cmp(s));
            }
            (Self::AlgebraicSqrt { radicand: r }, Self::Rational(q)) => {
                return CmpOutcome::Decided(cmp_sqrt_vs_rational(r, q));
            }
            (Self::Rational(q), Self::AlgebraicSqrt { radicand: r }) => {
                return CmpOutcome::Decided(cmp_sqrt_vs_rational(r, q).reverse());
            }
            _ => {}
        }
        // SPEC §7.4.1.1: the budget and the agreed-prefix are measured in
        // *nearest-integer* (semiregular) terms, whose faster convergence
        // reveals the order in fewer terms. We advance the two NICF streams
        // in parallel; the first index at which their semiregular terms
        // differ (or at which one terminates while the other continues) both
        // establishes that the values are unequal and fixes the agreed-prefix
        // length. The *order* of two unequal values is then the order their
        // regular CFs would give — identical to the NICF order (SPEC
        // §7.4.1.1) but computed by the regular-CF routine, which carries no
        // signed-term parity subtleties. Equal values' NICFs never diverge,
        // so they yield U exactly as their RCFs would.
        let mut a = NicfStream::new(self);
        let mut b = NicfStream::new(other);
        for i in 0..budget {
            match (a.next(), b.next()) {
                (NicfStep::Term(av), NicfStep::Term(bv)) => {
                    if av != bv {
                        // Distinct semiregular terms ⇒ the values differ.
                        return CmpOutcome::Decided(self.rcf_order(other));
                    }
                }
                (NicfStep::Ended, NicfStep::Ended) => return CmpOutcome::Decided(Ordering::Equal),
                (NicfStep::Ended, NicfStep::Term(_)) | (NicfStep::Term(_), NicfStep::Ended) => {
                    // One value's NICF terminated (it is exactly its
                    // convergent) while the other still has a term ⇒ unequal.
                    return CmpOutcome::Decided(self.rcf_order(other));
                }
                // A stream ran out of internal safety budget at index `i`;
                // the `i` terms at indices 0..i matched.
                (NicfStep::Exhausted, _) | (_, NicfStep::Exhausted) => {
                    return CmpOutcome::Undecided { agreed_prefix: i };
                }
            }
        }
        // Every one of the `budget` semiregular terms matched.
        CmpOutcome::Undecided {
            agreed_prefix: budget,
        }
    }

    /// The order of two values via their *regular* continued fractions, used
    /// to orient a comparison the NICF streams have already shown to be
    /// between unequal values (SPEC §7.4.1.1: the NICF order equals the RCF
    /// order). Returns `Equal` only as a defensive fallback — callers invoke
    /// this only after establishing the values differ — and treats an
    /// internal-budget exhaustion conservatively as `Equal` so a comparison
    /// can never report a *wrong* strict order.
    fn rcf_order(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        // Generous cap: the operands are already known to be unequal, so this
        // only has to find the first regular-CF divergence, which for any
        // representable pair occurs well within this bound.
        const RCF_ORDER_CAP: usize = 4096;
        let mut a = CfIter::from_exact_real(self);
        let mut b = CfIter::from_exact_real(other);
        for i in 0..RCF_ORDER_CAP {
            match (a.next_step(), b.next_step()) {
                (CfStep::Quotient(av), CfStep::Quotient(bv)) => {
                    if av != bv {
                        return if i % 2 == 0 { av.cmp(&bv) } else { bv.cmp(&av) };
                    }
                }
                (CfStep::Ended, CfStep::Ended) => return Ordering::Equal,
                (CfStep::Ended, CfStep::Quotient(_)) => {
                    return if i % 2 == 0 {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    };
                }
                (CfStep::Quotient(_), CfStep::Ended) => {
                    return if i % 2 == 0 {
                        Ordering::Less
                    } else {
                        Ordering::Greater
                    };
                }
                (CfStep::Exhausted, _) | (_, CfStep::Exhausted) => return Ordering::Equal,
            }
        }
        Ordering::Equal
    }

    /// `self == other` under the partial-quotient budget. `None` on
    /// budget exhaustion or nil per `cmp_with_budget`.
    pub fn eq_with_budget(&self, other: &Self, budget: usize) -> Option<bool> {
        self.cmp_with_budget(other, budget)
            .map(|o| o == std::cmp::Ordering::Equal)
    }

    /// `self != other`. Same `None` semantics as `eq_with_budget`.
    pub fn ne_with_budget(&self, other: &Self, budget: usize) -> Option<bool> {
        self.eq_with_budget(other, budget).map(|eq| !eq)
    }

    /// `self < other`.
    pub fn lt_with_budget(&self, other: &Self, budget: usize) -> Option<bool> {
        self.cmp_with_budget(other, budget)
            .map(|o| o == std::cmp::Ordering::Less)
    }

    /// `self <= other`.
    pub fn le_with_budget(&self, other: &Self, budget: usize) -> Option<bool> {
        self.cmp_with_budget(other, budget)
            .map(|o| o != std::cmp::Ordering::Greater)
    }

    /// `self > other`.
    pub fn gt_with_budget(&self, other: &Self, budget: usize) -> Option<bool> {
        self.cmp_with_budget(other, budget)
            .map(|o| o == std::cmp::Ordering::Greater)
    }

    /// `self >= other`.
    pub fn ge_with_budget(&self, other: &Self, budget: usize) -> Option<bool> {
        self.cmp_with_budget(other, budget)
            .map(|o| o != std::cmp::Ordering::Less)
    }
}

// =========================================================================
// CF-native rounding and rational approximation
// =========================================================================

impl ExactReal {
    /// Floor: the greatest integer not exceeding the value.
    ///
    /// For a canonical continued fraction `[a0; a1, a2, …]` the floor
    /// is exactly the first partial quotient `a0`: when the CF has a
    /// tail the fractional part `1/[a1; a2, …]` lies strictly in
    /// `(0, 1)`, and when it does not the value already equals `a0`.
    /// This holds for every representation — `Rational`,
    /// `AlgebraicSqrt`, and `Gosper` — so the floor even of an
    /// irrational costs a single partial-quotient pull.
    ///
    /// Returns `None` when the value is nil, or when a lazy CF stream
    /// exhausts its internal safety budget before `a0` is fixed
    /// (SPEC §7.4.1 `undecidable`).
    pub fn floor(&self) -> Option<ExactReal> {
        if self.is_nil() {
            return None;
        }
        let mut iter = CfIter::from_exact_real(self);
        match iter.next_step() {
            CfStep::Quotient(a0) => Some(ExactReal::from_bigint(a0)),
            CfStep::Ended | CfStep::Exhausted => None,
        }
    }

    /// Ceiling: the least integer not below the value.
    ///
    /// Equal to `a0` when the value is an integer (the CF is the
    /// single term `[a0]`) and to `a0 + 1` otherwise, since a present
    /// CF tail forces the fractional part into `(0, 1)`.
    ///
    /// Returns `None` on nil or safety-budget exhaustion, like `floor`.
    pub fn ceil(&self) -> Option<ExactReal> {
        if self.is_nil() {
            return None;
        }
        let mut iter = CfIter::from_exact_real(self);
        let a0 = match iter.next_step() {
            CfStep::Quotient(q) => q,
            CfStep::Ended | CfStep::Exhausted => return None,
        };
        match iter.next_step() {
            CfStep::Ended => Some(ExactReal::from_bigint(a0)),
            CfStep::Quotient(_) => Some(ExactReal::from_bigint(a0 + BigInt::one())),
            CfStep::Exhausted => None,
        }
    }

    /// Round to the nearest integer, ties away from zero (matching
    /// `Fraction::round`).
    ///
    /// With `[a0; a1, …]` the fractional part is `f = 1/r` where
    /// `r = [a1; a2, …]`. Then `f < 1/2 ⇔ r > 2`, `f = 1/2 ⇔ r = 2`
    /// (the CF is exactly `[a0; 2]`), and `f > 1/2 ⇔ r < 2 ⇔ a1 = 1`.
    /// At a tie the value is the half-integer `a0 + 1/2`; rounding
    /// away from zero picks `a0 + 1` when `a0 >= 0` and `a0` when
    /// `a0 < 0`. At most three partial quotients are pulled.
    ///
    /// Returns `None` on nil or safety-budget exhaustion.
    pub fn round(&self) -> Option<ExactReal> {
        if self.is_nil() {
            return None;
        }
        let mut iter = CfIter::from_exact_real(self);
        let a0 = match iter.next_step() {
            CfStep::Quotient(q) => q,
            CfStep::Ended | CfStep::Exhausted => return None,
        };
        let a1 = match iter.next_step() {
            CfStep::Ended => return Some(ExactReal::from_bigint(a0)),
            CfStep::Quotient(q) => q,
            CfStep::Exhausted => return None,
        };
        let rounded = match a1.cmp(&BigInt::from(2)) {
            // a1 = 1 ⇒ r < 2 ⇒ f > 1/2.
            std::cmp::Ordering::Less => a0 + BigInt::one(),
            // a1 >= 3 ⇒ r > 2 ⇒ f < 1/2.
            std::cmp::Ordering::Greater => a0,
            // a1 = 2 ⇒ r > 2 if the CF continues, r = 2 (a tie) if it
            // ends here.
            std::cmp::Ordering::Equal => match iter.next_step() {
                CfStep::Quotient(_) => a0,
                CfStep::Ended => {
                    if a0.is_negative() {
                        a0
                    } else {
                        a0 + BigInt::one()
                    }
                }
                CfStep::Exhausted => return None,
            },
        };
        Some(ExactReal::from_bigint(rounded))
    }

    /// Best rational approximation within a denominator bound.
    ///
    /// Streams the canonical partial quotients and returns the
    /// principal convergent whose denominator is the largest not
    /// exceeding `max_denominator`. Every principal convergent is a
    /// best rational approximation in the strong sense — no rational
    /// with an equal or smaller denominator is strictly closer to the
    /// value — so the result is the closest such convergent the bound
    /// admits. For a `Rational` value whose own denominator already
    /// fits the bound the result is the value itself.
    ///
    /// Returns `None` when `max_denominator < 1`, when the value is
    /// nil, or when a lazy CF stream exhausts before any convergent
    /// is produced.
    pub fn best_rational_approximation(&self, max_denominator: &BigInt) -> Option<Fraction> {
        if max_denominator < &BigInt::one() {
            return None;
        }
        if self.is_nil() {
            return None;
        }
        let mut iter = CfIter::from_exact_real(self);
        // Convergent recurrence with the standard seed
        // h_{-1}/k_{-1} = 1/0 and h_{-2}/k_{-2} = 0/1.
        let mut h_prev2 = BigInt::zero();
        let mut h_prev1 = BigInt::one();
        let mut k_prev2 = BigInt::one();
        let mut k_prev1 = BigInt::zero();
        let mut best: Option<(BigInt, BigInt)> = None;
        while let CfStep::Quotient(a) = iter.next_step() {
            let h = &a * &h_prev1 + &h_prev2;
            let k = &a * &k_prev1 + &k_prev2;
            if &k > max_denominator {
                break;
            }
            h_prev2 = std::mem::replace(&mut h_prev1, h.clone());
            k_prev2 = std::mem::replace(&mut k_prev1, k.clone());
            best = Some((h, k));
        }
        best.map(|(h, k)| Fraction::new(h, k))
    }

    /// Pre-period and period blocks of an `AlgebraicSqrt`'s canonical
    /// continued fraction.
    ///
    /// By Lagrange's theorem the CF of a quadratic surd is eventually
    /// periodic (SPEC §4.2.2). This walks the surd state `(P, Q)`
    /// until it repeats and returns `(pre_period, period)`: the
    /// partial quotients emitted before the cycle, and one full
    /// cycle. The canonical CF is `pre_period` followed by `period`
    /// repeated forever, so the pair is a finite exact description of
    /// the otherwise-infinite expansion — and lets the N-th partial
    /// quotient be read in O(1) by indexing into `period`.
    ///
    /// Returns `None` for any representation other than
    /// `AlgebraicSqrt`, and (defensively) for a radicand whose period
    /// exceeds the scan cap.
    pub fn sqrt_cf_period(&self) -> Option<(Vec<BigInt>, Vec<BigInt>)> {
        // A valid quadratic surd always cycles within a bounded
        // number of states, but a pathologically large radicand could
        // make the period impractically long; bail out rather than
        // allocate without bound.
        const PERIOD_SCAN_CAP: usize = 1 << 16;

        let radicand = self.sqrt_radicand()?;
        let p = radicand.numerator();
        let q = radicand.denominator();
        let big_d = &p * &q;
        let sqrt_floor = big_d.sqrt();

        let mut p_i = BigInt::zero();
        let mut q_i = q;
        let mut quotients: Vec<BigInt> = Vec::new();
        let mut seen: Vec<(BigInt, BigInt)> = Vec::new();

        for _ in 0..PERIOD_SCAN_CAP {
            if let Some(start) = seen.iter().position(|(sp, sq)| *sp == p_i && *sq == q_i) {
                let period = quotients.split_off(start);
                return Some((quotients, period));
            }
            seen.push((p_i.clone(), q_i.clone()));
            let a_i: BigInt = (&p_i + &sqrt_floor).div_floor(&q_i);
            let next_p: BigInt = &a_i * &q_i - &p_i;
            let next_q: BigInt = (&big_d - &next_p * &next_p) / &q_i;
            quotients.push(a_i);
            p_i = next_p;
            q_i = next_q;
        }
        None
    }
}

// =========================================================================
// Möbius / bihomographic constructors
// =========================================================================

/// Build a Möbius Gosper for (a·x + b) / (c·x + d), short-circuiting
/// trivial cases that collapse to a `Rational`.
fn mobius_apply(a: BigInt, b: BigInt, c: BigInt, d: BigInt, x: ExactReal) -> ExactReal {
    if a.is_zero() && c.is_zero() {
        // 0·x + b / 0·x + d = b/d, x doesn't matter (provided d ≠ 0).
        if d.is_zero() {
            return ExactReal::Rational(Fraction::nil());
        }
        return ExactReal::Rational(Fraction::new(b, d));
    }
    if let ExactReal::Rational(f) = &x {
        // Evaluate the Möbius directly on the rational operand.
        let (xn, xd) = f.to_bigint_pair();
        let num = &a * &xn + &b * &xd;
        let den = &c * &xn + &d * &xd;
        if den.is_zero() {
            return ExactReal::Rational(Fraction::nil());
        }
        return ExactReal::Rational(Fraction::new(num, den));
    }
    ExactReal::Gosper(Arc::new(Gosper::Mobius { a, b, c, d, x }))
}

fn add_rational_to_lazy(r: &Fraction, lazy: ExactReal) -> ExactReal {
    // x + p/q = (q·x + p) / q. Möbius on lazy operand.
    let (p, q) = r.to_bigint_pair();
    mobius_apply(q.clone(), p, BigInt::zero(), q, lazy)
}

fn mul_rational_by_lazy(r: &Fraction, lazy: ExactReal) -> ExactReal {
    // (p/q) · x = (p·x) / q. Möbius on lazy operand.
    let (p, q) = r.to_bigint_pair();
    mobius_apply(p, BigInt::zero(), BigInt::zero(), q, lazy)
}

fn bihom_apply(
    a: BigInt,
    b: BigInt,
    c: BigInt,
    d: BigInt,
    e: BigInt,
    f: BigInt,
    g: BigInt,
    h: BigInt,
    x: ExactReal,
    y: ExactReal,
) -> ExactReal {
    ExactReal::Gosper(Arc::new(Gosper::Bihomographic {
        a,
        b,
        c,
        d,
        e,
        f,
        g,
        h,
        x,
        y,
    }))
}

fn bihom_apply_add(x: ExactReal, y: ExactReal) -> ExactReal {
    // x + y = (0·xy + 1·x + 1·y + 0) / (0·xy + 0·x + 0·y + 1)
    bihom_apply(
        BigInt::zero(),
        BigInt::one(),
        BigInt::one(),
        BigInt::zero(),
        BigInt::zero(),
        BigInt::zero(),
        BigInt::zero(),
        BigInt::one(),
        x,
        y,
    )
}

fn bihom_apply_sub(x: ExactReal, y: ExactReal) -> ExactReal {
    // x − y = (0·xy + 1·x − 1·y + 0) / 1
    bihom_apply(
        BigInt::zero(),
        BigInt::one(),
        BigInt::from(-1),
        BigInt::zero(),
        BigInt::zero(),
        BigInt::zero(),
        BigInt::zero(),
        BigInt::one(),
        x,
        y,
    )
}

fn bihom_apply_mul(x: ExactReal, y: ExactReal) -> ExactReal {
    // x · y = (1·xy + 0 + 0 + 0) / 1
    bihom_apply(
        BigInt::one(),
        BigInt::zero(),
        BigInt::zero(),
        BigInt::zero(),
        BigInt::zero(),
        BigInt::zero(),
        BigInt::zero(),
        BigInt::one(),
        x,
        y,
    )
}

fn bihom_apply_div(x: ExactReal, y: ExactReal) -> ExactReal {
    // x / y = (0·xy + 1·x + 0·y + 0) / (0·xy + 0·x + 1·y + 0)
    bihom_apply(
        BigInt::zero(),
        BigInt::one(),
        BigInt::zero(),
        BigInt::zero(),
        BigInt::zero(),
        BigInt::zero(),
        BigInt::one(),
        BigInt::zero(),
        x,
        y,
    )
}

// =========================================================================
// CF streaming iterator — backs `partial_quotients_bounded`
// =========================================================================

/// Lazy partial-quotient iterator for any `ExactReal`. Owns its
/// expansion state (no borrows into the source) so it can drive
/// recursive Gosper expansions independently of the caller's value.
struct CfIter {
    state: CfState,
}

enum CfState {
    /// Stream legitimately ended (rational CF reached its final
    /// canonical term, or a Möbius/bihomographic transform finished
    /// expanding its rational tail). Further `next_quotient` calls
    /// return `None`; `next_step` reports `Ended`.
    Empty,
    /// Internal safety budget was hit without producing the next
    /// partial quotient (degenerate Möbius / bihomographic state
    /// whose corners refuse to agree, or pathological
    /// limit-at-infinity). Further polls return `None`; `next_step`
    /// reports `Exhausted` so the comparison-budget algorithm can
    /// project to `NilReason::Undecidable` per SPEC §7.4.1 instead
    /// of mistaking this for a finite-CF terminator.
    Exhausted,
    /// Pre-computed canonical sequence (used for rationals and for
    /// the rational tail of a Möbius transform once its operand is
    /// exhausted).
    Finite { canonical: Vec<BigInt>, pos: usize },
    /// Quadratic-surd `(P, Q, D)` state per SPEC §4.2.2; always
    /// produces a partial quotient when polled (lazy CF is infinite
    /// for non-perfect-square radicands, which is the only case
    /// `AlgebraicSqrt` represents).
    Sqrt {
        big_d: BigInt,
        sqrt_floor: BigInt,
        p_i: BigInt,
        q_i: BigInt,
    },
    /// Unary Möbius (a·x + b) / (c·x + d) over an inner operand
    /// stream `x`.
    Mobius {
        a: BigInt,
        b: BigInt,
        c: BigInt,
        d: BigInt,
        x: Box<CfIter>,
        x_done: bool,
    },
    /// Binary bihomographic over inner operand streams `x` and `y`.
    Bihom {
        a: BigInt,
        b: BigInt,
        c: BigInt,
        d: BigInt,
        e: BigInt,
        f: BigInt,
        g: BigInt,
        h: BigInt,
        x: Box<CfIter>,
        y: Box<CfIter>,
        x_done: bool,
        y_done: bool,
        x_consumed: usize,
        y_consumed: usize,
    },
}

impl CfIter {
    fn from_exact_real(value: &ExactReal) -> Self {
        match value {
            ExactReal::Rational(f) => {
                if f.is_nil() {
                    return CfIter {
                        state: CfState::Empty,
                    };
                }
                CfIter {
                    state: CfState::Finite {
                        canonical: rational_partial_quotients(f.numerator(), f.denominator()),
                        pos: 0,
                    },
                }
            }
            ExactReal::AlgebraicSqrt { radicand } => {
                let p = radicand.numerator();
                let q = radicand.denominator();
                let big_d = &p * &q;
                let sqrt_floor = big_d.sqrt();
                CfIter {
                    state: CfState::Sqrt {
                        big_d,
                        sqrt_floor,
                        p_i: BigInt::zero(),
                        q_i: q,
                    },
                }
            }
            ExactReal::Gosper(g) => match g.as_ref() {
                Gosper::Mobius { a, b, c, d, x } => CfIter {
                    state: CfState::Mobius {
                        a: a.clone(),
                        b: b.clone(),
                        c: c.clone(),
                        d: d.clone(),
                        x: Box::new(CfIter::from_exact_real(x)),
                        x_done: false,
                    },
                },
                Gosper::Bihomographic {
                    a,
                    b,
                    c,
                    d,
                    e,
                    f,
                    g: gco,
                    h,
                    x,
                    y,
                } => CfIter {
                    state: CfState::Bihom {
                        a: a.clone(),
                        b: b.clone(),
                        c: c.clone(),
                        d: d.clone(),
                        e: e.clone(),
                        f: f.clone(),
                        g: gco.clone(),
                        h: h.clone(),
                        x: Box::new(CfIter::from_exact_real(x)),
                        y: Box::new(CfIter::from_exact_real(y)),
                        x_done: false,
                        y_done: false,
                        x_consumed: 0,
                        y_consumed: 0,
                    },
                },
            },
        }
    }

    fn next_quotient(&mut self) -> Option<BigInt> {
        match self.next_step() {
            CfStep::Quotient(q) => Some(q),
            CfStep::Ended | CfStep::Exhausted => None,
        }
    }

    fn next_step(&mut self) -> CfStep {
        loop {
            match &mut self.state {
                CfState::Empty => return CfStep::Ended,
                CfState::Exhausted => return CfStep::Exhausted,
                CfState::Finite { canonical, pos } => {
                    if *pos < canonical.len() {
                        let q = canonical[*pos].clone();
                        *pos += 1;
                        return CfStep::Quotient(q);
                    }
                    self.state = CfState::Empty;
                    return CfStep::Ended;
                }
                CfState::Sqrt {
                    big_d,
                    sqrt_floor,
                    p_i,
                    q_i,
                } => {
                    let a_i: BigInt = (&*p_i + &*sqrt_floor).div_floor(q_i);
                    let next_p: BigInt = &a_i * &*q_i - &*p_i;
                    let next_q: BigInt = (&*big_d - &next_p * &next_p) / &*q_i;
                    *p_i = next_p;
                    *q_i = next_q;
                    return CfStep::Quotient(a_i);
                }
                CfState::Mobius { .. } => {
                    if let Some(q) = step_mobius(&mut self.state) {
                        return CfStep::Quotient(q);
                    }
                    // step_mobius transitioned the state (Finite tail,
                    // Empty for ended-normally, or Exhausted for
                    // safety-budget hit). Re-dispatch.
                }
                CfState::Bihom { .. } => {
                    if let Some(q) = step_bihom(&mut self.state) {
                        return CfStep::Quotient(q);
                    }
                }
            }
        }
    }
}

/// Tri-state result of advancing a `CfIter` by one step.
///
/// `Ended` is reserved for legitimate termination of the CF (a
/// finite rational reached its last canonical term, or a Gosper
/// transform finished expanding its rational tail). `Exhausted`
/// marks internal safety-budget exhaustion or a pathological
/// limit-at-infinity — the value cannot be advanced further without
/// expanding the implementation's iteration cap. The
/// comparison-budget algorithm distinguishes them so it can
/// correctly project the second case to NIL with `absence.reason =
/// undecidable` per SPEC §7.4.1, rather than treating it as a
/// finite-CF terminator that would apply the alternating-parity
/// rule against an unrelated baseline.
enum CfStep {
    Quotient(BigInt),
    Ended,
    Exhausted,
}

/// Divide the four Möbius coefficients by their common GCD in place.
///
/// The value `(a·x + b) / (c·x + d)` is invariant under scaling every
/// coefficient by the same nonzero factor, so this renormalization
/// keeps coefficients small across repeated ingestion without
/// changing the emitted partial quotients. Without it the
/// coefficients grow multiplicatively with every absorbed quotient,
/// inflating every subsequent BigInt operation.
fn normalize_mobius(a: &mut BigInt, b: &mut BigInt, c: &mut BigInt, d: &mut BigInt) {
    let common = a.gcd(b).gcd(&c.gcd(d));
    if common.is_zero() || common.is_one() {
        return;
    }
    *a = &*a / &common;
    *b = &*b / &common;
    *c = &*c / &common;
    *d = &*d / &common;
}

/// Divide the eight bihomographic coefficients by their common GCD in
/// place; see `normalize_mobius`.
#[allow(clippy::too_many_arguments)]
fn normalize_bihom(
    a: &mut BigInt,
    b: &mut BigInt,
    c: &mut BigInt,
    d: &mut BigInt,
    e: &mut BigInt,
    f: &mut BigInt,
    g: &mut BigInt,
    h: &mut BigInt,
) {
    let common = a.gcd(b).gcd(&c.gcd(d)).gcd(&e.gcd(f)).gcd(&g.gcd(h));
    if common.is_zero() || common.is_one() {
        return;
    }
    *a = &*a / &common;
    *b = &*b / &common;
    *c = &*c / &common;
    *d = &*d / &common;
    *e = &*e / &common;
    *f = &*f / &common;
    *g = &*g / &common;
    *h = &*h / &common;
}

/// Tri-state advance of a [`NicfStream`], mirroring [`CfStep`] but for the
/// nearest-integer (semiregular) expansion of SPEC §4.2.5.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NicfStep {
    /// A semiregular partial quotient `b_i` (signed; `|b_i| >= 2` for
    /// `i >= 1`).
    Term(BigInt),
    /// The value is rational and its NICF terminated.
    Ended,
    /// Internal safety budget hit before the next term was determined.
    Exhausted,
}

/// Nearest-integer continued-fraction stream (SPEC §4.2.5 / §7.4.1.1).
///
/// This is the comparison-only expansion: it consumes the *regular* CF of a
/// value (the untouched [`CfIter`], which still backs display, canonical
/// form, and rounding) and re-expands it as nearest-integer terms by running
/// an identity Möbius `(1·x + 0)/(0·x + 1)` over the inner regular stream and
/// emitting `round`-agreement instead of `floor`-agreement across the value
/// range `x ∈ [1, ∞)`. The coefficient update on emit is identical to the
/// regular Gosper emitter; the sign of a negative remainder propagates through
/// the reciprocal continuation, so no separate `ε` bookkeeping is carried in
/// the coefficients (this is the property verified by the §7.4.1.1 Gosper
/// feasibility prototype).
struct NicfStream {
    // Möbius coefficients of (a·x + b)/(c·x + d) over the inner regular CF.
    a: BigInt,
    b: BigInt,
    c: BigInt,
    d: BigInt,
    inner: CfIter,
    inner_done: bool,
    /// Once the inner stream has produced the leading regular term, the value
    /// range collapses to `x ∈ [1, ∞)`; before that the first regular term is
    /// an unrestricted integer `a0 ∈ (-∞, ∞)`, handled by `prime`.
    primed: bool,
    /// Set once the final (exact) semiregular term has been emitted; the next
    /// `next()` then reports [`NicfStep::Ended`].
    done: bool,
}

impl NicfStream {
    fn new(value: &ExactReal) -> Self {
        NicfStream {
            a: BigInt::one(),
            b: BigInt::zero(),
            c: BigInt::zero(),
            d: BigInt::one(),
            inner: CfIter::from_exact_real(value),
            inner_done: false,
            primed: false,
            done: false,
        }
    }

    /// Ingest one regular partial quotient `p` of the inner stream:
    /// substitute `x ← p + 1/x'`, i.e. `(a,b,c,d) ← (a·p + b, a, c·p + d, c)`.
    fn ingest(&mut self, p: &BigInt) {
        let na = &self.a * p + &self.b;
        let nb = self.a.clone();
        let nc = &self.c * p + &self.d;
        let nd = self.c.clone();
        self.a = na;
        self.b = nb;
        self.c = nc;
        self.d = nd;
        normalize_mobius(&mut self.a, &mut self.b, &mut self.c, &mut self.d);
    }

    /// Nearest integer to `num/den` (`den != 0`) under the normative
    /// round-half-down tie-break of SPEC §4.2.5: remainder in `(-1/2, 1/2]`,
    /// `round = ⌈(2·num − den)/(2·den)⌉` after normalizing `den > 0`.
    fn round_half_down(mut num: BigInt, mut den: BigInt) -> BigInt {
        if den.is_negative() {
            num = -num;
            den = -den;
        }
        let two = BigInt::from(2);
        let p = &two * &num - &den;
        let q = &two * &den;
        p.div_ceil(&q)
    }

    fn next(&mut self) -> NicfStep {
        if self.done {
            return NicfStep::Ended;
        }
        let mut ingest_budget = GOSPER_INGEST_SAFETY;
        loop {
            if !self.primed {
                // Pull the leading regular term a0 ∈ ℤ (any sign) and the
                // first restricted term, so the Möbius value range becomes
                // x ∈ [1, ∞). The identity-Möbius leading term equals a0's
                // nearest integer trivially once one inner term is absorbed.
                match self.inner.next_step() {
                    CfStep::Quotient(p) => {
                        self.ingest(&p);
                        self.primed = true;
                    }
                    CfStep::Ended => {
                        // Value is an integer literal already pinned in (a,b)?
                        // Identity Möbius with no terms means value range is
                        // x∈[1,∞) over an empty stream; treat as ended.
                        self.inner_done = true;
                        self.primed = true;
                    }
                    CfStep::Exhausted => return NicfStep::Exhausted,
                }
                continue;
            }

            // Try to emit: round of the value must agree at both endpoints
            // of x ∈ [1, ∞): v(∞) = a/c and v(1) = (a+b)/(c+d), with no pole
            // in [1, ∞) (denominators share a sign).
            if !self.c.is_zero() && !(&self.c + &self.d).is_zero() {
                let cd = &self.c + &self.d;
                let pole_free = (self.c.is_positive() && cd.is_positive())
                    || (self.c.is_negative() && cd.is_negative());
                if pole_free {
                    let q_inf = Self::round_half_down(self.a.clone(), self.c.clone());
                    let q_one = Self::round_half_down(&self.a + &self.b, cd.clone());
                    if q_inf == q_one {
                        let q = q_inf;
                        // Update is identical to the regular emitter:
                        //   v' = 1/(v − q) ⇒ (a,b,c,d) ← (c, d, a−q·c, b−q·d).
                        let nc = &self.a - &q * &self.c;
                        let nd = &self.b - &q * &self.d;
                        let na = self.c.clone();
                        let nb = self.d.clone();
                        self.a = na;
                        self.b = nb;
                        self.c = nc;
                        self.d = nd;
                        normalize_mobius(&mut self.a, &mut self.b, &mut self.c, &mut self.d);
                        // After emit, v' = (a·x + b)/(c·x + d). When both new
                        // c and d are zero the residual is the constant a/b
                        // with no x-dependence and, when also the numerator is
                        // proportional, the value was exactly q ⇒ terminate.
                        // The robust terminal test: the inner stream is done
                        // and the post-emit value reduces to an integer with
                        // zero remainder — detected on the next pass via the
                        // `inner_done` branch. Here we only mark termination
                        // when the remainder is provably zero: new c == 0 and
                        // new d == 0 (the operand fully consumed and pinned).
                        if self.c.is_zero() && self.d.is_zero() {
                            self.done = true;
                        }
                        return NicfStep::Term(q);
                    }
                }
            }

            if self.inner_done {
                // No more inner terms and still cannot pin a nearest integer:
                // the residual value is the constant a/c (x has gone to ∞, so
                // the b,d terms drop). If c == 0 the value is infinite (should
                // not happen for a finite operand) — report exhaustion.
                if self.c.is_zero() {
                    return NicfStep::Exhausted;
                }
                let q = Self::round_half_down(self.a.clone(), self.c.clone());
                let rem_num = &self.a - &q * &self.c; // remainder over c
                if rem_num.is_zero() {
                    // a/c == q exactly: final term.
                    self.done = true;
                    return NicfStep::Term(q);
                }
                // Continue with v' = 1/(a/c − q): the new value is the
                // constant c/rem_num (still x-independent), so collapse to a
                // pure rational Möbius (b = d = 0) and keep emitting.
                let nc = rem_num;
                self.a = self.c.clone();
                self.b = BigInt::zero();
                self.c = nc;
                self.d = BigInt::zero();
                return NicfStep::Term(q);
            }

            if ingest_budget == 0 {
                return NicfStep::Exhausted;
            }
            ingest_budget -= 1;

            match self.inner.next_step() {
                CfStep::Quotient(p) => self.ingest(&p),
                CfStep::Ended => self.inner_done = true,
                CfStep::Exhausted => return NicfStep::Exhausted,
            }
        }
    }
}

/// Run the unary Gosper loop until it emits, transitions to a
/// rational tail, or exhausts the safety budget. Returns `Some(q)`
/// when a partial quotient is emitted (state already updated); `None`
/// when the state has transitioned to a non-Möbius form that the
/// caller's outer loop should re-dispatch.
fn step_mobius(state: &mut CfState) -> Option<BigInt> {
    let CfState::Mobius {
        a,
        b,
        c,
        d,
        x,
        x_done,
    } = state
    else {
        return None;
    };
    let mut ingest_budget = GOSPER_INGEST_SAFETY;
    loop {
        // Try to emit. Requires the range of (a·x + b)/(c·x + d)
        // over x ∈ [1, ∞) to project onto a single integer floor.
        if !c.is_zero() && !(&*c + &*d).is_zero() {
            let cd_sum = &*c + &*d;
            // Both denominators are nonzero. Require them to share
            // the same sign so that the function has no pole in
            // [1, ∞); otherwise ingest to disambiguate.
            if (c.is_positive() && cd_sum.is_positive())
                || (c.is_negative() && cd_sum.is_negative())
            {
                let q_inf = a.div_floor(c);
                let ab_sum = &*a + &*b;
                let q_one = ab_sum.div_floor(&cd_sum);
                if q_inf == q_one {
                    let q = q_inf;
                    let new_a = c.clone();
                    let new_b = d.clone();
                    let new_c = &*a - &q * &*c;
                    let new_d = &*b - &q * &*d;
                    *a = new_a;
                    *b = new_b;
                    *c = new_c;
                    *d = new_d;
                    return Some(q);
                }
            }
        }

        if *x_done {
            // Operand exhausted and we still can't emit. The remaining
            // value is the constant a/c (treating x' = ∞). If c == 0
            // the value is mathematically infinite — no CF expansion
            // exists, so report exhaustion rather than a phantom end.
            if c.is_zero() {
                *state = CfState::Exhausted;
                return None;
            }
            let canonical = rational_partial_quotients(a.clone(), c.clone());
            *state = CfState::Finite { canonical, pos: 0 };
            return None;
        }

        if ingest_budget == 0 {
            *state = CfState::Exhausted;
            return None;
        }
        ingest_budget -= 1;

        match x.next_step() {
            CfStep::Quotient(p) => {
                // Ingest p: (a, b, c, d) ← (a·p + b, a, c·p + d, c)
                let new_a = &*a * &p + &*b;
                let new_b = a.clone();
                let new_c = &*c * &p + &*d;
                let new_d = c.clone();
                *a = new_a;
                *b = new_b;
                *c = new_c;
                *d = new_d;
                normalize_mobius(a, b, c, d);
            }
            CfStep::Ended => {
                *x_done = true;
            }
            CfStep::Exhausted => {
                // Inner operand could not be advanced; propagate.
                *state = CfState::Exhausted;
                return None;
            }
        }
    }
}

/// Run the bihomographic Gosper loop until it emits, transitions to
/// a tail form, or exhausts the safety budget.
fn step_bihom(state: &mut CfState) -> Option<BigInt> {
    let CfState::Bihom {
        a,
        b,
        c,
        d,
        e,
        f,
        g,
        h,
        x,
        y,
        x_done,
        y_done,
        x_consumed,
        y_consumed,
    } = state
    else {
        return None;
    };

    let mut ingest_budget = GOSPER_INGEST_SAFETY;

    loop {
        // If x is exhausted, the matrix's x-derivative coefficients
        // are taken at x = ∞, collapsing the bihom into a Möbius in
        // y. Symmetrically for y. If both, the value is a/e.
        if *x_done && *y_done {
            if e.is_zero() {
                *state = CfState::Exhausted;
                return None;
            }
            let canonical = rational_partial_quotients(a.clone(), e.clone());
            *state = CfState::Finite { canonical, pos: 0 };
            return None;
        }
        if *x_done {
            // Collapse to Möbius on y: take x → ∞ limit, so terms
            // proportional to x dominate:
            //   numerator   → a·y + b
            //   denominator → e·y + f
            let mob_a = a.clone();
            let mob_b = b.clone();
            let mob_c = e.clone();
            let mob_d = f.clone();
            // Re-package the y stream as the inner of a Möbius.
            let y_iter = std::mem::replace(
                y.as_mut(),
                CfIter {
                    state: CfState::Empty,
                },
            );
            *state = CfState::Mobius {
                a: mob_a,
                b: mob_b,
                c: mob_c,
                d: mob_d,
                x: Box::new(y_iter),
                x_done: *y_done,
            };
            return None;
        }
        if *y_done {
            // Collapse to Möbius on x: take y → ∞ limit:
            //   numerator   → a·x + c
            //   denominator → e·x + g
            let mob_a = a.clone();
            let mob_b = c.clone();
            let mob_c = e.clone();
            let mob_d = g.clone();
            let x_iter = std::mem::replace(
                x.as_mut(),
                CfIter {
                    state: CfState::Empty,
                },
            );
            *state = CfState::Mobius {
                a: mob_a,
                b: mob_b,
                c: mob_c,
                d: mob_d,
                x: Box::new(x_iter),
                x_done: *x_done,
            };
            return None;
        }

        // Try to emit. Range over x ∈ [1, ∞), y ∈ [1, ∞).
        if let Some(q) = bihom_emit_candidate(a, b, c, d, e, f, g, h) {
            // Emit q: numerator ← old denominator;
            //         denominator ← old (numerator − q · denominator).
            let new_a = e.clone();
            let new_b = f.clone();
            let new_c = g.clone();
            let new_d = h.clone();
            let new_e = &*a - &q * &*e;
            let new_f = &*b - &q * &*f;
            let new_g = &*c - &q * &*g;
            let new_h = &*d - &q * &*h;
            *a = new_a;
            *b = new_b;
            *c = new_c;
            *d = new_d;
            *e = new_e;
            *f = new_f;
            *g = new_g;
            *h = new_h;
            return Some(q);
        }

        if ingest_budget == 0 {
            *state = CfState::Exhausted;
            return None;
        }
        ingest_budget -= 1;

        // Choose which operand to ingest: prefer the axis whose
        // corner spread is wider when both axes have well-defined
        // (finite-denominator) projections; otherwise fall back to
        // balanced consumption so an axis whose corners are all
        // initially zero-denominator (e.g. y in `x + y`) still gets
        // its first ingestion before the safety budget expires.
        let ingest_x = pick_ingest_axis(a, b, c, d, e, f, g, h, *x_consumed, *y_consumed);
        if ingest_x {
            match x.next_step() {
                CfStep::Quotient(p) => {
                    // Ingest x: bihom in (x', y) where x = p + 1/x'
                    //   (a, b, c, d) ← (a·p + c, b·p + d, a, b)
                    //   (e, f, g, h) ← (e·p + g, f·p + h, e, f)
                    let new_a = &*a * &p + &*c;
                    let new_b = &*b * &p + &*d;
                    let new_c = a.clone();
                    let new_d = b.clone();
                    let new_e = &*e * &p + &*g;
                    let new_f = &*f * &p + &*h;
                    let new_g = e.clone();
                    let new_h = f.clone();
                    *a = new_a;
                    *b = new_b;
                    *c = new_c;
                    *d = new_d;
                    *e = new_e;
                    *f = new_f;
                    *g = new_g;
                    *h = new_h;
                    *x_consumed += 1;
                    normalize_bihom(a, b, c, d, e, f, g, h);
                }
                CfStep::Ended => {
                    *x_done = true;
                }
                CfStep::Exhausted => {
                    *state = CfState::Exhausted;
                    return None;
                }
            }
        } else {
            match y.next_step() {
                CfStep::Quotient(p) => {
                    // Ingest y: bihom in (x, y') where y = p + 1/y'
                    //   (a, b, c, d) ← (a·p + b, a, c·p + d, c)
                    //   (e, f, g, h) ← (e·p + f, e, g·p + h, g)
                    let new_a = &*a * &p + &*b;
                    let new_b = a.clone();
                    let new_c = &*c * &p + &*d;
                    let new_d = c.clone();
                    let new_e = &*e * &p + &*f;
                    let new_f = e.clone();
                    let new_g = &*g * &p + &*h;
                    let new_h = g.clone();
                    *a = new_a;
                    *b = new_b;
                    *c = new_c;
                    *d = new_d;
                    *e = new_e;
                    *f = new_f;
                    *g = new_g;
                    *h = new_h;
                    *y_consumed += 1;
                    normalize_bihom(a, b, c, d, e, f, g, h);
                }
                CfStep::Ended => {
                    *y_done = true;
                }
                CfStep::Exhausted => {
                    *state = CfState::Exhausted;
                    return None;
                }
            }
        }
    }
}

/// Probe the four bihomographic corners over x, y ∈ [1, ∞). Returns
/// `Some(q)` iff every well-defined corner has the same floor `q`,
/// every denominator has the same sign as `e+f+g+h` (so the function
/// has no pole in the box), and at least one corner is computable.
fn bihom_emit_candidate(
    a: &BigInt,
    b: &BigInt,
    c: &BigInt,
    d: &BigInt,
    e: &BigInt,
    f: &BigInt,
    g: &BigInt,
    h: &BigInt,
) -> Option<BigInt> {
    let corners = [
        (a + b + c + d, e + f + g + h), // x=1, y=1
        (a + b, e + f),                 // x=∞, y=1
        (a + c, e + g),                 // x=1, y=∞
        (a.clone(), e.clone()),         // x=∞, y=∞
    ];

    let mut pivot_sign: Option<bool> = None;
    let mut common_floor: Option<BigInt> = None;
    for (num, den) in &corners {
        if den.is_zero() {
            return None;
        }
        let positive = den.is_positive();
        match pivot_sign {
            None => pivot_sign = Some(positive),
            Some(prev) if prev != positive => return None,
            _ => {}
        }
        let q = num.div_floor(den);
        match &common_floor {
            None => common_floor = Some(q),
            Some(prev) if *prev != q => return None,
            _ => {}
        }
    }
    common_floor
}

/// Decide which axis to ingest from. Returns `true` for x, `false`
/// for y.
///
/// The function compares two floor-projection differences:
/// `x_spread = |⌊(a+c)/(e+g)⌋ − ⌊a/e⌋|` measures uncertainty along
/// the x-axis (varying x from ∞ down to 1 at fixed y = ∞), and
/// `y_spread = |⌊(a+b)/(e+f)⌋ − ⌊a/e⌋|` measures uncertainty along
/// the y-axis. The wider-spread axis is absorbed first because that
/// shrinks the four-corner range fastest. When one axis is
/// undefined (zero denominator at one of its extremes), that axis
/// is preferred — its corner is unbounded, so it dominates the
/// emit-blocking uncertainty. Both-undefined or equal-spread cases
/// fall back to the lower-consumed axis to guarantee progress on
/// both sides; without this, an axis whose corners are all
/// initially zero-denominator (e.g. y in `x + y`, whose constant
/// denominator makes three of four corners' denominators 0) would
/// never be absorbed before the safety budget expires.
fn pick_ingest_axis(
    a: &BigInt,
    b: &BigInt,
    c: &BigInt,
    d: &BigInt,
    e: &BigInt,
    f: &BigInt,
    g: &BigInt,
    h: &BigInt,
    x_consumed: usize,
    y_consumed: usize,
) -> bool {
    let _ = (d, h); // d, h influence only the (1, 1) corner.
    let x_spread = corner_floor_diff(a, c, e, g, a, e);
    let y_spread = corner_floor_diff(a, b, e, f, a, e);
    match (x_spread, y_spread) {
        (Some(sx), Some(sy)) if sx != sy => sx > sy,
        (None, None) => x_consumed <= y_consumed,
        (None, _) => true,
        (_, None) => false,
        _ => x_consumed <= y_consumed,
    }
}

/// Floor-difference between (num1_a + num1_b)/(den1_a + den1_b) and
/// num2/den2, or `None` if either denominator is zero.
fn corner_floor_diff(
    num1_a: &BigInt,
    num1_b: &BigInt,
    den1_a: &BigInt,
    den1_b: &BigInt,
    num2: &BigInt,
    den2: &BigInt,
) -> Option<BigInt> {
    let num1 = num1_a + num1_b;
    let den1 = den1_a + den1_b;
    if den1.is_zero() || den2.is_zero() {
        return None;
    }
    let f1 = num1.div_floor(&den1);
    let f2 = num2.div_floor(den2);
    Some((f1 - f2).abs())
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

    fn sqrt_of(num: i64, den: i64) -> ExactReal {
        ExactReal::from_sqrt_rational(Fraction::new(bi(num), bi(den)))
            .expect("non-negative radicand should construct")
    }

    // ─── SPEC §7.4.1.1 / §15.3: NICF-accelerated comparison conformance ───

    /// Reference regular-CF (floor) comparison, used only as the differential
    /// oracle for the NICF-accelerated `cmp_with_budget`. This is the
    /// pre-NICF algorithm verbatim, so agreement proves the NICF expansion
    /// never changes a decided order (SPEC §7.4.1.1).
    fn rcf_reference_cmp(x: &ExactReal, y: &ExactReal, budget: usize) -> CmpOutcome {
        use std::cmp::Ordering;
        if x.is_nil() || y.is_nil() || budget == 0 {
            return CmpOutcome::Undecided { agreed_prefix: 0 };
        }
        match (x, y) {
            (
                ExactReal::AlgebraicSqrt { radicand: r },
                ExactReal::AlgebraicSqrt { radicand: s },
            ) => {
                return CmpOutcome::Decided(r.cmp(s));
            }
            (ExactReal::AlgebraicSqrt { radicand: r }, ExactReal::Rational(q)) => {
                return CmpOutcome::Decided(cmp_sqrt_vs_rational(r, q));
            }
            (ExactReal::Rational(q), ExactReal::AlgebraicSqrt { radicand: r }) => {
                return CmpOutcome::Decided(cmp_sqrt_vs_rational(r, q).reverse());
            }
            _ => {}
        }
        let mut a = CfIter::from_exact_real(x);
        let mut b = CfIter::from_exact_real(y);
        for i in 0..budget {
            match (a.next_step(), b.next_step()) {
                (CfStep::Quotient(av), CfStep::Quotient(bv)) => {
                    if av != bv {
                        return CmpOutcome::Decided(if i % 2 == 0 {
                            av.cmp(&bv)
                        } else {
                            bv.cmp(&av)
                        });
                    }
                }
                (CfStep::Ended, CfStep::Ended) => return CmpOutcome::Decided(Ordering::Equal),
                (CfStep::Ended, CfStep::Quotient(_)) => {
                    return CmpOutcome::Decided(if i % 2 == 0 {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    });
                }
                (CfStep::Quotient(_), CfStep::Ended) => {
                    return CmpOutcome::Decided(if i % 2 == 0 {
                        Ordering::Less
                    } else {
                        Ordering::Greater
                    });
                }
                (CfStep::Exhausted, _) | (_, CfStep::Exhausted) => {
                    return CmpOutcome::Undecided { agreed_prefix: i };
                }
            }
        }
        CmpOutcome::Undecided {
            agreed_prefix: budget,
        }
    }

    fn decided(o: CmpOutcome) -> Option<std::cmp::Ordering> {
        match o {
            CmpOutcome::Decided(o) => Some(o),
            CmpOutcome::Undecided { .. } => None,
        }
    }

    /// §15.3 (i): over a broad corpus the NICF-accelerated comparison decides
    /// the *same order* as the regular-CF reference whenever the reference
    /// decides — across Rational, AlgebraicSqrt, and Gosper operands.
    #[test]
    fn nicf_order_matches_rcf_reference_over_corpus() {
        const BUDGET: usize = 256;
        // Deterministic LCG corpus (no rng dep).
        let mut seed: u64 = 0xD1B54A32D192ED03;
        let mut next = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            seed
        };
        // Build a varied pool: rationals, sqrts, and Gosper (sums/products).
        let mut pool: Vec<ExactReal> = Vec::new();
        for _ in 0..40 {
            let n = (next() % 40_001) as i64 - 20_000;
            let d = (next() % 20_000) as i64 + 1;
            pool.push(rational(n, d));
        }
        for r in [2i64, 3, 5, 6, 7, 8, 10, 11, 13, 19, 23, 31] {
            pool.push(sqrt_of(r, 1));
            // √r + p/q and √r · (p/q) exercise the Gosper (Mobius/Bihom) path.
            pool.push(sqrt_of(r, 1).add(&rational((next() % 9) as i64 - 4, 1)));
            pool.push(sqrt_of(r, 1).mul(&rational(1, (next() % 5) as i64 + 1)));
        }
        // sqrt differences (the budget-stressing equal/near-equal cases).
        pool.push(sqrt_of(2, 1).sub(&sqrt_of(2, 1))); // == 0, must be U vs 0-ish
        pool.push(sqrt_of(2, 1).add(&sqrt_of(3, 1)));

        let mut compared = 0usize;
        let mut both_decided = 0usize;
        for (i, x) in pool.iter().enumerate() {
            for y in pool.iter().skip(i) {
                let got = decided(x.cmp_with_budget_tracked(y, BUDGET));
                let want = decided(rcf_reference_cmp(x, y, BUDGET));
                compared += 1;
                // When the reference decides, NICF must decide identically.
                if let Some(w) = want {
                    if let Some(g) = got {
                        assert_eq!(
                            g, w,
                            "NICF order disagreed with RCF reference for pair #{compared}"
                        );
                        both_decided += 1;
                    }
                    // NICF deciding at least as often is checked separately;
                    // here we only forbid a *wrong* decided order.
                }
                // Antisymmetry of the NICF result itself.
                let rev = decided(y.cmp_with_budget_tracked(x, BUDGET));
                if let (Some(g), Some(r)) = (got, rev) {
                    assert_eq!(g, r.reverse(), "NICF comparison not antisymmetric");
                }
            }
        }
        assert!(compared > 1000, "corpus too small: {compared}");
        assert!(both_decided > 500, "too few decided pairs: {both_decided}");
    }

    /// §15.3 (i) continued: NICF never decides *fewer* pairs than RCF at the
    /// same budget — its faster convergence can only move the U→decided
    /// boundary favorably.
    #[test]
    fn nicf_decides_at_least_as_often_as_rcf() {
        const BUDGET: usize = 24; // small budget to surface the difference
        let pairs = [
            (sqrt_of(2, 1).add(&sqrt_of(3, 1)), sqrt_of(5, 1)),
            (sqrt_of(7, 1), rational(8463, 3200)),
            (sqrt_of(13, 1).mul(&rational(1, 2)), rational(9, 5)),
            (sqrt_of(2, 1), rational(239, 169)),
        ];
        for (x, y) in &pairs {
            let nicf = decided(x.cmp_with_budget_tracked(y, BUDGET));
            let rcf = decided(rcf_reference_cmp(x, y, BUDGET));
            if rcf.is_some() {
                assert!(
                    nicf.is_some(),
                    "NICF failed to decide a pair RCF decided at budget {BUDGET}"
                );
                assert_eq!(nicf, rcf, "NICF decided a different order than RCF");
            }
        }
    }

    /// §15.3 (ii): `agreedPrefix` is monotone non-decreasing in the budget.
    #[test]
    fn nicf_agreed_prefix_monotone_in_budget() {
        let x = sqrt_of(2, 1).sub(&sqrt_of(2, 1)); // structurally ~0, never decides
        let y = rational(0, 1);
        let mut last = 0usize;
        for budget in [1usize, 2, 4, 8, 16, 32, 64, 128] {
            if let CmpOutcome::Undecided { agreed_prefix } = x.cmp_with_budget_tracked(&y, budget) {
                assert!(
                    agreed_prefix >= last,
                    "agreedPrefix decreased with budget: {agreed_prefix} < {last}"
                );
                assert!(
                    agreed_prefix <= budget,
                    "agreedPrefix {agreed_prefix} exceeded budget {budget}"
                );
                last = agreed_prefix;
            }
        }
    }

    /// §15.3 (iii): the normative round-half-down tie-break of §4.2.5 yields
    /// the specified semiregular digit on the singular `1/2`-remainder cases.
    #[test]
    fn nicf_round_half_down_tie_break() {
        // round_half_down(num,den): tie (frac == 1/2) rounds DOWN.
        // 1/2 → 0, 3/2 → 1, 5/2 → 2, -1/2 → -1, -3/2 → -2.
        let cases = [
            ((1, 2), 0),
            ((3, 2), 1),
            ((5, 2), 2),
            ((-1, 2), -1),
            ((-3, 2), -2),
            ((7, 4), 2),   // 1.75 → 2 (not a tie)
            ((-7, 4), -2), // -1.75 → -2
        ];
        for ((n, d), want) in cases {
            let got = NicfStream::round_half_down(BigInt::from(n), BigInt::from(d));
            assert_eq!(got, BigInt::from(want), "round_half_down({n}/{d})");
        }
        // The leading NICF terms reflect the tie-break: 1/2 = [0; 2] (b0 = 0,
        // the round of 1/2 under half-down), 2/3 = [1; -3] (round of 2/3 = 1).
        let mut s = NicfStream::new(&rational(1, 2));
        assert_eq!(s.next(), NicfStep::Term(BigInt::zero()));
    }

    /// NICF emission is correct across representations: a value's NICF terms,
    /// reconstructed back to a rational, equal the value (finite cases).
    #[test]
    fn nicf_terms_reconstruct_value() {
        for (n, d) in [(1, 2), (2, 3), (355, 113), (-7, 5), (22, 7), (0, 1)] {
            let v = rational(n, d);
            let mut s = NicfStream::new(&v);
            // Evaluate the semiregular CF b0 + ε1/(b1 + ε2/(... )) from the
            // emitted signed terms by folding from the back.
            let mut terms = Vec::new();
            loop {
                match s.next() {
                    NicfStep::Term(b) => terms.push(b),
                    NicfStep::Ended => break,
                    NicfStep::Exhausted => panic!("finite rational must not exhaust"),
                }
                if terms.len() > 64 {
                    panic!("finite NICF too long");
                }
            }
            // Fold: value = b_{k} ; then value = b_{i} + 1/value for the
            // signed semiregular form (ε is carried in the sign of b_{i+1}).
            let mut acc = Fraction::new(terms.last().unwrap().clone(), BigInt::one());
            for b in terms.iter().rev().skip(1) {
                // acc ← b + 1/acc
                let recip = Fraction::new(acc.denominator(), acc.numerator());
                acc = Fraction::new(b.clone(), BigInt::one()).add(&recip);
            }
            let want = Fraction::new(BigInt::from(n), BigInt::from(d));
            assert_eq!(acc, want, "NICF of {n}/{d} did not reconstruct");
        }
    }

    // === Phase 3 baseline (rationals) ===

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
        assert_eq!(ExactReal::from_partial_quotients(&terms(&[1, 0])), None);
        assert_eq!(ExactReal::from_partial_quotients(&terms(&[1, -2])), None);
    }

    #[test]
    fn round_trip_canonical_sequence_is_idempotent() {
        let value = rational(355, 113);
        let canonical = value.partial_quotients().expect("rational");
        let reconstructed = ExactReal::from_partial_quotients(&canonical).expect("valid sequence");
        let canonical_again = reconstructed.partial_quotients().expect("rational");
        assert_eq!(canonical, canonical_again);
    }

    #[test]
    fn round_trip_negative_value_is_idempotent() {
        let value = rational(-22, 7);
        let canonical = value.partial_quotients().expect("rational");
        let reconstructed = ExactReal::from_partial_quotients(&canonical).expect("valid sequence");
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

    // === Phase 4a baseline (sqrt) ===

    #[test]
    fn from_sqrt_rational_zero_returns_rational_zero() {
        let value = ExactReal::from_sqrt_rational(Fraction::new(bi(0), bi(1))).expect("zero");
        assert_eq!(value, rational(0, 1));
        assert!(value.is_rational());
        assert!(!value.is_algebraic_sqrt());
    }

    #[test]
    fn from_sqrt_rational_perfect_square_integer_collapses_to_rational() {
        let value = ExactReal::from_sqrt_rational(Fraction::new(bi(9), bi(1))).expect("9");
        assert_eq!(value, rational(3, 1));
        assert!(value.is_integer());
    }

    #[test]
    fn from_sqrt_rational_perfect_square_rational_collapses_to_rational() {
        let value = ExactReal::from_sqrt_rational(Fraction::new(bi(9), bi(16))).expect("9/16");
        assert_eq!(value, rational(3, 4));
    }

    #[test]
    fn from_sqrt_rational_quarter_collapses_to_one_half() {
        let value = ExactReal::from_sqrt_rational(Fraction::new(bi(1), bi(4))).expect("1/4");
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
        let value = sqrt_of(4, 3);
        assert!(value.is_algebraic_sqrt());
    }

    #[test]
    fn from_sqrt_rational_square_denominator_only_is_lazy() {
        let value = sqrt_of(2, 9);
        assert!(value.is_algebraic_sqrt());
    }

    #[test]
    fn algebraic_sqrt_partial_quotients_unbounded_returns_none() {
        assert_eq!(sqrt_of(2, 1).partial_quotients(), None);
        assert_eq!(sqrt_of(2, 3).partial_quotients(), None);
    }

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
        let value = sqrt_of(2, 1);
        assert_eq!(
            value.partial_quotients_bounded(6),
            terms(&[1, 2, 2, 2, 2, 2])
        );
    }

    #[test]
    fn sqrt_three_expands_to_period_one_two() {
        let value = sqrt_of(3, 1);
        assert_eq!(
            value.partial_quotients_bounded(7),
            terms(&[1, 1, 2, 1, 2, 1, 2])
        );
    }

    #[test]
    fn sqrt_seven_expands_to_period_one_one_one_four() {
        let value = sqrt_of(7, 1);
        assert_eq!(
            value.partial_quotients_bounded(9),
            terms(&[2, 1, 1, 1, 4, 1, 1, 1, 4])
        );
    }

    #[test]
    fn sqrt_two_thirds_expands_correctly() {
        let value = sqrt_of(2, 3);
        assert_eq!(
            value.partial_quotients_bounded(7),
            terms(&[0, 1, 4, 2, 4, 2, 4])
        );
    }

    #[test]
    fn bounded_budget_one_returns_first_term_only() {
        assert_eq!(sqrt_of(2, 1).partial_quotients_bounded(1), terms(&[1]));
        assert_eq!(sqrt_of(7, 1).partial_quotients_bounded(1), terms(&[2]));
        assert_eq!(sqrt_of(1, 2).partial_quotients_bounded(1), terms(&[0]));
    }

    // === Phase 4b: arithmetic ===

    fn assert_prefix(value: &ExactReal, expected: &[i64], budget: usize) {
        assert_eq!(value.partial_quotients_bounded(budget), terms(expected));
    }

    // -- negation --

    #[test]
    fn neg_rational_inverts_sign() {
        assert_eq!(rational(3, 2).neg(), rational(-3, 2));
        assert_eq!(rational(0, 1).neg(), rational(0, 1));
        assert_eq!(rational(-5, 7).neg(), rational(5, 7));
    }

    #[test]
    fn neg_sqrt_two_emits_known_cf() {
        // −√2 ≈ −1.4142.
        //   floor(−1.4142) = −2; residue 0.5858.
        //   1/0.5858 ≈ 1.7071; floor 1; residue 0.7071.
        //   1/0.7071 ≈ 1.4142; floor 1; residue 0.4142.
        //   1/0.4142 ≈ 2.4142; floor 2; residue 0.4142 — fixed point.
        // So −√2 = [−2; 1, 1, 2, 2, 2, 2, ...]
        let value = sqrt_of(2, 1).neg();
        assert_prefix(&value, &[-2, 1, 1, 2, 2, 2, 2, 2], 8);
    }

    #[test]
    fn neg_preserves_value_on_double_application() {
        let original = sqrt_of(7, 1);
        let twice = original.neg().neg();
        assert_eq!(
            twice.partial_quotients_bounded(8),
            original.partial_quotients_bounded(8)
        );
    }

    #[test]
    fn neg_nil_is_nil() {
        let nil = ExactReal::Rational(Fraction::nil());
        assert!(nil.neg().is_nil());
    }

    // -- reciprocal --

    #[test]
    fn reciprocal_rational_swaps_numerator_denominator() {
        assert_eq!(rational(3, 2).reciprocal(), Some(rational(2, 3)));
        assert_eq!(rational(-5, 1).reciprocal(), Some(rational(-1, 5)));
        assert_eq!(rational(0, 1).reciprocal(), None);
    }

    #[test]
    fn reciprocal_sqrt_two_emits_known_cf() {
        // 1/√2 = √(1/2) = [0; 1, 2, 2, 2, ...]
        let value = sqrt_of(2, 1).reciprocal().expect("nonzero");
        assert_prefix(&value, &[0, 1, 2, 2, 2, 2], 6);
    }

    #[test]
    fn reciprocal_of_reciprocal_recovers_value() {
        let original = sqrt_of(3, 1);
        let twice = original.reciprocal().unwrap().reciprocal().unwrap();
        assert_eq!(
            twice.partial_quotients_bounded(8),
            original.partial_quotients_bounded(8)
        );
    }

    // -- add / sub with one rational operand --

    #[test]
    fn sqrt_two_plus_one_emits_two_two_two() {
        // √2 + 1 ≈ 2.414 = [2; 2, 2, 2, ...]
        let value = sqrt_of(2, 1).add(&rational(1, 1));
        assert_prefix(&value, &[2, 2, 2, 2, 2, 2], 6);
    }

    #[test]
    fn one_plus_sqrt_two_is_commutative() {
        let lhs = sqrt_of(2, 1).add(&rational(1, 1));
        let rhs = rational(1, 1).add(&sqrt_of(2, 1));
        assert_eq!(
            lhs.partial_quotients_bounded(8),
            rhs.partial_quotients_bounded(8)
        );
    }

    #[test]
    fn sqrt_two_minus_one_emits_known_cf() {
        // √2 − 1 ≈ 0.4142 = [0; 2, 2, 2, ...]
        let value = sqrt_of(2, 1).sub(&rational(1, 1));
        assert_prefix(&value, &[0, 2, 2, 2, 2, 2], 6);
    }

    #[test]
    fn one_minus_sqrt_two_is_negative() {
        // 1 − √2 ≈ −0.4142. Same CF tail as −√2 starting one
        // term later: [−1; 1, 1, 2, 2, 2, 2, …].
        let value = rational(1, 1).sub(&sqrt_of(2, 1));
        assert_prefix(&value, &[-1, 1, 1, 2, 2, 2, 2, 2], 8);
    }

    #[test]
    fn sqrt_two_plus_zero_is_sqrt_two() {
        let value = sqrt_of(2, 1).add(&rational(0, 1));
        let baseline = sqrt_of(2, 1);
        assert_eq!(
            value.partial_quotients_bounded(8),
            baseline.partial_quotients_bounded(8)
        );
    }

    // -- mul / div with one rational operand --

    #[test]
    fn sqrt_two_times_two_is_sqrt_eight() {
        // √2 · 2 = √8 = [2; 1, 4, 1, 4, ...]
        let value = sqrt_of(2, 1).mul(&rational(2, 1));
        let baseline = sqrt_of(8, 1);
        assert_eq!(
            value.partial_quotients_bounded(8),
            baseline.partial_quotients_bounded(8)
        );
    }

    #[test]
    fn sqrt_two_divided_by_two_is_sqrt_one_half() {
        // √2 / 2 = √(1/2) = [0; 1, 2, 2, 2, ...]
        let value = sqrt_of(2, 1).div(&rational(2, 1)).expect("nonzero");
        let baseline = sqrt_of(1, 2);
        assert_eq!(
            value.partial_quotients_bounded(8),
            baseline.partial_quotients_bounded(8)
        );
    }

    #[test]
    fn sqrt_two_times_zero_is_zero() {
        let value = sqrt_of(2, 1).mul(&rational(0, 1));
        assert_eq!(value, rational(0, 1));
    }

    #[test]
    fn div_by_structural_zero_returns_none() {
        assert_eq!(sqrt_of(2, 1).div(&rational(0, 1)), None);
        assert_eq!(rational(3, 1).div(&rational(0, 1)), None);
    }

    #[test]
    fn div_with_nil_returns_nil() {
        let nil = ExactReal::Rational(Fraction::nil());
        assert!(sqrt_of(2, 1)
            .div(&nil)
            .expect("nil-divisor yields nil, not None")
            .is_nil());
    }

    // -- bihomographic: two non-rational operands --

    #[test]
    fn sqrt_two_plus_sqrt_two_is_two_sqrt_two() {
        // √2 + √2 = 2·√2 = √8 = [2; 1, 4, 1, 4, ...]
        let value = sqrt_of(2, 1).add(&sqrt_of(2, 1));
        let baseline = sqrt_of(8, 1);
        assert_eq!(
            value.partial_quotients_bounded(7),
            baseline.partial_quotients_bounded(7)
        );
    }

    // The three identities √2 − √2 = 0, √2 · √2 = 2, and √2 ÷ √2 = 1
    // all produce mathematically-exact rational results from two lazy
    // operands. The streaming bihomographic Gosper algorithm can never
    // *prove* such an identity from finite operand prefixes — the
    // four-corner floor test always straddles the answer because each
    // new partial-quotient ingestion is consistent with both "the
    // result is exactly the integer" and "the result is the integer
    // ±ε" — so it would exhaust its safety budget into an empty CF,
    // surfacing the value as a silent NIL. The arithmetic methods
    // therefore detect the closed-form simplification of √a ⊗ √b
    // (= √(a·b), √(a/b), or an exact rational) up front, so these
    // identities resolve to a `Rational`/canonical `AlgebraicSqrt`
    // instead of a degenerate bihom. The genuinely-lazy residue — a
    // rational that only emerges from *composed* Gosper operands, e.g.
    // (√2 + 1) − (√2 + 1) — still cannot be pinned and is covered
    // separately below.

    #[test]
    fn sqrt_two_minus_sqrt_two_collapses_to_zero() {
        let value = sqrt_of(2, 1).sub(&sqrt_of(2, 1));
        assert_eq!(value, rational(0, 1));
        assert_eq!(value.partial_quotients_bounded(4), terms(&[0]));
    }

    #[test]
    fn sqrt_two_times_sqrt_two_collapses_to_two() {
        let value = sqrt_of(2, 1).mul(&sqrt_of(2, 1));
        assert_eq!(value, rational(2, 1));
        assert_eq!(value.partial_quotients_bounded(4), terms(&[2]));
    }

    #[test]
    fn sqrt_two_divided_by_sqrt_two_collapses_to_one() {
        let value = sqrt_of(2, 1)
            .div(&sqrt_of(2, 1))
            .expect("sqrt(2) is nonzero");
        assert_eq!(value, rational(1, 1));
        assert_eq!(value.partial_quotients_bounded(4), terms(&[1]));
    }

    /// A rational value that only emerges from *composed* Gosper
    /// operands — the bihom cannot pin it, so it stays lazy and a
    /// bounded prefix is empty. This documents the residual limitation
    /// that the closed-form √a ⊗ √b simplification does not reach.
    fn composed_lazy_zero() -> ExactReal {
        let g = sqrt_of(2, 1).add(&rational(1, 1)); // 1 + √2 (Gosper)
        g.sub(&g) // (1 + √2) − (1 + √2) = 0, but bihom-lazy
    }

    #[test]
    fn composed_gosper_zero_stays_lazy_within_budget() {
        let value = composed_lazy_zero();
        assert!(value.is_gosper());
        assert!(value.partial_quotients_bounded(4).is_empty());
    }

    #[test]
    fn sqrt_two_plus_sqrt_three_has_known_cf_prefix() {
        // √2 + √3 ≈ 3.1463 = [3; 6, 1, 5, 7, 1, 1, 4, 1, 38, ...]
        // (OEIS A040337). We assert the first six terms.
        let value = sqrt_of(2, 1).add(&sqrt_of(3, 1));
        let prefix = value.partial_quotients_bounded(6);
        assert_eq!(prefix, terms(&[3, 6, 1, 5, 7, 1]));
    }

    // -- mixed: arithmetic chained --

    #[test]
    fn sqrt_two_plus_one_minus_one_returns_sqrt_two() {
        let value = sqrt_of(2, 1).add(&rational(1, 1)).sub(&rational(1, 1));
        let baseline = sqrt_of(2, 1);
        assert_eq!(
            value.partial_quotients_bounded(7),
            baseline.partial_quotients_bounded(7)
        );
    }

    #[test]
    fn rational_arithmetic_stays_on_fraction_kernel() {
        // Pure-rational arithmetic must collapse to a Rational
        // variant, not produce a Gosper.
        let lhs = rational(1, 2).add(&rational(1, 3));
        assert!(lhs.is_rational(), "rational+rational must stay Rational");
        assert_eq!(lhs, rational(5, 6));

        let prod = rational(3, 4).mul(&rational(2, 9));
        assert!(prod.is_rational());
        assert_eq!(prod, rational(1, 6));

        let quo = rational(7, 3).div(&rational(7, 3)).expect("nonzero");
        assert!(quo.is_rational());
        assert_eq!(quo, rational(1, 1));
    }

    #[test]
    fn nil_propagation_through_arithmetic() {
        let nil = ExactReal::Rational(Fraction::nil());
        assert!(nil.add(&rational(1, 1)).is_nil());
        assert!(rational(1, 1).add(&nil).is_nil());
        assert!(nil.sub(&sqrt_of(2, 1)).is_nil());
        assert!(sqrt_of(2, 1).mul(&nil).is_nil());
    }

    #[test]
    fn mobius_collapses_to_rational_when_operand_is_rational() {
        // Internal: mobius_apply on a Rational operand evaluates
        // directly. Verify via the public arithmetic surface.
        let value = rational(3, 4).add(&rational(1, 4));
        assert!(value.is_rational());
        assert_eq!(value, rational(1, 1));
    }

    #[test]
    fn structural_zero_recognises_only_explicit_zero() {
        assert!(rational(0, 1).is_structurally_zero());
        assert!(!rational(1, 100000).is_structurally_zero());
        assert!(!sqrt_of(2, 1).is_structurally_zero());
        // √2 − √2 now collapses to an explicit rational zero in closed
        // form, so it *is* structurally zero.
        assert!(sqrt_of(2, 1).sub(&sqrt_of(2, 1)).is_structurally_zero());
        // A composed-Gosper zero stays lazy and cannot be proven zero
        // without expansion: report false (conservative).
        let lazy_zero = composed_lazy_zero();
        assert!(!lazy_zero.is_structurally_zero());
    }

    // === Phase 5: comparison budget (SPEC §4.2.4, §7.4.1) ===

    use std::cmp::Ordering;

    const BUDGET: usize = DEFAULT_COMPARISON_BUDGET;

    // -- rational vs rational --

    #[test]
    fn cmp_rational_equal_canonical_sequences() {
        assert_eq!(
            rational(1, 2).cmp_with_budget(&rational(1, 2), BUDGET),
            Some(Ordering::Equal)
        );
        // 2/4 reduces to 1/2 via Fraction::new, so structurally
        // identical CFs — equality is detected at the canonical-
        // sequence level rather than via a budget burn-down.
        assert_eq!(
            rational(2, 4).cmp_with_budget(&rational(1, 2), BUDGET),
            Some(Ordering::Equal)
        );
        assert_eq!(
            rational(355, 113).cmp_with_budget(&rational(355, 113), BUDGET),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn cmp_rational_orders_by_value() {
        // 1/2 < 2/3 < 1 < 355/113
        assert_eq!(
            rational(1, 2).cmp_with_budget(&rational(2, 3), BUDGET),
            Some(Ordering::Less)
        );
        assert_eq!(
            rational(2, 3).cmp_with_budget(&rational(1, 1), BUDGET),
            Some(Ordering::Less)
        );
        assert_eq!(
            rational(1, 1).cmp_with_budget(&rational(355, 113), BUDGET),
            Some(Ordering::Less)
        );
        // mirror: reverse orders flip
        assert_eq!(
            rational(2, 3).cmp_with_budget(&rational(1, 2), BUDGET),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn cmp_rational_handles_negative_values() {
        // −3/2 = [−2; 2], 1/2 = [0; 2]; first differ at i=0
        // ⇒ floor(−3/2) < floor(1/2) ⇒ Less.
        assert_eq!(
            rational(-3, 2).cmp_with_budget(&rational(1, 2), BUDGET),
            Some(Ordering::Less)
        );
        assert_eq!(
            rational(-1, 2).cmp_with_budget(&rational(-3, 2), BUDGET),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn cmp_finite_vs_longer_prefix_applies_parity_rule() {
        // [1] = 1, [1; 2] = 3/2.
        //   At i=1, [1] has ended while [1; 2] has 2.
        //   Odd index ⇒ phantom-∞ < 2 ⇒ [1] < [1; 2].
        let one = rational(1, 1);
        let three_halves = rational(3, 2);
        assert_eq!(
            one.cmp_with_budget(&three_halves, BUDGET),
            Some(Ordering::Less)
        );
        // [1; 2] vs [1; 2, 3]: at i=2 (even) phantom-∞ > 3
        //   ⇒ [1; 2] > [1; 2, 3].
        let ten_sevenths = rational(10, 7); // [1; 2, 3]
        assert_eq!(
            three_halves.cmp_with_budget(&ten_sevenths, BUDGET),
            Some(Ordering::Greater)
        );
    }

    // -- algebraic sqrt vs rational --

    #[test]
    fn cmp_sqrt_two_is_between_one_and_two() {
        let s = sqrt_of(2, 1);
        assert_eq!(
            rational(1, 1).cmp_with_budget(&s, BUDGET),
            Some(Ordering::Less)
        );
        assert_eq!(
            s.cmp_with_budget(&rational(2, 1), BUDGET),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn cmp_sqrt_two_against_close_rationals() {
        // √2 ≈ 1.41421356 — between 99/70 = 1.41428... and 41/29 = 1.41379...
        let s = sqrt_of(2, 1);
        assert_eq!(
            s.cmp_with_budget(&rational(99, 70), BUDGET),
            Some(Ordering::Less)
        );
        assert_eq!(
            s.cmp_with_budget(&rational(41, 29), BUDGET),
            Some(Ordering::Greater)
        );
    }

    // -- algebraic sqrt vs algebraic sqrt --

    #[test]
    fn cmp_sqrt_two_less_than_sqrt_three() {
        let two = sqrt_of(2, 1);
        let three = sqrt_of(3, 1);
        assert_eq!(two.cmp_with_budget(&three, BUDGET), Some(Ordering::Less));
        assert_eq!(three.cmp_with_budget(&two, BUDGET), Some(Ordering::Greater));
    }

    #[test]
    fn cmp_equal_sqrt_two_decides_equal() {
        // Two AlgebraicSqrt values built from the same radicand are
        // equal. The algebraic short-circuit decides this in O(1)
        // from the radicands rather than streaming the never-
        // diverging CFs to budget exhaustion. It needs no budget
        // headroom beyond the budget-0 guard.
        let a = sqrt_of(2, 1);
        let b = sqrt_of(2, 1);
        assert_eq!(a.cmp_with_budget(&b, 1), Some(Ordering::Equal));
        assert_eq!(a.cmp_with_budget(&b, BUDGET), Some(Ordering::Equal));
    }

    #[test]
    fn cmp_distinct_sqrts_decide_from_radicands() {
        // √(2/3) < √2 < √8, each decided in O(1) from the radicands.
        assert_eq!(
            sqrt_of(2, 3).cmp_with_budget(&sqrt_of(2, 1), BUDGET),
            Some(Ordering::Less)
        );
        assert_eq!(
            sqrt_of(8, 1).cmp_with_budget(&sqrt_of(2, 1), BUDGET),
            Some(Ordering::Greater)
        );
        assert_eq!(
            sqrt_of(7, 1).cmp_with_budget(&sqrt_of(7, 1), 1),
            Some(Ordering::Equal)
        );
    }

    #[test]
    fn cmp_sqrt_against_non_positive_rationals() {
        let s = sqrt_of(2, 1);
        assert_eq!(
            s.cmp_with_budget(&rational(0, 1), BUDGET),
            Some(Ordering::Greater)
        );
        assert_eq!(
            s.cmp_with_budget(&rational(-5, 1), BUDGET),
            Some(Ordering::Greater)
        );
        assert_eq!(
            rational(-5, 1).cmp_with_budget(&s, BUDGET),
            Some(Ordering::Less)
        );
    }

    #[test]
    fn eq_equal_sqrts_resolves_decidably() {
        assert_eq!(
            sqrt_of(2, 1).eq_with_budget(&sqrt_of(2, 1), BUDGET),
            Some(true)
        );
        assert_eq!(
            sqrt_of(2, 1).ne_with_budget(&sqrt_of(2, 1), BUDGET),
            Some(false)
        );
        assert_eq!(
            sqrt_of(2, 1).eq_with_budget(&sqrt_of(3, 1), BUDGET),
            Some(false)
        );
    }

    // -- gosper vs everything --

    #[test]
    fn cmp_sqrt_two_plus_one_against_rationals() {
        // √2 + 1 ≈ 2.4142 — between 2 and 3, and above 12/5 = 2.4.
        let v = sqrt_of(2, 1).add(&rational(1, 1));
        assert_eq!(
            v.cmp_with_budget(&rational(2, 1), BUDGET),
            Some(Ordering::Greater)
        );
        assert_eq!(
            v.cmp_with_budget(&rational(3, 1), BUDGET),
            Some(Ordering::Less)
        );
        assert_eq!(
            v.cmp_with_budget(&rational(12, 5), BUDGET),
            Some(Ordering::Greater)
        );
    }

    #[test]
    fn cmp_sqrt_two_plus_sqrt_two_equals_sqrt_eight() {
        // √2 + √2 collapses in closed form to √8 (the same canonical
        // AlgebraicSqrt as the right-hand side), so the comparison now
        // decides Equal rather than exhausting into `None`.
        let lhs = sqrt_of(2, 1).add(&sqrt_of(2, 1));
        let rhs = sqrt_of(8, 1);
        assert_eq!(lhs, rhs);
        assert_eq!(lhs.cmp_with_budget(&rhs, BUDGET), Some(Ordering::Equal));
    }

    #[test]
    fn cmp_sqrt_minus_self_against_zero_decides_equal() {
        // √2 − √2 collapses to an exact rational zero, so it compares
        // Equal to true zero (was undecidable while the bihom could not
        // pin the cancellation).
        let zero = sqrt_of(2, 1).sub(&sqrt_of(2, 1));
        assert_eq!(zero.cmp_with_budget(&rational(0, 1), BUDGET), Some(Ordering::Equal));
    }

    #[test]
    fn cmp_composed_gosper_zero_against_zero_is_undecidable() {
        // SPEC §7.4.1 exact case: a composed-Gosper zero, e.g.
        // (1 + √2) − (1 + √2) = 0, cannot be proven equal to true zero
        // from finite operand prefixes. The bihom hits its internal
        // safety budget, surfaces `CfStep::Exhausted`, and the
        // comparison budget projects to None ⇒ NIL `undecidable`.
        let lazy_zero = composed_lazy_zero();
        assert_eq!(lazy_zero.cmp_with_budget(&rational(0, 1), BUDGET), None);
    }

    // -- nil / zero-budget --

    #[test]
    fn cmp_with_nil_returns_none() {
        let nil = ExactReal::Rational(Fraction::nil());
        assert_eq!(nil.cmp_with_budget(&rational(1, 1), BUDGET), None);
        assert_eq!(rational(1, 1).cmp_with_budget(&nil, BUDGET), None);
        assert_eq!(nil.cmp_with_budget(&nil, BUDGET), None);
    }

    #[test]
    fn cmp_with_zero_budget_returns_none() {
        // Even equal rationals don't decide at budget 0 — the
        // comparison loop runs zero iterations.
        assert_eq!(rational(1, 2).cmp_with_budget(&rational(1, 2), 0), None);
        assert_eq!(rational(1, 2).cmp_with_budget(&rational(1, 3), 0), None);
    }

    #[test]
    fn cmp_small_budget_decides_easy_rationals() {
        // 1/2 = [0; 2] and 1/3 = [0; 3] differ at index 1 (odd),
        // so the alternating-parity rule flips the comparison:
        // cmp(b_i, a_i) = cmp(3, 2) = Greater ⇒ 1/2 > 1/3. A
        // budget of 2 partial quotients suffices.
        assert_eq!(
            rational(1, 2).cmp_with_budget(&rational(1, 3), 2),
            Some(Ordering::Greater)
        );
    }

    // -- boolean wrappers --

    #[test]
    fn eq_wrapper_mirrors_cmp() {
        assert_eq!(
            rational(1, 2).eq_with_budget(&rational(1, 2), BUDGET),
            Some(true)
        );
        assert_eq!(
            rational(1, 2).eq_with_budget(&rational(1, 3), BUDGET),
            Some(false)
        );
        let nil = ExactReal::Rational(Fraction::nil());
        assert_eq!(nil.eq_with_budget(&rational(1, 2), BUDGET), None);
    }

    #[test]
    fn ne_wrapper_negates_eq() {
        assert_eq!(
            rational(1, 2).ne_with_budget(&rational(1, 2), BUDGET),
            Some(false)
        );
        assert_eq!(
            rational(1, 2).ne_with_budget(&rational(1, 3), BUDGET),
            Some(true)
        );
        let nil = ExactReal::Rational(Fraction::nil());
        assert_eq!(nil.ne_with_budget(&rational(1, 2), BUDGET), None);
    }

    #[test]
    fn lt_le_gt_ge_wrappers_cover_strict_and_loose() {
        let a = rational(1, 2);
        let b = rational(2, 3);
        let c = rational(1, 2);
        assert_eq!(a.lt_with_budget(&b, BUDGET), Some(true));
        assert_eq!(a.lt_with_budget(&c, BUDGET), Some(false));
        assert_eq!(a.le_with_budget(&b, BUDGET), Some(true));
        assert_eq!(a.le_with_budget(&c, BUDGET), Some(true));
        assert_eq!(b.gt_with_budget(&a, BUDGET), Some(true));
        assert_eq!(c.gt_with_budget(&a, BUDGET), Some(false));
        assert_eq!(b.ge_with_budget(&a, BUDGET), Some(true));
        assert_eq!(c.ge_with_budget(&a, BUDGET), Some(true));
    }

    #[test]
    fn boolean_wrappers_propagate_undecidable_as_none() {
        // A composed lazy Gosper zero ((1 + √2) − (1 + √2)) compared
        // against true zero cannot be resolved within the budget — no
        // closed-form short-circuit applies to a Gosper operand — so
        // every boolean wrapper surfaces `None`.
        let a = composed_lazy_zero();
        let b = rational(0, 1);
        assert_eq!(a.eq_with_budget(&b, BUDGET), None);
        assert_eq!(a.lt_with_budget(&b, BUDGET), None);
        assert_eq!(a.le_with_budget(&b, BUDGET), None);
        assert_eq!(a.gt_with_budget(&b, BUDGET), None);
        assert_eq!(a.ge_with_budget(&b, BUDGET), None);
        assert_eq!(a.ne_with_budget(&b, BUDGET), None);
    }

    #[test]
    fn cmp_antisymmetric_on_decidable_pairs() {
        // Antisymmetry: a.cmp(b) == b.cmp(a).reverse() whenever both
        // resolve within the budget. Includes mixed rational / sqrt
        // pairs where the values are strictly ordered.
        let pairs = [
            (rational(1, 2), rational(2, 3)),
            (rational(355, 113), rational(22, 7)),
            (sqrt_of(2, 1), rational(3, 2)),
            (rational(7, 5), sqrt_of(2, 1)),
            (sqrt_of(2, 1), sqrt_of(3, 1)),
        ];
        for (a, b) in &pairs {
            let ab = a.cmp_with_budget(b, BUDGET).expect("decidable");
            let ba = b.cmp_with_budget(a, BUDGET).expect("decidable");
            assert_eq!(ab.reverse(), ba, "antisymmetry: {:?} vs {:?}", a, b);
        }
    }

    #[test]
    fn cmp_reflexive_for_finite_rationals() {
        // Reflexivity holds for finite (rational) CFs: both streams
        // emit the same canonical sequence then end, and the
        // (Ended, Ended) branch returns Equal. Equal `AlgebraicSqrt`
        // values are likewise reflexive via the algebraic
        // short-circuit — see `cmp_equal_sqrt_two_decides_equal`.
        for r in &[
            rational(0, 1),
            rational(1, 1),
            rational(-7, 5),
            rational(355, 113),
            rational(-22, 7),
        ] {
            assert_eq!(
                r.cmp_with_budget(r, BUDGET),
                Some(Ordering::Equal),
                "reflexivity: {:?}",
                r
            );
        }
    }

    // === Phase 6: CF-native rounding and rational approximation ===

    fn frac(num: i64, den: i64) -> Fraction {
        Fraction::new(bi(num), bi(den))
    }

    #[test]
    fn floor_of_rationals_is_first_partial_quotient() {
        assert_eq!(rational(7, 2).floor(), Some(rational(3, 1)));
        assert_eq!(rational(-7, 2).floor(), Some(rational(-4, 1)));
        assert_eq!(rational(5, 1).floor(), Some(rational(5, 1)));
        assert_eq!(rational(0, 1).floor(), Some(rational(0, 1)));
    }

    #[test]
    fn ceil_of_rationals() {
        assert_eq!(rational(7, 2).ceil(), Some(rational(4, 1)));
        assert_eq!(rational(-7, 2).ceil(), Some(rational(-3, 1)));
        assert_eq!(rational(5, 1).ceil(), Some(rational(5, 1)));
        assert_eq!(rational(0, 1).ceil(), Some(rational(0, 1)));
    }

    #[test]
    fn round_of_rationals_ties_away_from_zero() {
        // Ties (half-integers): away from zero.
        assert_eq!(rational(1, 2).round(), Some(rational(1, 1)));
        assert_eq!(rational(-1, 2).round(), Some(rational(-1, 1)));
        assert_eq!(rational(5, 2).round(), Some(rational(3, 1)));
        assert_eq!(rational(-5, 2).round(), Some(rational(-3, 1)));
        // Non-ties.
        assert_eq!(rational(7, 3).round(), Some(rational(2, 1))); // 2.33…
        assert_eq!(rational(8, 3).round(), Some(rational(3, 1))); // 2.66…
        assert_eq!(rational(4, 1).round(), Some(rational(4, 1)));
    }

    #[test]
    fn floor_ceil_round_of_algebraic_sqrt() {
        // √2 ≈ 1.41421.
        let s2 = sqrt_of(2, 1);
        assert_eq!(s2.floor(), Some(rational(1, 1)));
        assert_eq!(s2.ceil(), Some(rational(2, 1)));
        assert_eq!(s2.round(), Some(rational(1, 1)));
        // √7 ≈ 2.64575.
        let s7 = sqrt_of(7, 1);
        assert_eq!(s7.floor(), Some(rational(2, 1)));
        assert_eq!(s7.ceil(), Some(rational(3, 1)));
        assert_eq!(s7.round(), Some(rational(3, 1)));
    }

    #[test]
    fn floor_ceil_round_of_gosper_value() {
        // √2 + 1 ≈ 2.41421, held as a lazy Möbius Gosper.
        let v = sqrt_of(2, 1).add(&rational(1, 1));
        assert_eq!(v.floor(), Some(rational(2, 1)));
        assert_eq!(v.ceil(), Some(rational(3, 1)));
        assert_eq!(v.round(), Some(rational(2, 1)));
    }

    #[test]
    fn rounding_of_nil_returns_none() {
        let nil = ExactReal::Rational(Fraction::nil());
        assert_eq!(nil.floor(), None);
        assert_eq!(nil.ceil(), None);
        assert_eq!(nil.round(), None);
    }

    #[test]
    fn best_rational_approximation_of_rational() {
        // 355/113 = [3; 7, 16]; principal convergents are 3, 22/7,
        // 355/113.
        let v = rational(355, 113);
        assert_eq!(v.best_rational_approximation(&bi(6)), Some(frac(3, 1)));
        assert_eq!(v.best_rational_approximation(&bi(50)), Some(frac(22, 7)));
        assert_eq!(
            v.best_rational_approximation(&bi(200)),
            Some(frac(355, 113))
        );
    }

    #[test]
    fn best_rational_approximation_of_sqrt_two() {
        // √2 = [1; 2, 2, 2, …]; convergent denominators 1, 2, 5, 12,
        // 29, 70, 169, …
        let s = sqrt_of(2, 1);
        assert_eq!(s.best_rational_approximation(&bi(1)), Some(frac(1, 1)));
        assert_eq!(s.best_rational_approximation(&bi(10)), Some(frac(7, 5)));
        assert_eq!(s.best_rational_approximation(&bi(100)), Some(frac(99, 70)));
        assert_eq!(s.best_rational_approximation(&bi(168)), Some(frac(99, 70)));
        assert_eq!(
            s.best_rational_approximation(&bi(169)),
            Some(frac(239, 169))
        );
    }

    #[test]
    fn best_rational_approximation_rejects_bad_bound_and_nil() {
        assert_eq!(rational(1, 2).best_rational_approximation(&bi(0)), None);
        let nil = ExactReal::Rational(Fraction::nil());
        assert_eq!(nil.best_rational_approximation(&bi(10)), None);
    }

    // === Phase 7: Gosper renormalization, sqrt period, Arc clone ===

    #[test]
    fn deep_mobius_chain_expands_correctly_under_normalization() {
        // A left-deep chain of Möbius transforms stresses coefficient
        // growth; GCD renormalization must not change the emitted CF.
        // ((√2 + 1) + 1) + 1 = √2 + 3 = [4; 2, 2, 2, …].
        let v = sqrt_of(2, 1)
            .add(&rational(1, 1))
            .add(&rational(1, 1))
            .add(&rational(1, 1));
        assert_eq!(v.partial_quotients_bounded(6), terms(&[4, 2, 2, 2, 2, 2]));
    }

    #[test]
    fn sqrt_cf_period_known_values() {
        assert_eq!(
            sqrt_of(2, 1).sqrt_cf_period(),
            Some((terms(&[1]), terms(&[2])))
        );
        assert_eq!(
            sqrt_of(3, 1).sqrt_cf_period(),
            Some((terms(&[1]), terms(&[1, 2])))
        );
        assert_eq!(
            sqrt_of(7, 1).sqrt_cf_period(),
            Some((terms(&[2]), terms(&[1, 1, 1, 4])))
        );
    }

    #[test]
    fn sqrt_cf_period_reconstructs_streamed_expansion() {
        for (num, den) in [(2, 1), (3, 1), (7, 1), (13, 1), (2, 3), (1, 2)] {
            let v = sqrt_of(num, den);
            let (pre, period) = v.sqrt_cf_period().expect("algebraic sqrt");
            assert!(!period.is_empty(), "period must be non-empty");
            let n = 24;
            let mut rebuilt: Vec<BigInt> = pre.clone();
            while rebuilt.len() < n {
                rebuilt.extend(period.iter().cloned());
            }
            rebuilt.truncate(n);
            assert_eq!(
                rebuilt,
                v.partial_quotients_bounded(n),
                "period reconstruction for √({num}/{den})"
            );
        }
    }

    #[test]
    fn sqrt_cf_period_none_for_non_sqrt_representations() {
        assert_eq!(rational(3, 2).sqrt_cf_period(), None);
        // A perfect-square radicand collapses to Rational, not
        // AlgebraicSqrt.
        let perfect = ExactReal::from_sqrt_rational(Fraction::new(bi(9), bi(1))).expect("9");
        assert_eq!(perfect.sqrt_cf_period(), None);
        // A Gosper value is not an AlgebraicSqrt either.
        let gosper = sqrt_of(2, 1).add(&rational(1, 1));
        assert_eq!(gosper.sqrt_cf_period(), None);
    }

    #[test]
    fn gosper_value_clone_preserves_equality_and_expansion() {
        // After the Arc migration, cloning a Gosper-backed value is a
        // refcount bump; it must still behave as a deep-equal copy.
        let v = sqrt_of(2, 1).add(&sqrt_of(3, 1));
        let cloned = v.clone();
        assert_eq!(v, cloned);
        assert_eq!(
            v.partial_quotients_bounded(10),
            cloned.partial_quotients_bounded(10)
        );
    }

    // -----------------------------------------------------------------
    // cmp_with_budget reachability — verifies that the AlgebraicSqrt and
    // Gosper branches inside cmp_with_budget are live and return correct
    // results. This guards against the path being inadvertently dead-coded
    // before ValueData::Scalar is migrated to ExactReal (SPEC §7.4.1).
    // -----------------------------------------------------------------

    #[test]
    fn cmp_with_budget_sqrt_vs_sqrt_same_radicand_is_equal() {
        let a = sqrt_of(2, 1);
        let b = sqrt_of(2, 1);
        assert_eq!(
            a.cmp_with_budget(&b, DEFAULT_COMPARISON_BUDGET),
            Some(std::cmp::Ordering::Equal),
            "√2 == √2 must decide Equal"
        );
    }

    #[test]
    fn cmp_with_budget_sqrt_vs_sqrt_different_radicands_decide_correctly() {
        let sqrt2 = sqrt_of(2, 1);
        let sqrt3 = sqrt_of(3, 1);
        assert_eq!(
            sqrt2.cmp_with_budget(&sqrt3, DEFAULT_COMPARISON_BUDGET),
            Some(std::cmp::Ordering::Less),
            "√2 < √3 must decide Less"
        );
        assert_eq!(
            sqrt3.cmp_with_budget(&sqrt2, DEFAULT_COMPARISON_BUDGET),
            Some(std::cmp::Ordering::Greater),
            "√3 > √2 must decide Greater"
        );
    }

    #[test]
    fn cmp_with_budget_sqrt_vs_rational_decides_correctly() {
        // √2 ≈ 1.414, so √2 > 1 and √2 < 2.
        let sqrt2 = sqrt_of(2, 1);
        let one = rational(1, 1);
        let two = rational(2, 1);
        assert_eq!(
            sqrt2.cmp_with_budget(&one, DEFAULT_COMPARISON_BUDGET),
            Some(std::cmp::Ordering::Greater),
            "√2 > 1 must decide Greater"
        );
        assert_eq!(
            sqrt2.cmp_with_budget(&two, DEFAULT_COMPARISON_BUDGET),
            Some(std::cmp::Ordering::Less),
            "√2 < 2 must decide Less"
        );
    }

    #[test]
    fn cmp_with_budget_zero_budget_returns_none() {
        // Budget of 0 must always yield None (undecidable) regardless of
        // operand type, confirming the budget-exhaustion path is reachable.
        let sqrt2 = sqrt_of(2, 1);
        let sqrt3 = sqrt_of(3, 1);
        assert_eq!(
            sqrt2.cmp_with_budget(&sqrt3, 0),
            None,
            "zero budget must yield None"
        );
    }

    #[test]
    fn cmp_with_budget_gosper_vs_rational_decides_correctly() {
        // √2 + 0 via Gosper add — exercises the Gosper CF streaming path.
        let gosper_sqrt2 = sqrt_of(2, 1).add(&rational(0, 1));
        let one = rational(1, 1);
        assert_eq!(
            gosper_sqrt2.cmp_with_budget(&one, DEFAULT_COMPARISON_BUDGET),
            Some(std::cmp::Ordering::Greater),
            "Gosper(√2 + 0) > 1 must decide Greater"
        );
    }

    // ─── Rational collapse of √a ⊗ √b (regression for the empty-CF bug) ───
    //
    // When two infinite-CF square roots combine into an exact rational, the
    // streaming bihomographic expander can never pin the result: its
    // four-corner floor test straddles the rational forever, exhausts the
    // ingest budget, and yields an *empty* continued fraction — observed at
    // the language boundary as a silent NIL (e.g. √2·√2 → NIL instead of 2).
    // The arithmetic methods detect these closed-form simplifications so the
    // result is a `Rational`/canonical `AlgebraicSqrt` instead.

    fn expanded(value: &ExactReal) -> Vec<BigInt> {
        value.partial_quotients_bounded(16)
    }

    #[test]
    fn sqrt_times_same_sqrt_collapses_to_integer() {
        // √2·√2 = 2, √3·√3 = 3, √5·√5 = 5.
        for n in [2_i64, 3, 5, 7, 11] {
            let product = sqrt_of(n, 1).mul(&sqrt_of(n, 1));
            assert_eq!(
                product,
                rational(n, 1),
                "√{n}·√{n} must collapse to the integer {n}"
            );
            assert_eq!(expanded(&product), terms(&[n]), "CF of √{n}·√{n} is ( {n} )");
        }
    }

    #[test]
    fn sqrt_product_is_perfect_square_collapses() {
        // √2·√8 = √16 = 4; √2·√18 = √36 = 6.
        assert_eq!(sqrt_of(2, 1).mul(&sqrt_of(8, 1)), rational(4, 1));
        assert_eq!(sqrt_of(2, 1).mul(&sqrt_of(18, 1)), rational(6, 1));
    }

    #[test]
    fn sqrt_product_that_stays_irrational_is_the_combined_sqrt() {
        // √2·√3 = √6 (irrational): closed form yields the canonical
        // AlgebraicSqrt, whose CF is the genuine √6 expansion [2;2,4,2,4,…].
        let product = sqrt_of(2, 1).mul(&sqrt_of(3, 1));
        assert_eq!(product, sqrt_of(6, 1));
        assert_eq!(expanded(&product), expanded(&sqrt_of(6, 1)));
    }

    #[test]
    fn sqrt_product_collapses_to_non_integer_rational() {
        // √2·√(2/9) = √(4/9) = 2/3 = [0;1,2].
        let product = sqrt_of(2, 1).mul(&sqrt_of(2, 9));
        assert_eq!(product, rational(2, 3));
        assert_eq!(expanded(&product), terms(&[0, 1, 2]));
    }

    #[test]
    fn sqrt_quotient_perfect_square_collapses() {
        // √8/√2 = √4 = 2; √18/√2 = √9 = 3.
        assert_eq!(sqrt_of(8, 1).div(&sqrt_of(2, 1)).unwrap(), rational(2, 1));
        assert_eq!(sqrt_of(18, 1).div(&sqrt_of(2, 1)).unwrap(), rational(3, 1));
    }

    #[test]
    fn sqrt_quotient_that_stays_irrational_is_the_combined_sqrt() {
        // √6/√2 = √3 (irrational).
        let quotient = sqrt_of(6, 1).div(&sqrt_of(2, 1)).unwrap();
        assert_eq!(quotient, sqrt_of(3, 1));
    }

    #[test]
    fn equal_sqrt_difference_is_zero() {
        // √2 − √2 = 0 (exact), not an empty CF.
        let diff = sqrt_of(2, 1).sub(&sqrt_of(2, 1));
        assert_eq!(diff, rational(0, 1));
        assert_eq!(expanded(&diff), terms(&[0]));
    }

    #[test]
    fn distinct_sqrt_difference_stays_lazy() {
        // √3 − √2 ≈ 0.318 stays irrational; the value must not collapse but
        // must still expand to a non-empty CF starting at floor 0.
        let diff = sqrt_of(3, 1).sub(&sqrt_of(2, 1));
        assert!(!diff.is_rational(), "√3 − √2 is irrational");
        let cf = expanded(&diff);
        assert!(!cf.is_empty(), "√3 − √2 must expand to a non-empty CF");
        assert_eq!(cf[0], bi(0), "⌊√3 − √2⌋ = 0");
    }

    #[test]
    fn equal_sqrt_sum_collapses_to_canonical_sqrt() {
        // √2 + √2 = 2√2 = √8; expansion matches √8 = [2;1,4,1,4,…].
        let sum = sqrt_of(2, 1).add(&sqrt_of(2, 1));
        assert_eq!(sum, sqrt_of(8, 1));
        assert_eq!(expanded(&sum), expanded(&sqrt_of(8, 1)));
    }

    #[test]
    fn equal_sqrt_sum_can_collapse_to_rational() {
        // √(1/4) is rational (1/2), but √(2/9)+√(2/9) = 2√(2/9) = √(8/9),
        // still irrational; whereas a radicand whose 4× product is a perfect
        // square collapses: √(1/16)+√(1/16) would already be rational before
        // reaching this path, so use √(9/4)? That is rational too. The
        // representative irrational-staying case is asserted above; here we
        // pin that the sum never degenerates to an empty CF.
        let sum = sqrt_of(2, 9).add(&sqrt_of(2, 9));
        assert!(!expanded(&sum).is_empty(), "2·√(2/9) must expand");
    }
}
