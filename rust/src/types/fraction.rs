use num_bigint::BigInt;
use num_traits::{Zero, One, ToPrimitive, Signed};
use num_integer::Integer;
use std::str::FromStr;

#[inline]
fn compute_gcd_i64(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

#[inline]
fn create_bigint_from_i128(n: i128) -> BigInt {
    if n >= i64::MIN as i128 && n <= i64::MAX as i128 {
        BigInt::from(n as i64)
    } else {
        let sign = n.signum();
        let abs_n = n.unsigned_abs();
        let high = (abs_n >> 64) as u64;
        let low = abs_n as u64;
        let result = if high == 0 {
            BigInt::from(low)
        } else {
            BigInt::from(high) * BigInt::from(1u128 << 64) + BigInt::from(low)
        };
        if sign < 0 { -result } else { result }
    }
}

// ---------------------------------------------------------------------------
// FractionRepr: Small Value Optimization
// ---------------------------------------------------------------------------
// Small(num, den) stores both parts inline on the stack (no heap allocation).
// Invariant: den >= 0. den == 0 represents NIL. When den > 0, reduced form.
// Big is the fallback for values that overflow i64.

#[derive(Debug, Clone)]
enum FractionRepr {
    Small(i64, i64),
    Big { numerator: BigInt, denominator: BigInt },
}

/// Fraction with Small Value Optimization.
///
/// Arithmetic hot paths are heap-independent when both operands fit in i64.
/// The internal representation uses `Small(i64, i64)` for common cases and
/// falls back to `Big { BigInt, BigInt }` only when values overflow.
#[derive(Debug, Clone)]
pub struct Fraction {
    repr: FractionRepr,
}

/// Mathematical equality via cross-multiplication.
/// This correctly handles unreduced fractions: 440/2 == 220/1.
impl PartialEq for Fraction {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.is_nil() || other.is_nil() {
            return self.is_nil() && other.is_nil();
        }
        match (&self.repr, &other.repr) {
            (FractionRepr::Small(a, b), FractionRepr::Small(c, d)) => {
                if b == d { return a == c; }
                (*a as i128) * (*d as i128) == (*c as i128) * (*b as i128)
            }
            _ => {
                let (an, ad) = self.to_bigint_pair();
                let (bn, bd) = other.to_bigint_pair();
                if ad == bd { return an == bn; }
                an * &bd == bn * &ad
            }
        }
    }
}

impl Eq for Fraction {}

impl Fraction {
    // ── Constructors ─────────────────────────────────────────────────

    /// NIL sentinel: denominator = 0. Propagates through arithmetic (three-valued logic).
    #[inline]
    pub fn nil() -> Self {
        Fraction { repr: FractionRepr::Small(0, 0) }
    }

    #[inline]
    pub fn is_nil(&self) -> bool {
        match &self.repr {
            FractionRepr::Small(_, d) => *d == 0,
            FractionRepr::Big { denominator, .. } => denominator.is_zero(),
        }
    }

    pub fn new(numerator: BigInt, denominator: BigInt) -> Self {
        if denominator.is_zero() { panic!("Division by zero"); }

        if numerator.is_zero() {
            return Fraction { repr: FractionRepr::Small(0, 1) };
        }

        if let (Some(n), Some(d)) = (numerator.to_i64(), denominator.to_i64()) {
            let g = compute_gcd_i64(n, d);
            let mut num = n / g;
            let mut den = d / g;
            if den < 0 {
                num = -num;
                den = -den;
            }
            return Fraction { repr: FractionRepr::Small(num, den) };
        }

        let common = numerator.gcd(&denominator);
        let mut num = &numerator / &common;
        let mut den = &denominator / &common;
        if den < BigInt::zero() {
            num = -num;
            den = -den;
        }
        Self::from_bigint_pair(num, den)
    }

