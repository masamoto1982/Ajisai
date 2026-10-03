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
//! one length, with no absent lane. They answer exactly the `Value` the
//! general route builds — the same dense Tensor (`from_fractions` of the same
//! lanes is `from_columns` of the same columns), or for a comparison the same
//! Vector of Booleans — and decline (`None`) wherever that route could answer
//! anything else: a zero divisor (a reasoned NIL lane), a lane that no longer
//! fits a machine word (a boxed Vector), any other shape. The charges are
//! untouched, because the dispatcher made them before choosing a route
//! (`charge_binary_schema`); which route ran is unobservable
//! (LANG.AUTHORITY.FREEDOM). `dense_kernels_tests` holds the two equal.

use crate::interpreter::arithmetic::ExactArithmeticSchema;
use crate::interpreter::comparison::OrderingKind;
use crate::types::small_rational;
use crate::types::Column;
use crate::types::{DenseTensor, Value, ValueData};
use std::collections::BTreeMap;
use std::sync::Arc;

/// An operand's lanes: a Tensor's columns, or one rational for every lane.
#[derive(Clone, Copy)]
enum Lanes<'a> {
    Columns {
        nums: &'a [i64],
        dens: &'a [i64],
        integer: bool,
    },
    Splat(i64, i64),
}

impl Lanes<'_> {
    fn of(value: &Value) -> Option<Lanes<'_>> {
        match &value.data {
            ValueData::Tensor { data, shape }
                if shape.len() == 1
                    && !data.is_empty()
                    && (data.is_pure_integer || data.all_lanes_valid()) =>
            {
                Some(Lanes::Columns {
                    nums: &data.numerators,
                    dens: &data.denominators,
                    integer: data.is_pure_integer,
                })
            }
            ValueData::Scalar(f) if value.absence.is_none() && !f.is_nil() => {
                let (n, d) = f.extract_i64_pair()?;
                Some(Lanes::Splat(n, d))
            }
            _ => None,
        }
    }

    fn len(self) -> Option<usize> {
        match self {
            Lanes::Columns { nums, .. } => Some(nums.len()),
            Lanes::Splat(..) => None,
        }
    }

    fn integer(self) -> bool {
        match self {
            Lanes::Columns { integer, .. } => integer,
            Lanes::Splat(_, d) => d == 1,
        }
    }

    #[inline(always)]
    fn at(self, i: usize) -> (i64, i64) {
        match self {
            Lanes::Columns { nums, dens, .. } => (nums[i], dens[i]),
            Lanes::Splat(n, d) => (n, d),
        }
    }

    #[inline(always)]
    fn num(self, i: usize) -> i64 {
        match self {
            Lanes::Columns { nums, .. } => nums[i],
            Lanes::Splat(n, _) => n,
        }
    }
}

/// The lane count two operands pair over: a Tensor's length against a
/// scalar, or the shared length of two Tensors. Anything else — two scalars,
/// two lengths — is the general route's.
fn paired(a: Lanes, b: Lanes) -> Option<usize> {
    match (a.len(), b.len()) {
        (Some(n), None) | (None, Some(n)) => Some(n),
        (Some(n), Some(m)) if n == m => Some(n),
        _ => None,
    }
}

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

