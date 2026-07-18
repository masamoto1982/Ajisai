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

/// Materialize a K3 combination result as a `Value`, re-attaching the
/// comparison diagnosis (`agreedPrefix`, SPEC §4.5.0 / §7.4.1) when the
/// result is the logical Unknown (U).
///
/// The truth tables (`and`/`or`/`not`) stay pure `Ternary -> Ternary`; the
/// diagnosis is composed only here, at the `Value` layer, so the K3
/// semantics are never polluted by metadata. Policy: when the result is U,
/// the diagnosis of the **first U operand** (left-priority) is carried over
/// by cloning that operand value; a bare `Value::unknown()` operand carries
/// no diagnosis, so the result stays bare (diagnosis is never fabricated).
/// This is diagnostic-only: the observable `truthValue` is identical to
/// [`Ternary::into_value`] (both yield `unknown` for U), preserving the
/// SPEC §2.3 firewall.
pub(crate) fn into_value_with_diagnosis(result: Ternary, operands: &[&Value]) -> Value {
    if result != Ternary::Unknown {
        return result.into_value();
    }
    for op in operands {
        if op.is_unknown() {
            // The operand is itself a U value carrying (or lacking) the
            // comparison diagnosis; cloning it faithfully preserves the
            // agreedPrefix without inventing one.
            return (*op).clone();
        }
    }
    result.into_value()
}

/// A definite `true`/`false` carrying the `TruthValue` interpretation
/// role so it displays as `TRUE`/`FALSE` and serializes through the
/// `truthValue` axis.
fn truth_bool(b: bool) -> Value {
    let mut v = Value::from_bool(b);
    v.hint = crate::types::Interpretation::TruthValue;
    v
}