    /// Constructs a fraction without GCD reduction. Only normalizes sign.
    /// Used for music DSL where n/d represents frequency/duration as independent parameters.
    #[inline]
    pub fn create_unreduced(mut numerator: BigInt, mut denominator: BigInt) -> Self {
        if denominator.is_zero() { panic!("Division by zero"); }
        if denominator < BigInt::zero() {
            numerator = -numerator;
            denominator = -denominator;
        }
        Self::from_bigint_pair(numerator, denominator)
    }

    /// Constructs a fraction that is already in lowest terms. Only normalizes sign.
    #[inline]
    fn create_already_reduced(mut numerator: BigInt, mut denominator: BigInt) -> Self {
        debug_assert!(!denominator.is_zero());
        if denominator < BigInt::zero() {
            numerator = -numerator;
            denominator = -denominator;
        }
        Self::from_bigint_pair(numerator, denominator)
    }

    /// Internal: create from BigInt pair, using Small when both fit in i64.
    #[inline]
    fn from_bigint_pair(numerator: BigInt, denominator: BigInt) -> Self {
        if let (Some(n), Some(d)) = (numerator.to_i64(), denominator.to_i64()) {
            Fraction { repr: FractionRepr::Small(n, d) }
        } else {
            Fraction { repr: FractionRepr::Big { numerator, denominator } }
        }
    }

    // ── Accessor methods (for external callers needing BigInt) ────────

    /// Returns the numerator as BigInt. For Small repr, constructs on the fly.
    #[inline]
    pub fn numerator(&self) -> BigInt {
        match &self.repr {
            FractionRepr::Small(n, _) => BigInt::from(*n),
            FractionRepr::Big { numerator, .. } => numerator.clone(),
        }
    }

    /// Returns the denominator as BigInt. For Small repr, constructs on the fly.
    #[inline]
    pub fn denominator(&self) -> BigInt {
        match &self.repr {
            FractionRepr::Small(_, d) => BigInt::from(*d),
            FractionRepr::Big { denominator, .. } => denominator.clone(),
        }
    }

    /// Returns (numerator, denominator) as owned BigInt pair.
    #[inline]
    pub fn to_bigint_pair(&self) -> (BigInt, BigInt) {
        match &self.repr {
            FractionRepr::Small(n, d) => (BigInt::from(*n), BigInt::from(*d)),
            FractionRepr::Big { numerator, denominator } => {
                (numerator.clone(), denominator.clone())
            }
        }
    }

    // ── Predicates ───────────────────────────────────────────────────

    #[inline]
    pub fn is_integer(&self) -> bool {
        match &self.repr {
            FractionRepr::Small(_, d) => *d == 1,
            FractionRepr::Big { denominator, .. } => denominator.is_one(),
        }
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        match &self.repr {
            FractionRepr::Small(n, _) => *n == 0,
            FractionRepr::Big { numerator, .. } => numerator.is_zero(),
        }
    }

    #[inline]
    pub fn is_exact_integer(&self) -> bool {
        self.is_integer()
    }

    // ── Fast-path extraction ─────────────────────────────────────────

    #[inline]
    fn extract_i64_pair(&self) -> Option<(i64, i64)> {
        match &self.repr {
            FractionRepr::Small(n, d) => Some((*n, *d)),
            FractionRepr::Big { numerator, denominator } => {
                let n = numerator.to_i64()?;
                let d = denominator.to_i64()?;
                Some((n, d))
            }
        }
    }

    #[inline]
    pub fn to_i64(&self) -> Option<i64> {
        match &self.repr {
            FractionRepr::Small(n, d) => {
                if *d == 1 { Some(*n) } else { None }
            }
            FractionRepr::Big { numerator, denominator } => {
                if denominator.is_one() {
                    numerator.to_i64()
                } else {
                    None
                }
            }
        }
    }

