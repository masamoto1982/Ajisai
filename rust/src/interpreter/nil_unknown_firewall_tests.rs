//! Firewall between the logical Unknown (U) and operational NIL in the
//! generic passthrough helpers (SPEC §7.5 / §2.3).
//!
//! `nil_passthrough_unary/value/binary` now key off `is_operational_nil()`
//! rather than `is_nil()` (which is a storage predicate that also matches
//! U). This stops U from being silently absorbed as an operational NIL when
//! it reaches an arithmetic/generic Coreword that uses those helpers, while
//! leaving genuine operational NIL passthrough intact.
//!
//! Observed behavior of `U 1 +` (recorded here per the task's request to
//! observe and report what U does when it reaches an arithmetic broadcast):
//!   - BEFORE the fix: `nil_passthrough_binary` matched U via `is_nil()` and
//!     absorbed it into an operational NIL node (`Ok(Nil)`, hint demoted
//!     `TruthValue` -> `Nil`). U was silently collapsed to an operational
//!     absence even though it is a logical truth value.
//!   - AFTER the fix: passthrough no longer matches U, so U reaches
//!     `op_add`'s broadcast guard (`apply_binary_broadcast_with_metrics`,
//!     left unchanged per task Step 3), which raises an explicit error
//!     "Cannot broadcast NIL values". U is NOT collapsed into a reasonless
//!     operational NIL.
//!
//! Either an explicit error or preserved-U is acceptable per the acceptance
//! criteria; the firewall guarantee is that U is never silently turned into a
//! reasonless operational NIL. Since CS4, U is a distinct `ValueData::Unknown`
//! variant (not a NIL node carrying a reason), so the guarantee is a type
//! invariant rather than a reason-check.

use crate::error::NilReason;
use crate::interpreter::Interpreter;
use crate::types::Value;

/// The logical Unknown (U), produced authentically through a comparison.
/// Comparison is total over Tier ≤ 1 (everything the current vocabulary
/// constructs, SPEC §7.4), so U comes from `COMPARE-WITHIN` against a
/// **Tier 2** observation — a type-level starvation witness no word can
/// build yet — exhausting the explicit 8-step water budget; the U carries
/// an agreedPrefix.
async fn u_value() -> Value {
    use crate::types::exact::{Computable, ExactReal};
    let mut interp = Interpreter::new();
    interp
        .stack
        .push(Value::from_exact_real(ExactReal::Computable(
            Computable::vanishing(),
        )));
    interp
        .execute("0 8 COMPARE-WITHIN")
        .await
        .expect("COMPARE-WITHIN executes");
    interp.get_stack().last().expect("U on stack").clone()
}

/// Run `code` with `preload` already on the stack (bottom first).
async fn run_with(preload: Vec<Value>, code: &str) -> Result<Vec<Value>, String> {
    let mut interp = Interpreter::new();
    for v in preload {
        interp.stack.push(v);
    }
    interp.execute(code).await.map_err(|e| e.to_string())?;
    Ok(interp.get_stack().to_vec())
}

async fn run(code: &str) -> Result<Vec<Value>, String> {
    run_with(Vec::new(), code).await
}

