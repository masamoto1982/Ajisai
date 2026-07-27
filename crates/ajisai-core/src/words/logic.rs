//! K3 logic words and the observation predicates.
//!
//! `NOT`, `AND`, and `OR` read their operands as truth values and reject
//! anything else, `NIL` included. The predicates below are the opposite: they
//! accept any value and always settle, because "is this absent?" and "is this
//! undetermined?" are questions a single observation answers.

use std::ops::Not;

use crate::error::Result;
use crate::k3::Truth;
use crate::value::{Value, ValueData};

use super::support::arity_bug;

pub fn not(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [a] = args else {
        return Err(arity_bug(word, 1, args.len()));
    };
    Ok(vec![Truth::read(word, a)?.not().into_value()])
}

pub fn and(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [a, b] = args else {
        return Err(arity_bug(word, 2, args.len()));
    };
    let result = Truth::read(word, a)?.and(Truth::read(word, b)?);
    Ok(vec![result.into_value()])
}

pub fn or(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [a, b] = args else {
        return Err(arity_bug(word, 2, args.len()));
    };
    let result = Truth::read(word, a)?.or(Truth::read(word, b)?);
    Ok(vec![result.into_value()])
}

fn predicate(word: &str, args: &[Value], test: fn(&Value) -> bool) -> Result<Vec<Value>> {
    let [a] = args else {
        return Err(arity_bug(word, 1, args.len()));
    };
    Ok(vec![Value::boolean(test(a))])
}

pub fn is_nil(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    predicate(word, args, Value::is_nil)
}

pub fn is_unknown(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    predicate(word, args, Value::is_unknown)
}

pub fn is_number(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    predicate(word, args, |v| matches!(v.data(), ValueData::Number(_)))
}

pub fn is_vector(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    predicate(word, args, |v| matches!(v.data(), ValueData::Vector(_)))
}

pub fn is_boolean(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    predicate(word, args, |v| matches!(v.data(), ValueData::Boolean(_)))
}

pub fn is_quote(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    predicate(word, args, |v| matches!(v.data(), ValueData::Quote(_)))
}

pub fn constant_true(_word: &str, _args: &[Value]) -> Result<Vec<Value>> {
    Ok(vec![Value::boolean(true)])
}

pub fn constant_false(_word: &str, _args: &[Value]) -> Result<Vec<Value>> {
    Ok(vec![Value::boolean(false)])
}

pub fn constant_unknown(_word: &str, _args: &[Value]) -> Result<Vec<Value>> {
    Ok(vec![Value::unknown()])
}

pub fn constant_nil(_word: &str, _args: &[Value]) -> Result<Vec<Value>> {
    Ok(vec![Value::nil()])
}
