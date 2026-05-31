//! Strong Kleene three-valued logic (K3) for `AND` / `OR` / `NOT`
//! (SPEC §7.5), with the NIL/Unknown interaction rules of SPEC §4.5.2.
//!
//! This is the single canonical implementation of the K3 truth tables;
//! both the `StackTop` and `STAK` paths in [`super::logic`] route through
//! it so the comparison-word `Unknown` (U) and the logic words never
//! drift apart (SPEC §14.1). The truth domain is {T, F, U}; operational
//! NIL is tracked as a fourth input category so that the absorbing rules
//! (`F` for `AND`, `T` for `OR`) and the NIL-over-U priority rule
//! (SPEC §4.5.2) can be expressed uniformly.

use crate::types::Value;

/// An operand of a logic word, classified into the logical truth domain
/// {True, False, Unknown} plus the operational `Nil` category.
///
/// `Unknown` is the logical U (SPEC §7.5); `Nil` is an operational
/// absence (a reasoned Bubble/NIL, SPEC §4.5) that is *not* U. A
/// non-NIL, non-U operand is collapsed to `True`/`False` by its
/// truthiness, which is how a vector/scalar operand participates when
/// the *other* operand forces the scalar K3 path.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Ternary {
    True,
    False,
    Unknown,
    Nil,
}

impl Ternary {
    /// Classify a value for K3 combination. U is detected via the
    /// canonical [`Value::is_unknown`] predicate (never by matching the
    /// underlying NIL representation) so U is always separated from an
    /// operational NIL.
    pub(crate) fn classify(value: &Value) -> Ternary {
        if value.is_unknown() {
            Ternary::Unknown
        } else if value.is_nil() {
            Ternary::Nil
        } else if value.is_truthy() {
            Ternary::True
        } else {
            Ternary::False
        }
    }

    /// Materialize the combination result as a stack value: T/F become
    /// `TruthValue`-role booleans, U becomes [`Value::unknown`], and an
    /// operational NIL becomes a plain NIL (matching the historical
    /// reasonless NIL the logic words produced for NIL combinations).
    pub(crate) fn into_value(self) -> Value {
        match self {
            Ternary::True => truth_bool(true),
            Ternary::False => truth_bool(false),
            Ternary::Unknown => Value::unknown(),
            Ternary::Nil => Value::nil(),
        }
    }
}

/// A definite `true`/`false` carrying the `TruthValue` interpretation
/// role so it displays as `TRUE`/`FALSE` and serializes through the
/// `truthValue` axis.
fn truth_bool(b: bool) -> Value {
    let mut v = Value::from_bool(b);
    v.hint = crate::types::Interpretation::TruthValue;
    v
}

/// K3 `AND`: `F` absorbs (even over U and NIL); otherwise NIL takes
/// priority over U (SPEC §4.5.2); otherwise U propagates; else T.
pub(crate) fn and(a: Ternary, b: Ternary) -> Ternary {
    if a == Ternary::False || b == Ternary::False {
        Ternary::False
    } else if a == Ternary::Nil || b == Ternary::Nil {
        Ternary::Nil
    } else if a == Ternary::Unknown || b == Ternary::Unknown {
        Ternary::Unknown
    } else {
        Ternary::True
    }
}

/// K3 `OR`: `T` absorbs (even over U and NIL); otherwise NIL takes
/// priority over U (SPEC §4.5.2); otherwise U propagates; else F.
pub(crate) fn or(a: Ternary, b: Ternary) -> Ternary {
    if a == Ternary::True || b == Ternary::True {
        Ternary::True
    } else if a == Ternary::Nil || b == Ternary::Nil {
        Ternary::Nil
    } else if a == Ternary::Unknown || b == Ternary::Unknown {
        Ternary::Unknown
    } else {
        Ternary::False
    }
}