fn dense_value(nums: Column, dens: Column, integer: bool) -> Value {
    let shape = vec![nums.len()];
    let tensor = DenseTensor::from_columns(nums, dens, shape.clone(), integer, BTreeMap::new());
    Value::new(
        ValueData::Tensor {
            data: Arc::new(tensor),
            shape: Arc::new(shape),
        },
        None,
    )
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
) -> Option<Column> {
    let mut out: Column = smallvec::smallvec![0i64; n];
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

/// `a / b` on integer lanes: each quotient reduced by one gcd, with the sign
/// carried by the numerator. `None` for a zero divisor (a NIL lane) or the
/// one quotient a machine word cannot hold, `i64::MIN / -1`.
fn integer_quotients(a: Lanes, b: Lanes, n: usize) -> Option<Value> {
    let mut nums = Column::with_capacity(n);
    let mut dens = Column::with_capacity(n);
    let mut integer = true;
    for i in 0..n {
        let (x, y) = (a.num(i), b.num(i));
        if y == 0 {
            return None;
        }
        let g = crate::types::fraction::binary_gcd_u64(x.unsigned_abs(), y.unsigned_abs());
        let g = i64::try_from(g).ok()?;
        let (q, d) = (x / g, y / g);
        let (q, d) = if d < 0 {
            (q.checked_neg()?, d.checked_neg()?)
        } else {
            (q, d)
        };
        nums.push(q);
        dens.push(d);
        integer &= d == 1;
    }
    Some(dense_value(nums, dens, integer))
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
            ExactArithmeticSchema::Mul => integer_lanes(a, b, n, i64::overflowing_mul),
            ExactArithmeticSchema::Div => None,
        };
        if let Some(nums) = lanes {
            return Some(dense_value(nums, smallvec::smallvec![1; n], true));
        }
        if matches!(schema, ExactArithmeticSchema::Div) {
            return integer_quotients(a, b, n);
        }
    }
    let mut nums = Column::with_capacity(n);
    let mut dens = Column::with_capacity(n);
    let mut integer = true;
    for i in 0..n {
        let (x, y) = (a.at(i), b.at(i));
        // A zero divisor is a reasoned NIL lane, the general route's to make,
        // and a lane that outgrows a machine word a boxed Vector.
        let (rn, rd) = match schema {
            ExactArithmeticSchema::Add => small_rational::add(x, y, false),
            ExactArithmeticSchema::Sub => small_rational::add(x, y, true),
            ExactArithmeticSchema::Mul => small_rational::mul(x, y),
            ExactArithmeticSchema::Div => small_rational::div(x, y),
        }?;
        nums.push(rn);
        dens.push(rd);
        integer &= rd == 1;
    }
    Some(dense_value(nums, dens, integer))
}

/// `a LT b` / `a GT b` lane by lane: the Vector of Booleans `lift_lanes`
/// builds, or `None` for the general route. Denominators are positive, so
/// the cross products order the lanes, and they fit `i128`.
pub(crate) fn ordering(kind: OrderingKind, a: &Value, b: &Value) -> Option<Value> {
    hit(ordering_lanes(kind, a, b))
}

fn ordering_lanes(kind: OrderingKind, a: &Value, b: &Value) -> Option<Value> {
    let (a, b) = (Lanes::of(a)?, Lanes::of(b)?);
    let n = paired(a, b)?;
    let integer = a.integer() && b.integer();
    let decide = |i: usize| -> bool {
        let ordering = if integer {
            a.num(i).cmp(&b.num(i))
        } else {
            let (an, ad) = a.at(i);
            let (bn, bd) = b.at(i);
            (i128::from(an) * i128::from(bd)).cmp(&(i128::from(bn) * i128::from(ad)))
        };
        kind.apply_ordering(ordering)
    };
    Some(Value::from_vector(
        (0..n).map(|i| Value::from_bool(decide(i))).collect(),
    ))
}

/// Which rounding `rounded` applies.
#[derive(Clone, Copy)]
pub(crate) enum Rounding {
    Floor,
    /// Half away from zero, as `Fraction::round`.
    Round,
}

/// `FLOOR`/`ROUND` of every lane, or `None` for the general route. Every
/// result is an integer, so the answer is a pure-integer Tensor; for one that
/// already was, that is the operand itself.
pub(crate) fn rounded(rounding: Rounding, value: &Value) -> Option<Value> {
    hit(rounded_lanes(rounding, value))
}

fn rounded_lanes(rounding: Rounding, value: &Value) -> Option<Value> {
    let Lanes::Columns {
        nums,
        dens,
        integer,
    } = Lanes::of(value)?
    else {
        return None;
    };
    if integer {
        return Some(value.clone());
    }
    let out = nums
        .iter()
        .zip(dens)
        .map(|(&n, &d)| match rounding {
            Rounding::Floor => n.div_euclid(d),
            Rounding::Round => {
                let (n, d) = (i128::from(n), i128::from(d));
                let magnitude = (2 * n.abs() + d) / (2 * d);
                (if n < 0 { -magnitude } else { magnitude }) as i64
            }
        })
        .collect::<Column>();
    let ones = smallvec::smallvec![1; out.len()];
    Some(dense_value(out, ones, true))
}