async fn run_ok(code: &str) -> Vec<Value> {
    run(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` unexpectedly errored: {e}"))
}

/// Passing U to an arithmetic word must not silently collapse it into a
/// reasonless operational NIL. The firewall holds if EITHER an explicit
/// error is raised OR `is_unknown()` is preserved; what must never happen is
/// a successful result that is an operational NIL with U's identity erased.
#[tokio::test]
async fn u_into_arithmetic_does_not_silently_collapse_to_reasonless_nil() {
    let res = run_with(vec![u_value().await], "1 +").await;
    match res {
        Err(_) => {
            // Acceptable: U reaches the broadcast guard and errors explicitly
            // (the guard is intentionally left unchanged in this task).
        }
        Ok(stack) => {
            assert_eq!(stack.len(), 1);
            let top = &stack[0];
            assert!(
                top.is_unknown(),
                "U passed to + must preserve is_unknown() if it does not \
                 error; got {top:?}"
            );
            assert!(
                !top.is_operational_nil(),
                "U must never collapse to a reasonless operational NIL; got {top:?}"
            );
        }
    }
}

/// A genuine operational NIL with a reason still passes through arithmetic
/// unchanged (no regression): the reason is preserved and the result is an
/// operational NIL (not U).
#[tokio::test]
async fn operational_nil_passthrough_preserves_reason() {
    // `1 0 DIV` is a recoverable DivisionByZero Bubble (operational NIL).
    let nil_src = "1 0 DIV";
    let stack = run_ok(&format!("{nil_src} 1 +")).await;
    assert_eq!(stack.len(), 1);
    let top = &stack[0];
    assert!(
        top.is_operational_nil(),
        "NIL 1 + must stay an operational NIL, got {top:?}"
    );
    assert_eq!(
        top.nil_reason().cloned(),
        Some(NilReason::DivisionByZero),
        "operational NIL must keep its reason through arithmetic passthrough, got {top:?}"
    );
}

/// CS4 predicate contract: the logical Unknown (U) is a distinct
/// `ValueData::Unknown` variant, so the U/NIL split is a type invariant, not
/// a predicate convention. A freshly constructed U must satisfy the split
/// exactly, and a genuine operational NIL must remain outside it.
#[test]
fn unknown_variant_satisfies_type_level_firewall() {
    let u = Value::unknown();
    assert!(u.is_unknown(), "unknown() must be U");
    assert!(!u.is_nil(), "U is not NIL (type-level split)");
    assert!(!u.is_absent(), "U is not an operational absence");
    assert!(!u.is_operational_nil(), "U is not an operational NIL");
    assert_eq!(u.nil_reason(), None, "U carries no NIL reason");
    assert!(
        u.absence_metadata().is_none(),
        "U carries no NIL absence metadata"
    );

    let nil = Value::nil();
    assert!(!nil.is_unknown(), "NIL is not U");
    assert!(nil.is_nil(), "NIL is NIL");
    assert!(nil.is_operational_nil(), "NIL is an operational NIL");
}

/// CS4: U's CF-comparison `agreedPrefix` diagnosis survives the variant split,
/// surfaced through `nil_diagnosis()` (U's own carrier, not NIL metadata),
/// while a bare U carries no diagnosis.
#[test]
fn unknown_agreed_prefix_diagnosis_survives_on_its_own_carrier() {
    let diagnosed = Value::unknown_with_agreed_prefix(Some("COMPARE-WITHIN"), 5);
    assert!(diagnosed.is_unknown());
    assert_eq!(
        diagnosed.nil_diagnosis().and_then(|d| d.agreed_prefix),
        Some(5),
        "agreedPrefix must survive on U's own diagnostic carrier"
    );
    assert!(
        diagnosed.absence_metadata().is_none(),
        "the diagnosis must not be carried as NIL absence metadata"
    );

    let bare = Value::unknown();
    assert!(
        bare.nil_diagnosis().is_none(),
        "a bare U carries no diagnosis"
    );
}

/// CS4 PR-2: U is classified as a scalar **truth value**, not an operational
/// absence, on the observation axes — distinct from NIL at every point the
/// audit split them.
#[test]
fn unknown_is_classified_as_a_truth_value_not_an_absence() {
    use crate::semantic::{Capability, SemanticKind, ValueShape};
    use crate::types::Interpretation;

    let u = Value::unknown();
    let nil = Value::nil();

    // Coarse kind / shape: U reports like a Boolean (number / scalar), NIL
    // reports absence.
    assert_eq!(u.semantic_kind(), SemanticKind::Number);
    assert_eq!(u.shape_kind(), ValueShape::Scalar);
    assert_eq!(nil.semantic_kind(), SemanticKind::Absence);
    assert_eq!(nil.shape_kind(), ValueShape::Absence);

    // Capabilities: U is truth-valued, diagnosable and AI-explainable, but
    // must NOT advertise NIL-passthrough (that would contradict the firewall).
    assert!(u.has_capability(Capability::TruthValued));
    assert!(u.has_capability(Capability::Diagnosable));
    assert!(u.has_capability(Capability::AiExplainable));
    assert!(
        !u.has_capability(Capability::NilPassthrough),
        "U must not advertise NilPassthrough"
    );
    assert!(nil.has_capability(Capability::NilPassthrough));

    // A single scalar truth value: length 1 (like a Boolean), rank 0, not
    // indexable; NIL is empty.
    assert_eq!(u.len(), 1);
    assert!(!u.is_empty());
    assert!(u.child(0).is_none(), "U is not indexable");
    assert_eq!(u.shape(), Vec::<usize>::new());
    assert_eq!(nil.len(), 0);

    // Default role is TruthValue, never Nil.
    assert_eq!(u.resolve_default_hint(), Interpretation::TruthValue);

    // U is not numeric content: it contributes no fraction lane (like a
    // Boolean), whereas NIL contributes a nil lane.
    assert_eq!(u.count_fractions(), 0);
    assert!(u.collect_fractions_flat().is_empty());
    assert_eq!(nil.count_fractions(), 1);
}

/// CS4 PR-2: U always renders as `UNKNOWN`, even nested inside a non-truth
/// collection, and never as `NIL`.
#[test]
fn unknown_renders_as_unknown_even_when_nested() {
    let nested = Value::from_children(vec![
        Value::from_int(1),
        Value::unknown(),
        Value::from_int(3),
    ]);
    let rendered = format!("{nested}");
    assert!(
        rendered.contains("UNKNOWN"),
        "nested U must render as UNKNOWN, got {rendered}"
    );
    assert!(
        !rendered.contains("NIL"),
        "nested U must never render as NIL, got {rendered}"
    );
}

/// The K3 logic path (`logic_kleene`) is unaffected: `U U AND` is U.
/// (Ajisai has no `DUP`; two independently-produced U operands stand in for
/// the task's `U DUP AND`, exercising the same `Unknown AND Unknown` cell.)
#[tokio::test]
async fn u_and_u_is_unknown() {
    let stack = run_with(vec![u_value().await, u_value().await], "AND")
        .await
        .unwrap_or_else(|e| panic!("`U U AND` unexpectedly errored: {e}"));
    assert_eq!(stack.len(), 1);
    assert!(
        stack[0].is_unknown(),
        "U U AND must remain Unknown, got {:?}",
        stack[0]
    );
}
