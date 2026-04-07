use num_bigint::BigInt;
use num_traits::{Zero, One, ToPrimitive, Signed};
use num_integer::Integer;
use std::str::FromStr;

#[inline]
pub(crate) fn compute_gcd_i64(mut a: i64, mut b: i64) -> i64 {
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
pub(crate) fn create_bigint_from_i128(n: i128) -> BigInt {
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

// SVO: Small(num, den) stores both parts inline on the stack (no heap allocation).
// Invariant: den >= 0. den == 0 represents NIL. When den > 0, reduced form.
// Big is the fallback for values that overflow i64.
#[derive(Debug, Clone)]
pub(crate) enum FractionRepr {
    Small(i64, i64),
    Big { numerator: BigInt, denominator: BigInt },
}

#[derive(Debug, Clone)]
pub struct Fraction {
    pub(crate) repr: FractionRepr,
}

impl Fraction {
    #[inline]
    pub(crate) fn from_repr(repr: FractionRepr) -> Self {
        Fraction { repr }
    }
}

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
            (FractionRepr::Small(..), FractionRepr::Big { .. })
            | (FractionRepr::Big { .. }, FractionRepr::Small(..))
            | (FractionRepr::Big { .. }, FractionRepr::Big { .. }) => {
                let (an, ad): (BigInt, BigInt) = self.to_bigint_pair();
                let (bn, bd): (BigInt, BigInt) = other.to_bigint_pair();
                if ad == bd { return an == bn; }
                an * &bd == bn * &ad
            }
        }
    }
}

impl Eq for Fraction {}

impl Fraction {
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

    /// Returns `true` if this fraction is stored in the stack-allocated Small
    /// representation (`FractionRepr::Small(i64, i64)`), meaning its numerator
    /// and denominator both fit in i64 and no heap memory is required.
    ///
    /// Callers can use this to select heap-free hot paths in arithmetic and
    /// tensor operations when all operands are known to be Small.
    #[inline]
    pub fn is_small(&self) -> bool {
        matches!(self.repr, FractionRepr::Small(..))
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

        let common: BigInt = numerator.gcd(&denominator);
        let mut num: BigInt = &numerator / &common;
        let mut den: BigInt = &denominator / &common;
        if den < BigInt::zero() {
            num = -num;
            den = -den;
        }
        Self::from_bigint_pair(num, den)
    }

    #[inline]
    pub fn create_unreduced(mut numerator: BigInt, mut denominator: BigInt) -> Self {
        if denominator.is_zero() { panic!("Division by zero"); }
        if denominator < BigInt::zero() {
            numerator = -numerator;
            denominator = -denominator;
        }
        Self::from_bigint_pair(numerator, denominator)
    }

    #[inline]
    pub(crate) fn create_already_reduced(mut numerator: BigInt, mut denominator: BigInt) -> Self {
        debug_assert!(!denominator.is_zero());
        if denominator < BigInt::zero() {
            numerator = -numerator;
            denominator = -denominator;
        }
        Self::from_bigint_pair(numerator, denominator)
    }

    #[inline]
    pub(crate) fn from_bigint_pair(numerator: BigInt, denominator: BigInt) -> Self {
        if let (Some(n), Some(d)) = (numerator.to_i64(), denominator.to_i64()) {
            return Fraction { repr: FractionRepr::Small(n, d) };
        }
        Fraction { repr: FractionRepr::Big { numerator, denominator } }
    }

    #[inline]
    pub fn numerator(&self) -> BigInt {
        match &self.repr {
            FractionRepr::Small(n, _) => BigInt::from(*n),
            FractionRepr::Big { numerator, .. } => numerator.clone(),
        }
    }

    #[inline]
    pub fn denominator(&self) -> BigInt {
        match &self.repr {
            FractionRepr::Small(_, d) => BigInt::from(*d),
            FractionRepr::Big { denominator, .. } => denominator.clone(),
        }
    }

    #[inline]
    pub fn to_bigint_pair(&self) -> (BigInt, BigInt) {
        match &self.repr {
            FractionRepr::Small(n, d) => (BigInt::from(*n), BigInt::from(*d)),
            FractionRepr::Big { numerator, denominator } => {
                (numerator.clone(), denominator.clone())
            }
        }
    }

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