/// K3 `NOT`: `¬T=F`, `¬F=T`, `¬U=U`; NIL passes through as NIL.
pub(crate) fn not(a: Ternary) -> Ternary {
    match a {
        Ternary::True => Ternary::False,
        Ternary::False => Ternary::True,
        Ternary::Unknown => Ternary::Unknown,
        Ternary::Nil => Ternary::Nil,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> Value {
        let mut v = Value::from_bool(true);
        v.hint = crate::types::Interpretation::TruthValue;
        v
    }
    fn f() -> Value {
        let mut v = Value::from_bool(false);
        v.hint = crate::types::Interpretation::TruthValue;
        v
    }
    fn u() -> Value {
        Value::unknown()
    }
    fn n() -> Value {
        Value::nil()
    }

    #[test]
    fn classify_separates_unknown_from_nil() {
        assert_eq!(Ternary::classify(&t()), Ternary::True);
        assert_eq!(Ternary::classify(&f()), Ternary::False);
        assert_eq!(Ternary::classify(&u()), Ternary::Unknown);
        assert_eq!(Ternary::classify(&n()), Ternary::Nil);
    }

    // SPEC §7.5 AND truth table, all nine {T,F,U}^2 cells.
    #[test]
    fn k3_and_truth_table() {
        use Ternary::*;
        assert_eq!(and(True, True), True);
        assert_eq!(and(True, Unknown), Unknown);
        assert_eq!(and(True, False), False);
        assert_eq!(and(Unknown, True), Unknown);
        assert_eq!(and(Unknown, Unknown), Unknown);
        assert_eq!(and(Unknown, False), False);
        assert_eq!(and(False, True), False);
        assert_eq!(and(False, Unknown), False);
        assert_eq!(and(False, False), False);
    }

    // SPEC §7.5 OR truth table, all nine {T,F,U}^2 cells.
    #[test]
    fn k3_or_truth_table() {
        use Ternary::*;
        assert_eq!(or(False, False), False);
        assert_eq!(or(False, Unknown), Unknown);
        assert_eq!(or(False, True), True);
        assert_eq!(or(Unknown, False), Unknown);
        assert_eq!(or(Unknown, Unknown), Unknown);
        assert_eq!(or(Unknown, True), True);
        assert_eq!(or(True, False), True);
        assert_eq!(or(True, Unknown), True);
        assert_eq!(or(True, True), True);
    }

    // SPEC §7.5 NOT truth table.
    #[test]
    fn k3_not_truth_table() {
        use Ternary::*;
        assert_eq!(not(True), False);
        assert_eq!(not(Unknown), Unknown);
        assert_eq!(not(False), True);
    }

    // SPEC §4.5.2: NIL takes priority over U when neither is absorbed.
    #[test]
    fn nil_takes_priority_over_unknown() {
        use Ternary::*;
        assert_eq!(and(Nil, Unknown), Nil);
        assert_eq!(and(Unknown, Nil), Nil);
        assert_eq!(or(Nil, Unknown), Nil);
        assert_eq!(or(Unknown, Nil), Nil);
        // but an absorbing definite still wins over NIL
        assert_eq!(and(False, Nil), False);
        assert_eq!(or(True, Nil), True);
    }

    // Existing NIL semantics (SPEC §7.12) are unchanged by the U addition.
    #[test]
    fn nil_semantics_preserved() {
        use Ternary::*;
        assert_eq!(and(Nil, True), Nil);
        assert_eq!(and(Nil, False), False);
        assert_eq!(and(Nil, Nil), Nil);
        assert_eq!(or(Nil, False), Nil);
        assert_eq!(or(Nil, True), True);
        assert_eq!(or(Nil, Nil), Nil);
        assert_eq!(not(Nil), Nil);
    }

    #[test]
    fn into_value_roundtrips_truth_value() {
        assert!(Ternary::Unknown.into_value().is_unknown());
        assert!(Ternary::Nil.into_value().is_nil() && !Ternary::Nil.into_value().is_unknown());
        assert!(Ternary::True.into_value().is_truth_value());
        assert!(Ternary::False.into_value().is_truth_value());
    }
}
