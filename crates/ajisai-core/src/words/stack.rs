//! Flow-shaping words.
//!
//! These are ordinary words with ordinary contracts, not a second mode system.
//! Roles travel with values here for free, because a value's reading lives on
//! the value and nowhere else.

use crate::error::Result;
use crate::value::Value;

use super::support::arity_bug;

pub fn dup(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [a] = args else {
        return Err(arity_bug(word, 1, args.len()));
    };
    Ok(vec![a.clone(), a.clone()])
}

pub fn drop(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [_a] = args else {
        return Err(arity_bug(word, 1, args.len()));
    };
    Ok(Vec::new())
}

pub fn swap(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [a, b] = args else {
        return Err(arity_bug(word, 2, args.len()));
    };
    Ok(vec![b.clone(), a.clone()])
}