/// K3 meet (`AND`): `F` absorbs (even over U and NIL); otherwise NIL takes
/// priority over U (SPEC §4.5.2); otherwise U propagates; else T.
pub(crate) fn meet_k3(a: Ternary, b: Ternary) -> Ternary {
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

/// K3 join (`OR`): `T` absorbs (even over U and NIL); otherwise NIL takes
/// priority over U (SPEC §4.5.2); otherwise U propagates; else F.
pub(crate) fn join_k3(a: Ternary, b: Ternary) -> Ternary {
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

/// K3 involution (`NOT`): `¬T=F`, `¬F=T`, `¬U=U`; NIL passes through as NIL.
pub(crate) fn involution_k3(a: Ternary) -> Ternary {
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
        assert_eq!(meet_k3(True, True), True);
        assert_eq!(meet_k3(True, Unknown), Unknown);
        assert_eq!(meet_k3(True, False), False);
        assert_eq!(meet_k3(Unknown, True), Unknown);
        assert_eq!(meet_k3(Unknown, Unknown), Unknown);
        assert_eq!(meet_k3(Unknown, False), False);
        assert_eq!(meet_k3(False, True), False);
        assert_eq!(meet_k3(False, Unknown), False);
        assert_eq!(meet_k3(False, False), False);
    }

    // SPEC §7.5 OR truth table, all nine {T,F,U}^2 cells.
    #[test]
    fn k3_or_truth_table() {
        use Ternary::*;
        assert_eq!(join_k3(False, False), False);
        assert_eq!(join_k3(False, Unknown), Unknown);
        assert_eq!(join_k3(False, True), True);
        assert_eq!(join_k3(Unknown, False), Unknown);
        assert_eq!(join_k3(Unknown, Unknown), Unknown);
        assert_eq!(join_k3(Unknown, True), True);
        assert_eq!(join_k3(True, False), True);
        assert_eq!(join_k3(True, Unknown), True);
        assert_eq!(join_k3(True, True), True);
    }

    // SPEC §7.5 NOT truth table.
    #[test]
    fn k3_not_truth_table() {
        use Ternary::*;
        assert_eq!(involution_k3(True), False);
        assert_eq!(involution_k3(Unknown), Unknown);
        assert_eq!(involution_k3(False), True);
    }

    // SPEC §4.5.2: NIL takes priority over U when neither is absorbed.
    #[test]
    fn nil_takes_priority_over_unknown() {
        use Ternary::*;
        assert_eq!(meet_k3(Nil, Unknown), Nil);
        assert_eq!(meet_k3(Unknown, Nil), Nil);
        assert_eq!(join_k3(Nil, Unknown), Nil);
        assert_eq!(join_k3(Unknown, Nil), Nil);
        // but an absorbing definite still wins over NIL
        assert_eq!(meet_k3(False, Nil), False);
        assert_eq!(join_k3(True, Nil), True);
    }

    // Existing NIL semantics (SPEC §7.12) are unchanged by the U addition.
    #[test]
    fn nil_semantics_preserved() {
        use Ternary::*;
        assert_eq!(meet_k3(Nil, True), Nil);
        assert_eq!(meet_k3(Nil, False), False);
        assert_eq!(meet_k3(Nil, Nil), Nil);
        assert_eq!(join_k3(Nil, False), Nil);
        assert_eq!(join_k3(Nil, True), True);
        assert_eq!(join_k3(Nil, Nil), Nil);
        assert_eq!(involution_k3(Nil), Nil);
    }

    #[test]
    fn into_value_roundtrips_truth_value() {
        assert!(Ternary::Unknown.into_value().is_unknown());
        assert!(Ternary::Nil.into_value().is_nil() && !Ternary::Nil.into_value().is_unknown());
        assert!(Ternary::True.into_value().is_truth_value());
        assert!(Ternary::False.into_value().is_truth_value());
    }

    // --- P2: agreedPrefix diagnosis carry-over (SPEC §4.5.0 / §7.4.1) -------

    /// A U carrying an `agreedPrefix=k` comparison diagnosis, as produced by
    /// COMPARE-WITHIN / the comparison words via
    /// `Value::unknown_with_agreed_prefix`.
    fn u_with_prefix(k: usize) -> Value {
        Value::unknown_with_agreed_prefix(Some("COMPARE-WITHIN"), k)
    }

    fn agreed_prefix_of(v: &Value) -> Option<usize> {
        v.nil_diagnosis().and_then(|d| d.agreed_prefix)
    }

    /// `NOT` of a diagnosed U keeps the same `agreedPrefix`.
    #[test]
    fn not_preserves_agreed_prefix() {
        let input = u_with_prefix(7);
        let result = into_value_with_diagnosis(involution_k3(Ternary::classify(&input)), &[&input]);
        assert!(result.is_unknown());
        assert_eq!(result.truth_value(), Some("unknown"));
        assert_eq!(agreed_prefix_of(&result), Some(7));
    }

    /// `U(k) AND TRUE` is U and keeps `agreedPrefix=k` (the lone U operand).
    #[test]
    fn and_unknown_true_preserves_agreed_prefix() {
        let a = u_with_prefix(3);
        let b = t();
        let result = into_value_with_diagnosis(
            meet_k3(Ternary::classify(&a), Ternary::classify(&b)),
            &[&a, &b],
        );
        assert!(result.is_unknown());
        assert_eq!(agreed_prefix_of(&result), Some(3));
    }

    /// `U(k1) AND U(k2)` is U and keeps the left operand's `agreedPrefix=k1`.
    #[test]
    fn and_two_unknowns_keeps_left_diagnosis() {
        let a = u_with_prefix(11);
        let b = u_with_prefix(99);
        let result = into_value_with_diagnosis(
            meet_k3(Ternary::classify(&a), Ternary::classify(&b)),
            &[&a, &b],
        );
        assert!(result.is_unknown());
        assert_eq!(agreed_prefix_of(&result), Some(11));
    }

    /// `OR` mirrors `AND`: left-priority diagnosis carry-over.
    #[test]
    fn or_two_unknowns_keeps_left_diagnosis() {
        let a = u_with_prefix(5);
        let b = u_with_prefix(8);
        let result = into_value_with_diagnosis(
            join_k3(Ternary::classify(&a), Ternary::classify(&b)),
            &[&a, &b],
        );
        assert!(result.is_unknown());
        assert_eq!(agreed_prefix_of(&result), Some(5));
    }

    /// Bare U operands (no diagnosis) yield a bare U: the carry-over never
    /// fabricates an `agreedPrefix`.
    #[test]
    fn bare_unknowns_stay_bare() {
        let a = u();
        let b = u();
        let result = into_value_with_diagnosis(
            meet_k3(Ternary::classify(&a), Ternary::classify(&b)),
            &[&a, &b],
        );
        assert!(result.is_unknown());
        assert_eq!(agreed_prefix_of(&result), None);
    }

    /// Definite and NIL results are unchanged by the diagnosis wrapper, and
    /// no diagnosis leaks onto them: the observable `truthValue` is intact.
    #[test]
    fn definite_and_nil_results_unchanged() {
        let u_k = u_with_prefix(4);
        let ff = f();
        // U(k) AND FALSE = FALSE (F absorbs); no agreedPrefix carried.
        let result = into_value_with_diagnosis(
            meet_k3(Ternary::classify(&u_k), Ternary::classify(&ff)),
            &[&u_k, &ff],
        );
        assert_eq!(result.truth_value(), Some("false"));
        assert_eq!(agreed_prefix_of(&result), None);
        // U(k) AND NIL = NIL (NIL priority, SPEC §4.5.2); stays operational NIL.
        let nn = n();
        let result = into_value_with_diagnosis(
            meet_k3(Ternary::classify(&u_k), Ternary::classify(&nn)),
            &[&u_k, &nn],
        );
        assert!(result.is_nil() && !result.is_unknown());
    }

    /// End-to-end through the interpreter: an undecidable comparison emits a
    /// U with an `agreedPrefix`; `NOT` must keep that diagnosis (this pins
    /// the `logic.rs` wiring, not just the helper). `AND TRUE` likewise.
    #[tokio::test]
    async fn interpreter_not_and_preserve_agreed_prefix() {
        use crate::interpreter::Interpreter;
        // Comparison is total over Tier ≤ 1 (SPEC §7.4), so U is produced
        // through COMPARE-WITHIN against a Tier 2 starvation witness under
        // an explicit 8-step water budget; the U carries agreedPrefix = 8.
        async fn top(rest: &str) -> Value {
            use crate::types::exact::{Computable, ExactReal};
            let mut interp = Interpreter::new();
            interp
                .stack
                .push(Value::from_exact_real(ExactReal::Computable(
                    Computable::vanishing(),
                )));
            interp
                .execute(&format!("0 8 COMPARE-WITHIN {rest}"))
                .await
                .expect("executes");
            interp.get_stack().last().expect("nonempty").clone()
        }
        let base = top("").await;
        let k = agreed_prefix_of(&base).expect("U carries an agreedPrefix");

        let negated = top("NOT").await;
        assert!(negated.is_unknown());
        assert_eq!(
            agreed_prefix_of(&negated),
            Some(k),
            "NOT U must preserve agreedPrefix through op_not"
        );

        let anded = top("TRUE AND").await;
        assert!(anded.is_unknown());
        assert_eq!(
            agreed_prefix_of(&anded),
            Some(k),
            "U AND TRUE must preserve agreedPrefix through op_and"
        );
    }
}
