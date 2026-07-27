//! Semantic Plane words.
//!
//! These are the words that write and read the Semantic Plane. They are the
//! only way a role is set deliberately; every other role on a value got there
//! by a literal or by the propagation rule in
//! [`role::retain`](crate::role::retain).
//!
//! Asserting a role is checked. `>TEXT` on a vector holding `-1` is an error,
//! not a role that renders as garbage: a reading that the shape contradicts is
//! never allowed onto a value.

use crate::error::{Error, Result};
use crate::role::{self, Role};
use crate::value::Value;

use super::support::arity_bug;

fn assert_role(word: &str, args: &[Value], role: Role) -> Result<Vec<Value>> {
    let [a] = args else {
        return Err(arity_bug(word, 1, args.len()));
    };
    role::admits(role, a).map_err(|reason| Error::BadRole {
        role: role.name().to_string(),
        reason,
    })?;
    Ok(vec![a.clone().with_role(role)])
}

pub fn to_text(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    assert_role(word, args, Role::Text)
}

pub fn to_interval(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    assert_role(word, args, Role::Interval)
}

pub fn to_raw(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    assert_role(word, args, Role::Raw)
}

/// Push the value's reading as text. Pair it with `KEEP` when you want to
/// observe the reading without swallowing the value: `& ROLE`.
pub fn role_of(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [a] = args else {
        return Err(arity_bug(word, 1, args.len()));
    };
    Ok(vec![Value::text(a.role().name())])
}
