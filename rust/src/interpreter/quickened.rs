//! A Word call specialised to the operands it meets: the scalar arithmetic,
//! comparison, rounding and logic Words, and SELECT, on plain machine-word
//! rationals and truth values.
//!
//! A compiled `CallBuiltin` runs the whole dispatch for every call: the
//! declared NIL contract, the declared lift, the Word's own NIL passthrough
//! and Record lift, the operand meter, the scalar fast path, the result-size
//! check and the NIL trace — about a thousand instructions to add two small
//! numbers, most of them there for operands that are absent, lifted or wide.
//! When both operands are plain scalars whose halves fit a machine word, none
//! of that machinery has anything to do, and what remains is the arithmetic
//! and its charges.
//!
//! This answers exactly what the dispatch would, and charges exactly what it
//! does, or declines and leaves the call to it:
//!
//! - one execution step (`charge_execution_step`, the same counter);
//! - for `ADD` `SUB` `MUL` `DIV`, one unit of numeric work — two operands of
//!   one limb each, `binary_numeric_work(1, 1)` — and a fast-path hit;
//! - for `LT` `GT` and an `EQ` of two numbers, a fast-path hit and no work;
//! - for `MIN` `MAX`, `FLOOR` `ROUND`, `NOT` `AND`, `SELECT` and an `EQ` of two
//!   truth values, neither;
//! - the nesting check the dispatch makes after every Word
//!   (`check_fresh_nesting`), which also resets the stack's fresh mark.
//!
//! It declines whenever the dispatch could do anything else: an operand that
//! is not a plain machine-word rational, a result that leaves a machine word
//! or a zero divisor (both the dispatch's to build), a size ceiling below a
//! machine word, or a step or work ceiling the call would cross (the
//! dispatch's to report). The NIL contract, the lift and the trace have
//! nothing to do on a call that meets no absence and makes none. Which route
//! answered is unobservable (LANG.AUTHORITY.FREEDOM); `quickened_tests`
//! holds the two equal.

use crate::interpreter::Interpreter;
use crate::kernel::generated::WordId;
use crate::types::fraction::{Fraction, FractionRepr};
use crate::types::small_rational::{add, div, mul, order, Pair};
use crate::types::{Value, ValueData};
use std::cmp::Ordering;

/// The machine-word rational a stack slot holds, when it holds a plain one.
#[inline]
fn small(value: &Value) -> Option<Pair> {
    if value.absence.is_some() {
        return None;
    }
    match &value.data {
        // A zero denominator is the absent sentinel, not a number.
        ValueData::Scalar(Fraction {
            repr: FractionRepr::Small(n, d),
        }) if *d != 0 => Some((*n, *d)),
        _ => None,
    }
}

/// A plain truth value in a stack slot.
#[inline]
fn truth(value: &Value) -> Option<bool> {
    match (&value.data, &value.absence) {
        (ValueData::Boolean(b), None) => Some(*b),
        _ => None,
    }
}

/// A slot `SELECT` may choose without lifting: a plain machine-word
/// rational or a plain truth value.
#[inline]
fn candidate(value: &Value) -> bool {
    small(value).is_some() || truth(value).is_some()
}

#[inline]
fn number((n, d): Pair) -> Value {
    Value::from_fraction(Fraction::from_repr(FractionRepr::Small(n, d)))
}

