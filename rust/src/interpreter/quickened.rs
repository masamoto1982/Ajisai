//! A compiled call site specialised to the operands it meets: the scalar
//! arithmetic, comparison and selection Words on two machine-word rationals.
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
//! - for `LT` `GT` `EQ`, a fast-path hit and no work;
//! - for `MIN` `MAX`, neither;
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

/// Run `word` on the two scalars on top of the stack, or answer `false`
/// having touched nothing.
pub(crate) fn try_scalar_call(interp: &mut Interpreter, word: WordId) -> bool {
    if !interp.quickening_enabled || !interp.scalar_fastpath_enabled {
        return false;
    }
    let (work, fastpath) = match word {
        WordId::Add | WordId::Sub | WordId::Mul | WordId::Div => (1, 1),
        WordId::Lt | WordId::Gt | WordId::Eq => (0, 1),
        WordId::Min | WordId::Max => (0, 0),
        _ => return false,
    };
    let len = interp.stack.len();
    if len < 2 {
        return false;
    }
    let slots = interp.stack.as_slice();
    let (Some(a), Some(b)) = (small(&slots[len - 2]), small(&slots[len - 1])) else {
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
    let number =
        |(n, d): Pair| Value::from_fraction(Fraction::from_repr(FractionRepr::Small(n, d)));
    let result = match word {
        WordId::Add => add(a, b, false).map(number),
        WordId::Sub => add(a, b, true).map(number),
        WordId::Mul => mul(a, b).map(number),
        WordId::Div => div(a, b).map(number),
        WordId::Lt => Some(Value::from_bool(order(a, b) == Ordering::Less)),
        WordId::Gt => Some(Value::from_bool(order(a, b) == Ordering::Greater)),
        WordId::Eq => Some(Value::from_bool(a == b)),
        // The left operand on a tie, as MIN and MAX keep it.
        WordId::Min if order(b, a) == Ordering::Less => Some(number(b)),
        WordId::Max if order(a, b) == Ordering::Less => Some(number(b)),
        WordId::Min | WordId::Max => Some(number(a)),
        _ => None,
    };
    let Some(result) = result else {
        return false;
    };

    interp.execution_step_count += 1;
    interp.numeric_work_used += work;
    interp.runtime_metrics.scalar_fastpath_count = interp
        .runtime_metrics
        .scalar_fastpath_count
        .saturating_add(fastpath);
    interp.stack.pop();
    interp.stack.pop();
    interp.stack.push(result);
    // A scalar nests nothing, so this cannot fail; it is made for the
    // fresh mark it resets, which the dispatch resets too.
    interp
        .check_fresh_nesting()
        .expect("a scalar result is within any nesting ceiling");
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
