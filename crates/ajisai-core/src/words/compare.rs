//! Comparison — the canonical source of `UNKNOWN`.
//!
//! Comparison is where an observation can fail to settle a question, so it is
//! where the third truth value enters the language:
//!
//! * An operand that is `NIL` — there is no value to compare, so the result is
//!   `UNKNOWN`, not `FALSE`. `NIL NIL EQ` is `UNKNOWN` too: two absences are
//!   not evidence of sameness. Ask `NIL?` when you want to observe absence
//!   itself, which is a question observation *can* settle.
//! * An operand that is `UNKNOWN` — the result is `UNKNOWN`.
//!
//! Nothing here ever produces `FALSE` for a comparison it could not make, and
//! no path converts the resulting `UNKNOWN` back into a Boolean.

use std::cmp::Ordering;

use crate::error::Result;
use crate::value::Value;

use super::support::{arity_bug, number_of};

/// `NIL` or `UNKNOWN` in either operand makes the comparison unsettleable.
fn undecided(a: &Value, b: &Value) -> bool {
    a.is_nil() || b.is_nil() || a.is_unknown() || b.is_unknown()
}

fn equality(word: &str, args: &[Value], want_equal: bool) -> Result<Vec<Value>> {
    let [a, b] = args else {
        return Err(arity_bug(word, 2, args.len()));
    };
    if undecided(a, b) {
        return Ok(vec![Value::unknown()]);
    }
    // Data Plane equality: the Semantic Plane reading is deliberately not
    // consulted, so `"A"` and `[ 65 ]` compare equal.
    Ok(vec![Value::boolean((a == b) == want_equal)])
}

fn ordering(word: &str, args: &[Value], accept: fn(Ordering) -> bool) -> Result<Vec<Value>> {
    let [a, b] = args else {
        return Err(arity_bug(word, 2, args.len()));
    };
    if undecided(a, b) {
        return Ok(vec![Value::unknown()]);
    }
    let order = number_of(word, a)?.cmp(number_of(word, b)?);
    Ok(vec![Value::boolean(accept(order))])
}

pub fn eq(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    equality(word, args, true)
}

pub fn ne(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    equality(word, args, false)
}

pub fn lt(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    ordering(word, args, |o| o == Ordering::Less)
}

pub fn le(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    ordering(word, args, |o| o != Ordering::Greater)
}

pub fn gt(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    ordering(word, args, |o| o == Ordering::Greater)
}

pub fn ge(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    ordering(word, args, |o| o != Ordering::Less)
}
