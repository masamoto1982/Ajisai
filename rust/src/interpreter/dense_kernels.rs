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
//! An absent lane is not a reason to decline either, in an operand or in the
//! answer. A dense Tensor holds one as it holds any other lane: the
//! denominator-0 sentinel in the columns, and the reason in the Tensor's
//! absence map, which is sized to the failures rather than to the data. So
//! the two laws that meet absence are the kernels' own:
//!
//! - **Passthrough** (LANG.FAILURE.PASSTHROUGH): an absent operand lane is
//!   the result lane, the leftmost operand first, carried with its reason and
//!   never re-minted. On the integer route the sentinel's numerator is 0, so
//!   the vectorised loop runs over it like any lane and the absent lanes are
//!   overwritten afterwards, from the sentinel scan, at the cost of the
//!   failures rather than the data (`carry_absent_lanes`).
//! - **Projection** (LANG.FAILURE.PROJECT): `DIV` by a zero lane writes the
//!   sentinel, records `divisionByZero` for that lane, and keeps dividing the
//!   rest.
//!
//! The kernels used to hand both cases to the general route, which boxed a
//! million lanes to say that one of them was absent, and left the result
//! boxed for every Word after it. One absent lane then cost a pipeline its
//! columns from that Word on.

mod lanes;

use crate::interpreter::arithmetic::ExactArithmeticSchema;
use crate::interpreter::comparison::OrderingKind;
use crate::types::small_rational;
use crate::types::Column;
use crate::types::Value;
use lanes::{carry_absent_lanes, paired, Lanes, Out};

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
/// to vectorise. A set flag declines the whole operation. An absent lane's
/// sentinel numerator is 0, which `f` takes like any other integer; the lane
/// is overwritten afterwards (`carry_absent_lanes`).
#[inline(always)]
fn integer_lanes(
    a: Lanes,
    b: Lanes,
    n: usize,
    f: impl Fn(i64, i64) -> (i64, bool),
) -> Option<Column> {
    let mut out: Column = smallvec::smallvec![0i64; n];
    let mut bad = false;
    match (a, b) {
        (Lanes::Columns { tensor: x, .. }, Lanes::Columns { tensor: y, .. }) => {
            for ((o, &x), &y) in out.iter_mut().zip(&x.numerators).zip(&y.numerators) {
                let (v, f) = f(x, y);
                *o = v;
                bad |= f;
            }
        }
        (Lanes::Columns { tensor: x, .. }, Lanes::Splat(y, _)) => {
            for (o, &x) in out.iter_mut().zip(&x.numerators) {
                let (v, f) = f(x, y);
                *o = v;
                bad |= f;
            }
        }
        (Lanes::Splat(x, _), Lanes::Columns { tensor: y, .. }) => {
            for (o, &y) in out.iter_mut().zip(&y.numerators) {
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
/// carried by the numerator; an absent operand lane carried, a zero divisor
/// an absent lane. `None` for the one quotient a machine word cannot hold,
/// `i64::MIN / -1`.
fn integer_quotients(a: Lanes, b: Lanes, n: usize) -> Option<Value> {
    use crate::types::fraction::binary_gcd_u64;
    // A Tensor divided by one small integer (`7 DIV`, the common case) meets
    // only `|y|` residues, so their gcds are read from a table built once.
    let residue_gcds: Option<Vec<u16>> = match b {
        Lanes::Splat(y, _) if (1..=GCD_TABLE_MAX).contains(&y.unsigned_abs()) => {
            let m = y.unsigned_abs();
            (m as usize <= n).then(|| (0..m).map(|r| binary_gcd_u64(m, r) as u16).collect())
        }
        _ => None,
    };
    let absent = a.absent() || b.absent();
    let mut out = Out::with_capacity(n);
    for i in 0..n {
        if absent {
            if a.den(i) == 0 {
                out.carry(i, a);
                continue;
            }
            if b.den(i) == 0 {
                out.carry(i, b);
                continue;
            }
        }
        let (x, y) = (a.num(i), b.num(i));
        if y == 0 {
            out.project(i);
            continue;
        }
        // gcd(x, y) = gcd(y, x mod y): one division brings both operands
        // below the divisor, where Stein's loop takes a few steps rather than
        // one per bit of `x`. Most quotients are already in lowest terms, and
        // those need no further division.
        let m = y.unsigned_abs();
        let r = x.unsigned_abs() % m;
        let g = match &residue_gcds {
            Some(table) => u64::from(table[r as usize]),
            None => binary_gcd_u64(m, r),
        };
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
            let mut out = Out::integers(nums);
            carry_absent_lanes(&mut out, a, b);
            return Some(out.into_value());
        }
        if matches!(schema, ExactArithmeticSchema::Div) {
            return integer_quotients(a, b, n);
        }
    }
    let mut out = Out::with_capacity(n);
    for i in 0..n {
        let (x, y) = (a.at(i), b.at(i));
        // A present lane never has denominator 0, so the sentinel alone says
        // which operand lane is absent: that lane is the result, leftmost
        // first (LANG.FAILURE.PASSTHROUGH). Then a zero divisor projects its
        // own lane; a lane that outgrows a machine word is a boxed Vector,
        // the general route's to make.
        if x.1 == 0 {
            out.carry(i, a);
            continue;
        }
        if y.1 == 0 {
            out.carry(i, b);
            continue;
        }
        if matches!(schema, ExactArithmeticSchema::Div) && y.0 == 0 {
            out.project(i);
            continue;
        }
        let (rn, rd) = match schema {
            ExactArithmeticSchema::Add => small_rational::add(x, y, false),
            ExactArithmeticSchema::Sub => small_rational::add(x, y, true),
            ExactArithmeticSchema::Mul => small_rational::mul(x, y),
            ExactArithmeticSchema::Div => small_rational::div(x, y),
        }?;
        out.push(rn, rd);
    }
    Some(out.into_value())
}

/// `a LT b` / `a GT b` lane by lane: the Vector of Booleans `lift_lanes`
/// builds, an absent operand lane passed through as the NIL it would answer
/// there, or `None` for the general route. Denominators are positive, so the
/// cross products order the lanes, and they fit `i128`.
pub(crate) fn ordering(kind: OrderingKind, a: &Value, b: &Value) -> Option<Value> {
    hit(ordering_lanes(kind, a, b))
}

fn ordering_lanes(kind: OrderingKind, a: &Value, b: &Value) -> Option<Value> {
    let (a, b) = (Lanes::of(a)?, Lanes::of(b)?);
    let n = paired(a, b)?;
    let integer = a.integer() && b.integer();
    let absent = a.absent() || b.absent();
    let decide = |i: usize| -> Value {
        if absent {
            if a.den(i) == 0 {
                return Value::nil_with_absence(a.absence(i));
            }
            if b.den(i) == 0 {
                return Value::nil_with_absence(b.absence(i));
            }
        }
        let ordering = if integer {
            a.num(i).cmp(&b.num(i))
        } else {
            let (an, ad) = a.at(i);
            let (bn, bd) = b.at(i);
            (i128::from(an) * i128::from(bd)).cmp(&(i128::from(bn) * i128::from(ad)))
        };
        Value::from_bool(kind.apply_ordering(ordering))
    };
    Some(Value::from_vector((0..n).map(decide).collect()))
}

/// Which rounding `rounded` applies.
#[derive(Clone, Copy)]
pub(crate) enum Rounding {
    Floor,
    /// Half away from zero, as `Fraction::round`.
    Round,
}

/// `FLOOR`/`ROUND` of every lane, or `None` for the general route. Every
/// present result is an integer, so the answer is a pure-integer Tensor; for
/// one that already was, that is the operand itself. An absent lane passes
/// through with its reason.
pub(crate) fn rounded(rounding: Rounding, value: &Value) -> Option<Value> {
    hit(rounded_lanes(rounding, value))
}

fn rounded_lanes(rounding: Rounding, value: &Value) -> Option<Value> {
    let lanes = Lanes::of(value)?;
    let Lanes::Columns {
        tensor,
        integer,
        absent,
    } = lanes
    else {
        return None;
    };
    if integer && !absent {
        return Some(value.clone());
    }
    let mut out = Out::with_capacity(tensor.len());
    for (i, (&n, &d)) in tensor
        .numerators
        .iter()
        .zip(&tensor.denominators)
        .enumerate()
    {
        if d == 0 {
            out.carry(i, lanes);
            continue;
        }
        let rounded = match rounding {
            Rounding::Floor => n.div_euclid(d),
            Rounding::Round => small_rational::round_half_away_from_zero(n, d),
        };
        out.push(rounded, 1);
    }
    Some(out.into_value())
}
