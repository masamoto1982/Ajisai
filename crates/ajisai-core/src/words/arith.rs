//! Exact arithmetic.
//!
//! Absence and indeterminacy propagate by two separate rules, and where they
//! meet, absence wins: there is nothing for an arithmetic word to be
//! indeterminate *about* once an operand turns out not to exist.
//!
//! * `NIL` in any operand position — the result is `NIL`. The computation is
//!   not attempted.
//! * `UNKNOWN` in any operand position, with no `NIL` present — the result is
//!   `UNKNOWN`.

use crate::error::{Error, Result};
use crate::number::Number;
use crate::value::Value;

use super::support::{arity_bug, number_of};

fn binary(
    word: &str,
    args: &[Value],
    op: impl Fn(&Number, &Number) -> Result<Number>,
) -> Result<Vec<Value>> {
    let [a, b] = args else {
        return Err(arity_bug(word, 2, args.len()));
    };
    if a.is_nil() || b.is_nil() {
        return Ok(vec![Value::nil()]);
    }
    if a.is_unknown() || b.is_unknown() {
        return Ok(vec![Value::unknown()]);
    }
    let result = op(number_of(word, a)?, number_of(word, b)?)?;
    Ok(vec![Value::number(result)])
}

fn unary(word: &str, args: &[Value], op: impl Fn(&Number) -> Number) -> Result<Vec<Value>> {
    let [a] = args else {
        return Err(arity_bug(word, 1, args.len()));
    };
    if a.is_nil() {
        return Ok(vec![Value::nil()]);
    }
    if a.is_unknown() {
        return Ok(vec![Value::unknown()]);
    }
    Ok(vec![Value::number(op(number_of(word, a)?))])
}

pub fn add(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    binary(word, args, |a, b| Ok(a + b))
}

pub fn sub(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    binary(word, args, |a, b| Ok(a - b))
}

pub fn mul(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    binary(word, args, |a, b| Ok(a * b))
}

pub fn div(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    binary(word, args, |a, b| {
        a.checked_div(b).ok_or(Error::DivisionByZero)
    })
}

pub fn min(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    binary(word, args, |a, b| {
        Ok(if a <= b { a.clone() } else { b.clone() })
    })
}

pub fn max(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    binary(word, args, |a, b| {
        Ok(if a >= b { a.clone() } else { b.clone() })
    })
}

pub fn neg(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    unary(word, args, |a| -a)
}

pub fn abs(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    unary(word, args, Number::abs)
}
