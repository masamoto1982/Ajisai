use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

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
    AlgebraicSqrt { radicand: Fraction },
    /// Unevaluated Gosper transform of one or two operand CFs per
    /// SPEC §4.2.2. Constructed only by the arithmetic methods, which
    /// fold pure-rational operands through the `Fraction` fast path
    /// instead of building a Gosper node.
    Gosper(Box<Gosper>),
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
            (None, None) => bihom_apply_add(self.clone(), other.clone()),
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
            (None, None) => bihom_apply_sub(self.clone(), other.clone()),
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
            (None, None) => bihom_apply_mul(self.clone(), other.clone()),
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
            (None, None) => bihom_apply_div(self.clone(), other.clone()),
        })
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
    ExactReal::Gosper(Box::new(Gosper::Mobius { a, b, c, d, x }))
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
    ExactReal::Gosper(Box::new(Gosper::Bihomographic {
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
    /// Stream exhausted; further `next_quotient` calls return `None`.
    Empty,
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
        loop {
            match &mut self.state {
                CfState::Empty => return None,
                CfState::Finite { canonical, pos } => {
                    if *pos < canonical.len() {
                        let q = canonical[*pos].clone();
                        *pos += 1;
                        return Some(q);
                    }
                    self.state = CfState::Empty;
                    return None;
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
                    return Some(a_i);
                }
                CfState::Mobius { .. } => {
                    if let Some(q) = step_mobius(&mut self.state) {
                        return Some(q);
                    }
                    // step_mobius transitioned the state (either to
                    // Finite for the rational-tail case, or returned
                    // None to signal exhaustion). Re-dispatch.
                }
                CfState::Bihom { .. } => {
                    if let Some(q) = step_bihom(&mut self.state) {
                        return Some(q);
                    }
                }
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
    let CfState::Mobius { a, b, c, d, x, x_done } = state else {
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
            // the value is infinite — no further output is possible.
            if c.is_zero() {
                *state = CfState::Empty;
                return None;
            }
            let canonical = rational_partial_quotients(a.clone(), c.clone());
            *state = CfState::Finite {
                canonical,
                pos: 0,
            };
            return None;
        }

        if ingest_budget == 0 {
            *state = CfState::Empty;
            return None;
        }
        ingest_budget -= 1;

        match x.next_quotient() {
            Some(p) => {
                // Ingest p: (a, b, c, d) ← (a·p + b, a, c·p + d, c)
                let new_a = &*a * &p + &*b;
                let new_b = a.clone();
                let new_c = &*c * &p + &*d;
                let new_d = c.clone();
                *a = new_a;
                *b = new_b;
                *c = new_c;
                *d = new_d;
            }
            None => {
                *x_done = true;
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
                *state = CfState::Empty;
                return None;
            }
            let canonical = rational_partial_quotients(a.clone(), e.clone());
            *state = CfState::Finite {
                canonical,
                pos: 0,
            };
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
            *state = CfState::Empty;
            return None;
        }
        ingest_budget -= 1;

        // Choose which operand to ingest: prefer the axis whose
        // corner spread is wider when both axes have well-defined
        // (finite-denominator) projections; otherwise fall back to
        // balanced consumption so an axis whose corners are all
        // initially zero-denominator (e.g. y in `x + y`) still gets
        // its first ingestion before the safety budget expires.
        let ingest_x = pick_ingest_axis(
            a,
            b,
            c,
            d,
            e,
            f,
            g,
            h,
            *x_consumed,
            *y_consumed,
        );
        if ingest_x {
            match x.next_quotient() {
                Some(p) => {
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
                }
                None => {
                    *x_done = true;
                }
            }
        } else {
            match y.next_quotient() {
                Some(p) => {
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
                }
                None => {
                    *y_done = true;
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

    // === Phase 4a baseline (sqrt) ===

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
        assert!(
            sqrt_of(2, 1)
                .div(&nil)
                .expect("nil-divisor yields nil, not None")
                .is_nil()
        );
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
    // all produce mathematically-exact integer results from two lazy
    // operands. The bihomographic Gosper algorithm can never *prove*
    // such an identity from finite operand prefixes — the four-corner
    // floor test always straddles the answer because each new
    // partial-quotient ingestion is consistent with both "the result
    // is exactly the integer" and "the result is the integer ±ε".
    // SPEC §7.4.1 calls out this exact case ("two equal irrationals
    // never differ — the procedure does not terminate by itself")
    // and projects it to NIL with `absence.reason = undecidable`
    // under the language-level comparison budget. That budget and
    // the surrounding Bubble Rule wiring are Phase 5 work; for
    // Phase 4b we only assert that the bihom result is well-formed
    // and yields a bounded partial-quotient prefix without panicking.

    #[test]
    fn sqrt_two_minus_sqrt_two_is_lazy_within_budget() {
        let value = sqrt_of(2, 1).sub(&sqrt_of(2, 1));
        assert!(value.is_gosper());
        // The bihom safety budget caps internal ingestion; with no
        // emit reachable, the bounded prefix is empty.
        assert!(value.partial_quotients_bounded(4).is_empty());
    }

    #[test]
    fn sqrt_two_times_sqrt_two_is_lazy_within_budget() {
        let value = sqrt_of(2, 1).mul(&sqrt_of(2, 1));
        assert!(value.is_gosper());
        assert!(value.partial_quotients_bounded(4).is_empty());
    }

    #[test]
    fn sqrt_two_divided_by_sqrt_two_is_lazy_within_budget() {
        let value = sqrt_of(2, 1)
            .div(&sqrt_of(2, 1))
            .expect("sqrt(2) is nonzero");
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
        let value = sqrt_of(2, 1)
            .add(&rational(1, 1))
            .sub(&rational(1, 1));
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

        let quo = rational(7, 3)
            .div(&rational(7, 3))
            .expect("nonzero");
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
        // √2 − √2 is mathematically zero but lazy ExactReal cannot
        // prove it without expansion: report false (conservative).
        let lazy_zero = sqrt_of(2, 1).sub(&sqrt_of(2, 1));
        assert!(!lazy_zero.is_structurally_zero());
    }
}
