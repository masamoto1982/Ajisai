//! Element-wise Words over one-dimensional dense Tensors, computed on the
//! columns themselves.
//!
//! A dense Tensor stores its lanes as two `i64` columns — numerators and
//! denominators, each pair already in lowest terms. The general routes for
//! `DIV`, `FLOOR`/`ROUND` and `LT`/`GT` read those columns back out as a
//! `Vec<Fraction>` (`FlatTensor`) or as one boxed `Value` per lane
//! (`lift_lanes`), apply the law, and rebuild: about 200 ns a lane for what is
//! a handful of machine instructions. `ADD`/`SUB`/`MUL` had a column route
//! already (`simd_ops`), but through a function pointer it could not inline,
//! with an early exit that kept it from vectorising.
//!
//! These kernels take the columns as they are, for the shapes whose answer is
//! fixed by the lanes alone: a Tensor beside a plain scalar, or two Tensors of
//! one length. They answer exactly the `Value` the general route builds — the
//! same dense Tensor (`from_fractions` of the same lanes is `from_columns` of
//! the same columns), or for a comparison the same Vector of Booleans — and
//! decline (`None`) wherever that route could answer anything else: a lane
//! that no longer fits a machine word (a boxed Vector), any other shape. The
//! charges are untouched, because the dispatcher made them before choosing a
//! route (`charge_binary_schema`); which route ran is unobservable
//! (LANG.AUTHORITY.FREEDOM). `dense_kernels_tests` holds the two equal.
//!
//! A lane over zero — one of the three points `1/0`, `-1/0`, `0/0` — is a
//! lane like any other, in an operand or in the answer. The pair laws here
//! assume a positive denominator, so such a lane, and a zero divisor, are
//! answered by the `Fraction`'s own total arithmetic (`fraction_extended`)
//! and written back as the reduced pair it answers: `[ 6 6 ] [ 1 0 ] DIV`
//! holds `6/1` and `1/0`, and keeps its columns.

mod lanes;

use crate::interpreter::arithmetic::ExactArithmeticSchema;
use crate::interpreter::comparison::OrderingKind;
use crate::types::small_rational;
use crate::types::Value;
use lanes::{paired, Lanes, Out};