    #[inline]
    pub fn as_usize(&self) -> Option<usize> {
        match &self.repr {
            FractionRepr::Small(n, d) => {
                if *d == 1 && *n >= 0 {
                    Some(*n as usize)
                } else {
                    None
                }
            }
            FractionRepr::Big { numerator, denominator } => {
                if denominator.is_one() && *numerator >= BigInt::zero() {
                    numerator.to_usize()
                } else {
                    None
                }
            }
        }
    }

    // ── i128 construction (used by arithmetic fast paths) ────────────

    #[inline]
    fn create_from_i128(num: i128, den: i128) -> Self {
        debug_assert!(den != 0);
        fn compute_gcd_i128(mut a: i128, mut b: i128) -> i128 {
            a = a.abs();
            b = b.abs();
            while b != 0 {
                let t = b;
                b = a % b;
                a = t;
            }
            a
        }
        let g = compute_gcd_i128(num, den);
        let mut n = num / g;
        let mut d = den / g;
        if d < 0 {
            n = -n;
            d = -d;
        }
        // SVO: store as Small when result fits in i64
        if n >= i64::MIN as i128 && n <= i64::MAX as i128
            && d >= 0 && d <= i64::MAX as i128
        {
            return Fraction { repr: FractionRepr::Small(n as i64, d as i64) };
        }
        Fraction {
            repr: FractionRepr::Big {
                numerator: create_bigint_from_i128(n),
                denominator: create_bigint_from_i128(d),
            },
        }
    }

    // ── Multiplication/Division by integer helpers ───────────────────

    /// (a/b) × n: Cross-cancellation with gcd(n, b).
    #[inline]
    pub fn mul_by_integer(&self, n: &Fraction) -> Fraction {
        debug_assert!(n.is_integer());

        if let (Some((a, b)), Some((n_val, _))) = (self.extract_i64_pair(), n.extract_i64_pair()) {
            let g = compute_gcd_i64(n_val, b);
            let n_r = (n_val / g) as i128;
            let b_r = (b / g) as i128;
            let num = (a as i128) * n_r;
            return Self::create_from_i128(num, b_r);
        }

        let (sn, sd) = self.to_bigint_pair();
        let nn = n.numerator();
        let g = nn.gcd(&sd);
        let n_reduced = &nn / &g;
        let b_reduced = &sd / &g;
        Self::create_already_reduced(sn * n_reduced, b_reduced)
    }

    /// (a/b) ÷ n: Cross-cancellation with gcd(a, n).
    #[inline]
    pub fn div_by_integer(&self, n: &Fraction) -> Fraction {
        debug_assert!(n.is_integer());
        debug_assert!(!n.is_zero());

        if let (Some((a, b)), Some((n_val, _))) = (self.extract_i64_pair(), n.extract_i64_pair()) {
            let g = compute_gcd_i64(a, n_val);
            let a_r = (a / g) as i128;
            let n_r = (n_val / g) as i128;
            let den = (b as i128) * n_r;
            return Self::create_from_i128(a_r, den);
        }

        let (sn, sd) = self.to_bigint_pair();
        let nn = n.numerator();
        let g = sn.gcd(&nn);
        let a_reduced = &sn / &g;
        let n_reduced = &nn / &g;
        Self::create_already_reduced(a_reduced, sd * n_reduced)
    }

    // ── String parsing ───────────────────────────────────────────────