    #[inline]
    pub(crate) fn extract_i64_pair(&self) -> Option<(i64, i64)> {
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
                if !denominator.is_one() { return None; }
                numerator.to_i64()
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
                if !denominator.is_one() || *numerator < BigInt::zero() { return None; }
                numerator.to_usize()
            }
        }
    }

    #[inline]
    pub(crate) fn create_from_i128(num: i128, den: i128) -> Self {
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
        let g: i128 = compute_gcd_i128(num, den);
        let mut n: i128 = num / g;
        let mut d: i128 = den / g;
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

    pub fn from_str(s: &str) -> std::result::Result<Self, String> {
        if s.is_empty() { return Err("Empty string".to_string()); }

        if let Some(e_pos) = s.find(|c| char::is_ascii(&c) && (c == 'e' || c == 'E')) {
            let mantissa_str = &s[..e_pos];
            let exponent_str = &s[e_pos+1..];

            let mantissa: Fraction = Self::from_str(mantissa_str)?;
            let exponent: i32 = exponent_str.parse::<i32>().map_err(|e| e.to_string())?;

            let (mn, md): (BigInt, BigInt) = mantissa.to_bigint_pair();
            if exponent >= 0 {
                let power: BigInt = BigInt::from(10).pow(exponent as u32);
                return Ok(Fraction::new(mn * power, md));
            } else {
                let power: BigInt = BigInt::from(10).pow((-exponent) as u32);
                return Ok(Fraction::new(mn, md * power));
            }
        }
        if let Some(pos) = s.find('/') {
            let num: BigInt = BigInt::from_str(&s[..pos]).map_err(|e| e.to_string())?;
            let den: BigInt = BigInt::from_str(&s[pos+1..]).map_err(|e| e.to_string())?;
            Ok(Fraction::new(num, den))
        } else if let Some(dot_pos) = s.find('.') {
            let int_part_str = if s.starts_with('.') { "0" } else { &s[..dot_pos] };
            let frac_part_str = &s[dot_pos+1..];
            if frac_part_str.is_empty() { return Self::from_str(int_part_str); }
            let int_part: BigInt = BigInt::from_str(int_part_str).map_err(|e| e.to_string())?;
            let frac_num: BigInt = BigInt::from_str(frac_part_str).map_err(|e| e.to_string())?;
            let frac_den: BigInt = BigInt::from(10).pow(frac_part_str.len() as u32);
            let total_num: BigInt = int_part.abs() * &frac_den + frac_num;
            Ok(Fraction::new(if int_part < BigInt::zero() { -total_num } else { total_num }, frac_den))
        } else {
            let num: BigInt = BigInt::from_str(s).map_err(|e| e.to_string())?;
            // Already in lowest terms -- skip GCD
            if let Some(n) = num.to_i64() {
                return Ok(Fraction { repr: FractionRepr::Small(n, 1) });
            }
            Ok(Fraction { repr: FractionRepr::Big { numerator: num, denominator: BigInt::one() } })
        }
    }

    // Skip GCD reduction for explicit a/b forms to preserve frequency/duration semantics
    pub fn parse_unreduced_from_str(s: &str) -> std::result::Result<Self, String> {
        if s.is_empty() { return Err("Empty string".to_string()); }

        if s.contains(|c: char| c == 'e' || c == 'E') {
            return Self::from_str(s);
        }

        if let Some(pos) = s.find('/') {
            let num: BigInt = BigInt::from_str(&s[..pos]).map_err(|e| e.to_string())?;
            let den: BigInt = BigInt::from_str(&s[pos+1..]).map_err(|e| e.to_string())?;
            return Ok(Self::create_unreduced(num, den));
        }

        Self::from_str(s)
    }

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
}

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
                if *numerator < BigInt::zero() { return None; }
                (numerator / denominator).to_u64()
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
                let num_f64: f64 = numerator.to_f64()?;
                let den_f64: f64 = denominator.to_f64()?;
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