#[cfg(test)]
thread_local! {
    static KERNEL_HITS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Kernel answers on this thread, for tests that pin which route ran.
#[cfg(test)]
pub(crate) fn kernel_hits_on_this_thread() -> u64 {
    KERNEL_HITS.with(|c| c.get())
}

fn hit<T>(answer: Option<T>) -> Option<T> {
    #[cfg(test)]
    if answer.is_some() {
        KERNEL_HITS.with(|c| c.set(c.get() + 1));
    }
    answer
}

/// `out[i] = f(a[i], b[i])` on integer lanes, with the overflow flags of
/// every lane gathered rather than checked one by one, so the loop is free
/// to vectorise. A set flag declines the whole operation.
#[inline(always)]
fn integer_lanes(
    a: Lanes,
    b: Lanes,
    n: usize,
    f: impl Fn(i64, i64) -> (i64, bool),
) -> Option<Vec<i64>> {
    let mut out = vec![0i64; n];
    let mut bad = false;
    match (a, b) {
        (Lanes::Columns { nums: x, .. }, Lanes::Columns { nums: y, .. }) => {
            for ((o, &x), &y) in out.iter_mut().zip(x).zip(y) {
                let (v, f) = f(x, y);
                *o = v;
                bad |= f;
            }
        }
        (Lanes::Columns { nums: x, .. }, Lanes::Splat(y, _)) => {
            for (o, &x) in out.iter_mut().zip(x) {
                let (v, f) = f(x, y);
                *o = v;
                bad |= f;
            }
        }
        (Lanes::Splat(x, _), Lanes::Columns { nums: y, .. }) => {
            for (o, &y) in out.iter_mut().zip(y) {
                let (v, f) = f(x, y);
                *o = v;
                bad |= f;
            }
        }
        (Lanes::Splat(..), Lanes::Splat(..)) => return None,
    }
    (!bad).then_some(out)
}

/// The largest scalar divisor whose residue gcds `integer_quotients` tables:
/// a gcd of it fits a `u16`, and the table is a few kilobytes at most.
const GCD_TABLE_MAX: u64 = 4096;

/// `a / b` on integer lanes: each quotient reduced by one gcd, with the sign
/// carried by the numerator; a zero divisor answers the dividend's sign over
/// zero. `None` for the one quotient a machine word cannot hold,
/// `i64::MIN / -1`.
fn integer_quotients(a: Lanes, b: Lanes, n: usize) -> Option<Value> {
    use crate::types::fraction::binary_gcd_u64;
    // A Tensor divided by one small integer (`7 DIV`, the common case) meets
    // only `|y|` residues, so everything a lane needs is read from a table
    // built once.
    if let (Lanes::Columns { nums, .. }, Lanes::Splat(y, _)) = (a, b) {
        let m = y.unsigned_abs();
        if (1..=GCD_TABLE_MAX).contains(&m) && m as usize <= n {
            return quotients_by_small_integer(nums, y);
        }
    }
    let mut out = Out::with_capacity(n);
    for i in 0..n {
        let (x, y) = (a.num(i), b.num(i));
        if y == 0 {
            out.push(x.signum(), 0);
            continue;
        }
        // gcd(x, y) = gcd(y, x mod y): one division brings both operands
        // below the divisor, where Stein's loop takes a few steps rather than
        // one per bit of `x`. Most quotients are already in lowest terms, and
        // those need no further division.
        let m = y.unsigned_abs();
        let r = x.unsigned_abs() % m;
        let g = binary_gcd_u64(m, r);
        let (q, d) = if g == 1 {
            (x, y)
        } else {
            let g = i64::try_from(g).ok()?;
            (x / g, y / g)
        };
        let (q, d) = if d < 0 {
            (q.checked_neg()?, d.checked_neg()?)
        } else {
            (q, d)
        };
        out.push(q, d);
    }
    Some(out.into_value())
}

/// `x / y` for every lane `x` and one integer `y` with `1 <= |y| <=
/// GCD_TABLE_MAX`: [`integer_quotients`]'s answer, with no hardware division
/// per lane.
///
/// `x = q·m + r` with `m = |y|` and `0 <= r < m` ([`floor_divmod`]), and
/// `g = gcd(m, r) = gcd(x, y)`, which divides both `m` and `r`, so the reduced
/// numerator `x / g` is `q·(m/g) + r/g` — two table reads and a multiply.
/// `None` for the one quotient a machine word cannot hold, `i64::MIN / -1`.
fn quotients_by_small_integer(nums: &[i64], y: i64) -> Option<Value> {
    use crate::types::fraction::binary_gcd_u64;
    let m = y.unsigned_abs();
    // Per residue: `m / g` and `r / g`.
    let table: Vec<(u16, u16)> = (0..m)
        .map(|r| {
            let g = binary_gcd_u64(m, r);
            ((m / g) as u16, (r / g) as u16)
        })
        .collect();
    let (signed_m, inv) = (m as i64, 1.0 / m as f64);
    let negative = y < 0;
    let mut out = Out::with_capacity(nums.len());
    for &x in nums {
        let (q, r) = floor_divmod(x, signed_m, inv);
        let (m_over_g, r_over_g) = table[r as usize];
        let (m_over_g, r_over_g) = (i64::from(m_over_g), i64::from(r_over_g));
        // `x / g` exactly: `q·m + r` divided through by `g`.
        let numerator = if m_over_g == signed_m {
            x
        } else {
            q.checked_mul(m_over_g)?.checked_add(r_over_g)?
        };
        if negative {
            out.push(numerator.checked_neg()?, m_over_g);
        } else {
            out.push(numerator, m_over_g);
        }
    }
    Some(out.into_value())
}

/// The floor quotient and remainder of `x` by `m >= 1`: `x = q·m + r` with
/// `0 <= r < m`, as `div_euclid` / `rem_euclid` answer them.
///
/// A hardware 64-bit division costs tens of cycles a lane, and on the column
/// kernels it was most of the work. Below 2^52 every integer is a double, and
/// `x · (1/m)` lands within one of the true quotient (`m >= 2` keeps it under
/// 2^51, where the two roundings together err by less than one), so the
/// estimate is corrected by at most a step each way against the exact
/// integer remainder. Anything wider takes the division.
#[inline(always)]
pub(crate) fn floor_divmod(x: i64, m: i64, inv: f64) -> (i64, i64) {
    const EXACT: u64 = 1 << 52;
    if m == 1 {
        return (x, 0);
    }
    if x.unsigned_abs() >= EXACT || m as u64 >= EXACT {
        return (x.div_euclid(m), x.rem_euclid(m));
    }
    let mut q = (x as f64 * inv) as i64;
    let mut r = x - q * m;
    while r < 0 {
        q -= 1;
        r += m;
    }
    while r >= m {
        q += 1;
        r -= m;
    }
    (q, r)
}

/// `n ± k` for a column of rationals and one integer `k`, either side:
/// `n/d ± k` is `(n ± k·d)/d`, already in lowest terms because
/// `gcd(n ± k·d, d) = gcd(n, d) = 1`, so a lane needs no gcd. The same law
/// answers the three points over zero (`d = 0` leaves `n` as it was, or
/// negated for `k - n`), which is what `add_total` answers for them. `None`
/// for anything else, or when a lane overflows.
fn shifted_by_integer(schema: ExactArithmeticSchema, a: Lanes, b: Lanes) -> Option<Value> {
    let subtract = match schema {
        ExactArithmeticSchema::Add => false,
        ExactArithmeticSchema::Sub => true,
        _ => return None,
    };
    let (nums, dens, k, k_first) = match (a, b) {
        (Lanes::Columns { nums, dens, .. }, Lanes::Splat(k, 1)) => (nums, dens, k, false),
        (Lanes::Splat(k, 1), Lanes::Columns { nums, dens, .. }) => (nums, dens, k, true),
        _ => return None,
    };
    let mut out = vec![0i64; nums.len()];
    let mut bad = false;
    for ((o, &n), &d) in out.iter_mut().zip(nums).zip(dens) {
        let (kd, f) = small_rational::overflowing_mul(k, d);
        let (v, g) = match (subtract, k_first) {
            (false, _) => n.overflowing_add(kd),
            (true, false) => n.overflowing_sub(kd),
            (true, true) => kd.overflowing_sub(n),
        };
        *o = v;
        bad |= f | g;
    }
    (!bad).then(|| Out::columns(out, dens.to_vec()).into_value())
}

/// `a schema b` lane by lane, or `None` for the general route.
pub(crate) fn arithmetic(schema: ExactArithmeticSchema, a: &Value, b: &Value) -> Option<Value> {
    hit(arithmetic_lanes(schema, a, b))
}

fn arithmetic_lanes(schema: ExactArithmeticSchema, a: &Value, b: &Value) -> Option<Value> {
    let (a, b) = (Lanes::of(a)?, Lanes::of(b)?);
    let n = paired(a, b)?;
    if a.integer() && b.integer() {
        let lanes = match schema {
            ExactArithmeticSchema::Add => integer_lanes(a, b, n, i64::overflowing_add),
            ExactArithmeticSchema::Sub => integer_lanes(a, b, n, i64::overflowing_sub),
            ExactArithmeticSchema::Mul => integer_lanes(a, b, n, small_rational::overflowing_mul),
            ExactArithmeticSchema::Div => None,
        };
        if let Some(nums) = lanes {
            return Some(Out::integers(nums).into_value());
        }
        if matches!(schema, ExactArithmeticSchema::Div) {
            return integer_quotients(a, b, n);
        }
    }
    if let Some(shifted) = shifted_by_integer(schema, a, b) {
        return Some(shifted);
    }
    let mut out = Out::with_capacity(n);
    for i in 0..n {
        let (x, y) = (a.at(i), b.at(i));
        // A lane over zero, or a zero divisor, is the `Fraction`'s own law
        // (`small_rational`'s total forms): its answer is one of the three
        // points or zero, a small pair. A lane that outgrows a machine word
        // is a boxed Vector, the general route's to make.
        let (rn, rd) = match schema {
            ExactArithmeticSchema::Add => small_rational::add_total(x, y, false),
            ExactArithmeticSchema::Sub => small_rational::add_total(x, y, true),
            ExactArithmeticSchema::Mul => small_rational::mul_total(x, y),
            ExactArithmeticSchema::Div => small_rational::div_total(x, y),
        }?;
        out.push(rn, rd);
    }
    Some(out.into_value())
}

/// `a LT b` / `a GT b` lane by lane: the Vector of Booleans `lift_lanes`
/// builds, or `None` for the general route — which a `0/0` lane takes, since
/// it has no order to answer (LANG.VALUES.EXACT). `±1/0` are ordered below
/// and above every rational (`small_rational::order_total`).
pub(crate) fn ordering(kind: OrderingKind, a: &Value, b: &Value) -> Option<Value> {
    hit(ordering_lanes(kind, a, b))
}

fn ordering_lanes(kind: OrderingKind, a: &Value, b: &Value) -> Option<Value> {
    let (a, b) = (Lanes::of(a)?, Lanes::of(b)?);
    let n = paired(a, b)?;
    let truth = |ordering| kind.apply_ordering(ordering);
    if a.integer() && b.integer() {
        return Some(Value::from_truths(
            (0..n).map(|i| truth(a.num(i).cmp(&b.num(i)))),
        ));
    }
    let mut truths = Vec::with_capacity(n);
    for i in 0..n {
        truths.push(truth(small_rational::order_total(a.at(i), b.at(i))?));
    }
    Some(Value::from_truths(truths.into_iter()))
}

/// Which rounding `rounded` applies.
#[derive(Clone, Copy)]
pub(crate) enum Rounding {
    Floor,
    /// Half away from zero, as `Fraction::round`.
    Round,
}

/// `FLOOR`/`ROUND` of every lane, or `None` for the general route. Every
/// rational rounds to an integer, so the answer is a pure-integer Tensor
/// unless a lane is one of the three points over zero, which is its own
/// floor and its own rounding; for a Tensor that already was pure-integer,
/// that is the operand itself.
pub(crate) fn rounded(rounding: Rounding, value: &Value) -> Option<Value> {
    hit(rounded_lanes(rounding, value))
}

fn rounded_lanes(rounding: Rounding, value: &Value) -> Option<Value> {
    let lanes = Lanes::of(value)?;
    let Lanes::Columns {
        nums,
        dens,
        integer,
    } = lanes
    else {
        return None;
    };
    if integer {
        return Some(value.clone());
    }
    // A column's denominators repeat (after `7 DIV` they are 7 or 1), so the
    // reciprocal `floor_divmod` multiplies by is kept for the last one met;
    // an integer lane is its own floor and rounding and leaves it alone.
    let (mut last_d, mut inv) = (1i64, 1.0f64);
    let mut over_zero = false;
    let out: Vec<i64> = nums
        .iter()
        .zip(dens)
        .map(|(&n, &d)| {
            if d == 1 {
                return n;
            }
            if d == 0 {
                over_zero = true;
                return n;
            }
            match rounding {
                Rounding::Floor => {
                    if d != last_d {
                        (last_d, inv) = (d, 1.0 / d as f64);
                    }
                    floor_divmod(n, d, inv).0
                }
                Rounding::Round => small_rational::round_half_away_from_zero(n, d),
            }
        })
        .collect();
    if !over_zero {
        return Some(Out::integers(out).into_value());
    }
    // The three points over zero keep their own denominator.
    let out_dens = dens.iter().map(|&d| i64::from(d != 0)).collect();
    Some(Out::columns(out, out_dens).into_value())
}