    pub fn from_str(s: &str) -> std::result::Result<Self, String> {
        if s.is_empty() { return Err("Empty string".to_string()); }

        if let Some(e_pos) = s.find(|c| char::is_ascii(&c) && (c == 'e' || c == 'E')) {
            let mantissa_str = &s[..e_pos];
            let exponent_str = &s[e_pos+1..];

            let mantissa = Self::from_str(mantissa_str)?;
            let exponent = exponent_str.parse::<i32>().map_err(|e| e.to_string())?;

            let (mn, md) = mantissa.to_bigint_pair();
            if exponent >= 0 {
                let power = BigInt::from(10).pow(exponent as u32);
                return Ok(Fraction::new(mn * power, md));
            } else {
                let power = BigInt::from(10).pow((-exponent) as u32);
                return Ok(Fraction::new(mn, md * power));
            }
        }
        if let Some(pos) = s.find('/') {
            let num = BigInt::from_str(&s[..pos]).map_err(|e| e.to_string())?;
            let den = BigInt::from_str(&s[pos+1..]).map_err(|e| e.to_string())?;
            Ok(Fraction::new(num, den))
        } else if let Some(dot_pos) = s.find('.') {
            let int_part_str = if s.starts_with('.') { "0" } else { &s[..dot_pos] };
            let frac_part_str = &s[dot_pos+1..];
            if frac_part_str.is_empty() { return Self::from_str(int_part_str); }
            let int_part = BigInt::from_str(int_part_str).map_err(|e| e.to_string())?;
            let frac_num = BigInt::from_str(frac_part_str).map_err(|e| e.to_string())?;
            let frac_den = BigInt::from(10).pow(frac_part_str.len() as u32);
            let total_num = int_part.abs() * &frac_den + frac_num;
            Ok(Fraction::new(if int_part < BigInt::zero() { -total_num } else { total_num }, frac_den))
        } else {
            let num = BigInt::from_str(s).map_err(|e| e.to_string())?;
            // Integer / 1 is already in lowest terms — skip GCD
            if let Some(n) = num.to_i64() {
                Ok(Fraction { repr: FractionRepr::Small(n, 1) })
            } else {
                Ok(Fraction { repr: FractionRepr::Big { numerator: num, denominator: BigInt::one() } })
            }
        }
    }

    /// Parses a fraction string without GCD reduction for explicit a/b forms.
    /// Integers and decimals are still reduced (they represent single mathematical values).
    /// Used for vector construction where a/b may represent frequency/duration.
    pub fn parse_unreduced_from_str(s: &str) -> std::result::Result<Self, String> {
        if s.is_empty() { return Err("Empty string".to_string()); }

        // Scientific notation: delegate to from_str (reduction is appropriate)
        if s.contains(|c: char| c == 'e' || c == 'E') {
            return Self::from_str(s);
        }

        // Explicit fraction a/b: preserve original numerator and denominator
        if let Some(pos) = s.find('/') {
            let num = BigInt::from_str(&s[..pos]).map_err(|e| e.to_string())?;
            let den = BigInt::from_str(&s[pos+1..]).map_err(|e| e.to_string())?;
            Ok(Self::create_unreduced(num, den))
        } else {
            // Integer or decimal: regular parsing (reduction is fine for single values)
            Self::from_str(s)
        }
    }

    // ── Arithmetic ───────────────────────────────────────────────────