/// What `word` does to the top of the stack — how many slots it consumes,
/// what it leaves, and the work and fast-path hits it is charged — when it
/// is one of the calls answered here, or `None`.
fn answer(slots: &[Value], word: WordId) -> Option<(usize, Value, u64, u64)> {
    let top = |k: usize| slots.len().checked_sub(k).map(|i| &slots[i]);
    match word {
        WordId::Add | WordId::Sub | WordId::Mul | WordId::Div => {
            let (a, b) = (small(top(2)?)?, small(top(1)?)?);
            let r = match word {
                WordId::Add => add(a, b, false),
                WordId::Sub => add(a, b, true),
                WordId::Mul => mul(a, b),
                _ => div(a, b),
            }?;
            Some((2, number(r), 1, 1))
        }
        WordId::Lt | WordId::Gt => {
            let (a, b) = (small(top(2)?)?, small(top(1)?)?);
            let want = if word == WordId::Lt {
                Ordering::Less
            } else {
                Ordering::Greater
            };
            Some((2, Value::from_bool(order(a, b) == want), 0, 1))
        }
        // Two numbers compare on the fast path; two truth values by
        // `pairwise_eq`, off it.
        WordId::Eq => match (top(2)?, top(1)?) {
            (x, y) if small(x).is_some() && small(y).is_some() => {
                Some((2, Value::from_bool(small(x) == small(y)), 0, 1))
            }
            (x, y) => Some((2, Value::from_bool(truth(x)? == truth(y)?), 0, 0)),
        },
        // The left operand on a tie, as MIN and MAX keep it.
        WordId::Min | WordId::Max => {
            let (a, b) = (small(top(2)?)?, small(top(1)?)?);
            let take_right = if word == WordId::Min {
                order(b, a) == Ordering::Less
            } else {
                order(a, b) == Ordering::Less
            };
            Some((2, number(if take_right { b } else { a }), 0, 0))
        }
        WordId::Floor => {
            let (n, d) = small(top(1)?)?;
            Some((1, number((n.div_euclid(d), 1)), 0, 0))
        }
        WordId::Round => {
            let (n, d) = small(top(1)?)?;
            let (n, d) = (i128::from(n), i128::from(d));
            let magnitude = (2 * n.abs() + d) / (2 * d);
            let rounded = i64::try_from(if n < 0 { -magnitude } else { magnitude }).ok()?;
            Some((1, number((rounded, 1)), 0, 0))
        }
        WordId::Not => Some((1, Value::from_bool(!truth(top(1)?)?), 0, 0)),
        WordId::And => {
            let (a, b) = (truth(top(2)?)?, truth(top(1)?)?);
            Some((2, Value::from_bool(a && b), 0, 0))
        }
        WordId::Select => {
            let mask = truth(top(1)?)?;
            let (when_true, when_false) = (top(3)?, top(2)?);
            (candidate(when_true) && candidate(when_false)).then_some(())?;
            let chosen = if mask { when_true } else { when_false };
            Some((3, chosen.clone(), 0, 0))
        }
        _ => None,
    }
}

/// Run `word` on the plain values on top of the stack, or answer `false`
/// having touched nothing.
pub(crate) fn try_scalar_call(interp: &mut Interpreter, word: WordId) -> bool {
    if !interp.quickening_enabled || !interp.scalar_fastpath_enabled {
        return false;
    }
    let Some((pops, result, work, fastpath)) = answer(interp.stack.as_slice(), word) else {
        return false;
    };
    // Every ceiling the dispatch could stop at, checked before anything
    // moves: past one, the dispatch runs and reports it.
    if interp.runtime_limits.max_bigint_bits < 64
        || interp.execution_step_count >= interp.max_execution_steps
        || interp.numeric_work_used.saturating_add(work) > interp.runtime_limits.max_numeric_work
    {
        return false;
    }

    interp.execution_step_count += 1;
    interp.numeric_work_used += work;
    interp.runtime_metrics.scalar_fastpath_count = interp
        .runtime_metrics
        .scalar_fastpath_count
        .saturating_add(fastpath);
    for _ in 0..pops {
        interp.stack.pop();
    }
    interp.stack.push(result);
    // A plain scalar or truth value nests nothing, so this cannot fail; it
    // is made for the fresh mark it resets, which the dispatch resets too.
    interp
        .check_fresh_nesting()
        .expect("a plain result is within any nesting ceiling");
    #[cfg(test)]
    QUICKENED.with(|c| c.set(c.get() + 1));
    true
}

#[cfg(test)]
thread_local! {
    static QUICKENED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Calls this route answered on this thread, for tests that pin it ran.
#[cfg(test)]
pub(crate) fn quickened_calls_on_this_thread() -> u64 {
    QUICKENED.with(|c| c.get())
}
