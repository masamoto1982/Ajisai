//! Strong Kleene three-valued logic (K3).
//!
//! Ajisai's logic has three values, and `UNKNOWN` is one of them rather than a
//! placeholder waiting to be resolved into a Boolean. The tables below are the
//! Strong Kleene tables, and they are the only implementation of them in the
//! language.
//!
//! `UNKNOWN` is not `NIL` and not an error:
//!
//! * `NIL` reaching a logical position is an [`Error::NotATruthValue`]. An
//!   absence is not a truth value, and reading it as one is exactly how a
//!   three-valued logic quietly collapses back into two.
//! * `UNKNOWN` never becomes `FALSE`. `UNKNOWN AND FALSE` is `FALSE` because
//!   the Strong Kleene table says so — whatever the unknown side turns out to
//!   be, the conjunction is false — not because `UNKNOWN` was read as falsity.
//!
//! Water: `UNKNOWN` is the flow that reached the gauge and did not settle.

use crate::error::{Error, Result};
use crate::value::{Value, ValueData};

/// A truth value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Truth {
    True,
    False,
    Unknown,
}

impl Truth {
    /// Read a value as a truth value. `NIL`, numbers, vectors, and quotes are
    /// rejected rather than coerced.
    pub fn read(word: &str, value: &Value) -> Result<Truth> {
        match value.data() {
            ValueData::Boolean(true) => Ok(Truth::True),
            ValueData::Boolean(false) => Ok(Truth::False),
            ValueData::Unknown => Ok(Truth::Unknown),
            _ => Err(Error::NotATruthValue {
                word: word.to_string(),
                found: value.type_name().to_string(),
            }),
        }
    }

    pub fn into_value(self) -> Value {
        match self {
            Truth::True => Value::boolean(true),
            Truth::False => Value::boolean(false),
            Truth::Unknown => Value::unknown(),
        }
    }

    /// Strong Kleene conjunction: the minimum under `FALSE < UNKNOWN < TRUE`.
    pub fn and(self, other: Truth) -> Truth {
        match (self, other) {
            (Truth::False, _) | (_, Truth::False) => Truth::False,
            (Truth::Unknown, _) | (_, Truth::Unknown) => Truth::Unknown,
            (Truth::True, Truth::True) => Truth::True,
        }
    }

    /// Strong Kleene disjunction: the maximum under `FALSE < UNKNOWN < TRUE`.
    pub fn or(self, other: Truth) -> Truth {
        match (self, other) {
            (Truth::True, _) | (_, Truth::True) => Truth::True,
            (Truth::Unknown, _) | (_, Truth::Unknown) => Truth::Unknown,
            (Truth::False, Truth::False) => Truth::False,
        }
    }
}

/// Strong Kleene negation.
impl std::ops::Not for Truth {
    type Output = Truth;

    fn not(self) -> Truth {
        match self {
            Truth::True => Truth::False,
            Truth::False => Truth::True,
            Truth::Unknown => Truth::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Truth::{False, True, Unknown};
    use std::ops::Not;

    #[test]
    fn negation_table() {
        assert_eq!(True.not(), False);
        assert_eq!(False.not(), True);
        assert_eq!(Unknown.not(), Unknown);
    }

    #[test]
    fn conjunction_table() {
        assert_eq!(True.and(True), True);
        assert_eq!(True.and(False), False);
        assert_eq!(True.and(Unknown), Unknown);
        assert_eq!(False.and(True), False);
        assert_eq!(False.and(False), False);
        assert_eq!(False.and(Unknown), False);
        assert_eq!(Unknown.and(True), Unknown);
        assert_eq!(Unknown.and(False), False);
        assert_eq!(Unknown.and(Unknown), Unknown);
    }

    #[test]
    fn disjunction_table() {
        assert_eq!(True.or(True), True);
        assert_eq!(True.or(False), True);
        assert_eq!(True.or(Unknown), True);
        assert_eq!(False.or(True), True);
        assert_eq!(False.or(False), False);
        assert_eq!(False.or(Unknown), Unknown);
        assert_eq!(Unknown.or(True), True);
        assert_eq!(Unknown.or(False), Unknown);
        assert_eq!(Unknown.or(Unknown), Unknown);
    }

    #[test]
    fn de_morgan_holds_in_k3() {
        for a in [True, False, Unknown] {
            for b in [True, False, Unknown] {
                assert_eq!(a.and(b).not(), a.not().or(b.not()));
                assert_eq!(a.or(b).not(), a.not().and(b.not()));
            }
        }
    }

    #[test]
    fn excluded_middle_fails_as_k3_requires() {
        // `p OR NOT p` is not a tautology in K3 — this is the point of having
        // a third value at all, and a regression here would mean UNKNOWN had
        // quietly become a Boolean.
        assert_eq!(Unknown.or(Unknown.not()), Unknown);
        assert_eq!(Unknown.and(Unknown.not()), Unknown);
    }
}