    /// (a/b) + (c/d) with same-denominator fast path and cross-cancellation for shared factors.
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
            if let Some(num) = (a as i128).checked_mul(d as i128)
                .and_then(|ad| (c as i128).checked_mul(b as i128)
                    .and_then(|cb| ad.checked_add(cb)))
            {
                return Self::create_from_i128(num, (b as i128) * (d as i128));
            }
        }

        let (an, ad) = self.to_bigint_pair();
        let (bn, bd) = other.to_bigint_pair();

        if ad == bd {
            let sum = &an + &bn;
            if sum.is_zero() {
                return Fraction { repr: FractionRepr::Small(0, 1) };
            }
            let g = sum.gcd(&ad);
            if g.is_one() {
                return Self::create_already_reduced(sum, ad);
            }
            return Self::create_already_reduced(&sum / &g, &ad / &g);
        }

        Fraction::new(
            &an * &bd + &bn * &ad,
            &ad * &bd,
        )
    }

    /// (a/b) - (c/d) with same-denominator fast path.
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
            if let Some(num) = (a as i128).checked_mul(d as i128)
                .and_then(|ad| (c as i128).checked_mul(b as i128)
                    .and_then(|cb| ad.checked_sub(cb)))
            {
                return Self::create_from_i128(num, (b as i128) * (d as i128));
            }
        }

        let (an, ad) = self.to_bigint_pair();
        let (bn, bd) = other.to_bigint_pair();

        if ad == bd {
            let diff = &an - &bn;
            if diff.is_zero() {
                return Fraction { repr: FractionRepr::Small(0, 1) };
            }
            let g = diff.gcd(&ad);
            if g.is_one() {
                return Self::create_already_reduced(diff, ad);
            }
            return Self::create_already_reduced(&diff / &g, &ad / &g);
        }

        Fraction::new(
            &an * &bd - &bn * &ad,
            &ad * &bd,
        )
    }

    /// (a/b) × (c/d): Cross-cancellation with g1 = gcd(a,d), g2 = gcd(c,b).
    /// Result = (a/g1 × c/g2) / (b/g2 × d/g1), already in lowest terms.
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

        let (an, ad) = self.to_bigint_pair();
        let (bn, bd) = other.to_bigint_pair();

        if ad.is_one() && bd.is_one() {
            return Self::from_bigint_pair(an * bn, BigInt::one());
        }

        if ad.is_one() {
            let g = an.gcd(&bd);
            let a_reduced = &an / &g;
            let d_reduced = &bd / &g;
            return Self::create_already_reduced(a_reduced * bn, d_reduced);
        }

        if bd.is_one() {
            let g = bn.gcd(&ad);
            let c_reduced = &bn / &g;
            let b_reduced = &ad / &g;
            return Self::create_already_reduced(an * c_reduced, b_reduced);
        }

        let g1 = an.gcd(&bd);
        let g2 = bn.gcd(&ad);

        let a_reduced = &an / &g1;
        let d_reduced = &bd / &g1;
        let c_reduced = &bn / &g2;
        let b_reduced = &ad / &g2;

        Self::create_already_reduced(
            a_reduced * c_reduced,
            b_reduced * d_reduced,
        )
    }

    /// (a/b) ÷ (c/d) = (a/b) × (d/c): Cross-cancellation with g1 = gcd(a,c), g2 = gcd(d,b).
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

        let (an, ad) = self.to_bigint_pair();
        let (bn, bd) = other.to_bigint_pair();

        if ad.is_one() && bd.is_one() {
            return Fraction::new(an, bn);
        }

        if ad.is_one() {
            let g = an.gcd(&bn);
            let a_reduced = &an / &g;
            let c_reduced = &bn / &g;
            return Self::create_already_reduced(a_reduced * bd, c_reduced);
        }

        if bd.is_one() {
            let g = an.gcd(&bn);
            let a_reduced = &an / &g;
            let c_reduced = &bn / &g;
            return Self::create_already_reduced(a_reduced, ad * c_reduced);
        }

        let g1 = an.gcd(&bn);
        let g2 = bd.gcd(&ad);

        let a_reduced = &an / &g1;
        let c_reduced = &bn / &g1;
        let d_reduced = &bd / &g2;
        let b_reduced = &ad / &g2;

        Self::create_already_reduced(
            a_reduced * d_reduced,
            b_reduced * c_reduced,
        )
    }

    // ── Comparison helpers ───────────────────────────────────────────

    /// a/b < c/d — delegates to Ord::cmp for unified fast path
    #[inline]
    pub fn lt(&self, other: &Fraction) -> bool {
        self.cmp(other) == std::cmp::Ordering::Less
    }

    #[inline]
    pub fn le(&self, other: &Fraction) -> bool {
        self.cmp(other) != std::cmp::Ordering::Greater
    }

    #[inline]
    pub fn gt(&self, other: &Fraction) -> bool {
        self.cmp(other) == std::cmp::Ordering::Greater
    }

    #[inline]
    pub fn ge(&self, other: &Fraction) -> bool {
        self.cmp(other) != std::cmp::Ordering::Less
    }

    // ── Unary operations ─────────────────────────────────────────────

    /// Absolute value: |a/b|
    #[inline]
    pub fn abs(&self) -> Fraction {
        if self.is_nil() {
            return self.clone();
        }
        match &self.repr {
            FractionRepr::Small(n, d) => {
                Fraction { repr: FractionRepr::Small(n.abs(), *d) }
            }
            FractionRepr::Big { numerator, denominator } => {
                Fraction {
                    repr: FractionRepr::Big {
                        numerator: if *numerator < BigInt::zero() {
                            -numerator.clone()
                        } else {
                            numerator.clone()
                        },
                        denominator: denominator.clone(),
                    },
                }
            }
        }
    }

    /// Floor: rounds toward negative infinity.
    pub fn floor(&self) -> Fraction {
        if self.is_integer() {
            return self.clone();
        }

        match &self.repr {
            FractionRepr::Small(n, d) => {
                let q = n / d;
                let r = n % d;
                let floored = if *n < 0 && r != 0 { q - 1 } else { q };
                Fraction { repr: FractionRepr::Small(floored, 1) }
            }
            FractionRepr::Big { numerator, denominator } => {
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

    /// Ceil: rounds toward positive infinity.
    pub fn ceil(&self) -> Fraction {
        if self.is_integer() {
            return self.clone();
        }

        match &self.repr {
            FractionRepr::Small(n, d) => {
                let q = n / d;
                let r = n % d;
                let ceiled = if *n > 0 && r != 0 { q + 1 } else { q };
                Fraction { repr: FractionRepr::Small(ceiled, 1) }
            }
            FractionRepr::Big { numerator, denominator } => {
                let q = numerator / denominator;
                let r = numerator % denominator;
                let ceiled = if *numerator > BigInt::zero() && !r.is_zero() {
                    q + BigInt::one()
                } else {
                    q
                };
                Self::from_bigint_pair(ceiled, BigInt::one())
            }
        }
    }

    /// Round half away from zero: floor(|x| + 0.5) with original sign.
    /// Formula: floor((2*|num| + den) / (2*den))
    pub fn round(&self) -> Fraction {
        if self.is_integer() {
            return self.clone();
        }

        if self.is_zero() {
            return Fraction { repr: FractionRepr::Small(0, 1) };
        }

        match &self.repr {
            FractionRepr::Small(n, d) => {
                let is_negative = *n < 0;
                let abs_n = n.abs() as i128;
                let d128 = *d as i128;
                let result = ((2 * abs_n + d128) / (2 * d128)) as i64;
                Fraction { repr: FractionRepr::Small(if is_negative { -result } else { result }, 1) }
            }
            FractionRepr::Big { numerator, denominator } => {
                let is_negative = *numerator < BigInt::zero();
                let abs_num = if is_negative {
                    -numerator.clone()
                } else {
                    numerator.clone()
                };
                let two = BigInt::from(2);
                let two_abs_num = &abs_num * &two;
                let result = (&two_abs_num + denominator) / (&two * denominator);
                Self::from_bigint_pair(
                    if is_negative { -result } else { result },
                    BigInt::one(),
                )
            }
        }
    }

    // ── Modulo ───────────────────────────────────────────────────────

    /// Mathematical modulo: a mod b = a - b * floor(a/b).
    /// Result has the same sign as b.
    pub fn modulo(&self, other: &Fraction) -> Fraction {
        if other.is_zero() {
            panic!("Modulo by zero");
        }

        // i64 fast path for integer modulo
        if let (Some((a, b)), Some((c, d))) = (self.extract_i64_pair(), other.extract_i64_pair()) {
            if b == 1 && d == 1 {
                let rem = a % c;
                let result = if rem < 0 {
                    if c > 0 { rem + c } else { rem - c }
                } else {
                    rem
                };
                return Fraction { repr: FractionRepr::Small(result, 1) };
            }

            // i64 fast path for fractional modulo: a/b mod c/d = (a*d mod c*b) / (b*d)
            let a = a as i128;
            let b = b as i128;
            let c = c as i128;
            let d = d as i128;
            let num = a * d;
            let mod_by = c * b;
            let den = b * d;
            let rem = num % mod_by;
            let result_num = if rem < 0 {
                if mod_by > 0 { rem + mod_by } else { rem - mod_by }
            } else {
                rem
            };
            return Self::create_from_i128(result_num, den);
        }

        // BigInt fallback for integer modulo
        let (sn, sd) = self.to_bigint_pair();
        let (on, od) = other.to_bigint_pair();

        if sd.is_one() && od.is_one() {
            let rem = &sn % &on;
            let result = if rem < BigInt::zero() {
                if on > BigInt::zero() {
                    rem + &on
                } else {
                    rem - &on
                }
            } else {
                rem
            };
            return Self::from_bigint_pair(result, BigInt::one());
        }

        let div_result = self.div(other);
        let floored = div_result.floor();
        self.sub(&other.mul(&floored))
    }
}

// ── Trait implementations ────────────────────────────────────────────

impl PartialOrd for Fraction {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fraction {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if let (Some((a, b)), Some((c, d))) = (self.extract_i64_pair(), other.extract_i64_pair()) {
            if b == d {
                return a.cmp(&c);
            }
            let lhs = (a as i128) * (d as i128);
            let rhs = (c as i128) * (b as i128);
            return lhs.cmp(&rhs);
        }
        let (an, ad) = self.to_bigint_pair();
        let (bn, bd) = other.to_bigint_pair();
        if ad == bd {
            return an.cmp(&bn);
        }
        let lhs = an * &bd;
        let rhs = bn * &ad;
        lhs.cmp(&rhs)
    }
}

impl ToPrimitive for Fraction {
    fn to_i64(&self) -> Option<i64> {
        match &self.repr {
            FractionRepr::Small(n, d) => {
                if *d == 0 { return None; }
                Some(n / d)
            }
            FractionRepr::Big { numerator, denominator } => {
                (numerator / denominator).to_i64()
            }
        }
    }

    fn to_u64(&self) -> Option<u64> {
        match &self.repr {
            FractionRepr::Small(n, d) => {
                if *d == 0 || *n < 0 { return None; }
                Some((*n / *d) as u64)
            }
            FractionRepr::Big { numerator, denominator } => {
                if *numerator < BigInt::zero() {
                    None
                } else {
                    (numerator / denominator).to_u64()
                }
            }
        }
    }

    fn to_f64(&self) -> Option<f64> {
        match &self.repr {
            FractionRepr::Small(n, d) => {
                if *d == 0 { return None; }
                Some(*n as f64 / *d as f64)
            }
            FractionRepr::Big { numerator, denominator } => {
                let num_f64 = numerator.to_f64()?;
                let den_f64 = denominator.to_f64()?;
                if den_f64 == 0.0 { None } else { Some(num_f64 / den_f64) }
            }
        }
    }
}

impl From<i64> for Fraction {
    #[inline]
    fn from(n: i64) -> Self {
        Fraction { repr: FractionRepr::Small(n, 1) }
    }
}

impl From<i32> for Fraction {
    #[inline]
    fn from(n: i32) -> Self {
        Fraction { repr: FractionRepr::Small(n as i64, 1) }
    }
}

impl std::fmt::Display for Fraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.repr {
            FractionRepr::Small(n, d) => {
                if *d == 1 { write!(f, "{}", n) }
                else { write!(f, "{}/{}", n, d) }
            }
            FractionRepr::Big { numerator, denominator } => {
                if denominator.is_one() {
                    write!(f, "{}", numerator)
                } else {
                    write!(f, "{}/{}", numerator, denominator)
                }
            }
        }
    }
}
