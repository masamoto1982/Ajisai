//! Test suite for NIL reason metadata.

use crate::error::NilReason;
use crate::interpreter::Interpreter;
use crate::semantic::{AbsenceMetadata, AbsenceOrigin, Recoverability};
use crate::types::Value;

fn last_nil_reason(interp: &Interpreter) -> Option<NilReason> {
    interp
        .get_stack()
        .last()
        .and_then(|v| v.nil_reason().cloned())
}

#[tokio::test]
async fn division_by_zero_preserves_direct_bubble_reason() {
    let mut interp = Interpreter::new();
    interp.execute("1 0 /").await.unwrap();
    let stack = interp.get_stack();
    assert!(
        stack.last().map(|v| v.is_nil()).unwrap_or(false),
        "top of stack must be NIL after division by zero"
    );
    let reason = last_nil_reason(&interp).expect("Bubble/NIL must carry a reason");
    assert_eq!(reason, NilReason::DivisionByZero);
}

#[tokio::test]
async fn index_out_of_bounds_preserves_direct_bubble_reason() {
    let mut interp = Interpreter::new();
    interp.execute("[ 10 20 ] [ 99 ] GET").await.unwrap();
    let stack = interp.get_stack();
    assert!(
        stack.last().map(|v| v.is_nil()).unwrap_or(false),
        "top of stack must be NIL after out-of-bounds GET"
    );
    let reason = last_nil_reason(&interp).expect("Bubble/NIL must carry a reason");
    assert_eq!(reason, NilReason::IndexOutOfBounds);
}

#[tokio::test]
async fn unknown_word_propagates_error() {
    let mut interp = Interpreter::new();
    let result = interp.execute("__NO_SUCH_WORD__").await;
    assert!(
        result.is_err(),
        "an unknown word must propagate its error, not project to NIL"
    );
}

#[tokio::test]
async fn successful_bubble_uses_normal_word_stack_effect() {
    let mut interp = Interpreter::new();
    interp.execute("1 2 3 0 /").await.unwrap();
    let stack = interp.get_stack();
    assert_eq!(
        stack.len(),
        3,
        "DIV consumes its two operands and pushes a single Bubble/NIL result"
    );
    assert_eq!(format!("{}", stack[0]), "1/1");
    assert_eq!(format!("{}", stack[1]), "2/1");
    assert!(stack[2].is_nil());
    assert_eq!(stack[2].nil_reason(), Some(&NilReason::DivisionByZero));
}

#[tokio::test]
async fn nil_passthrough_preserves_reason_through_arithmetic_pipeline() {
    let mut interp = Interpreter::new();
    interp.execute("1 0 /").await.unwrap();
    interp.execute("10 +").await.unwrap();
    interp.execute("2 *").await.unwrap();
    assert!(
        interp
            .get_stack()
            .last()
            .map(|v| v.is_nil())
            .unwrap_or(false),
        "top of stack should still be NIL after passthrough pipeline"
    );
    let reason = last_nil_reason(&interp).expect("reason must propagate through passthrough");
    assert_eq!(reason, NilReason::DivisionByZero);
}

#[tokio::test]
async fn nil_passthrough_preserves_full_absence_metadata() {
    let mut interp = Interpreter::new();
    let nil = Value::nil_with_absence(AbsenceMetadata::with_reason(
        NilReason::ExecutionFailure,
        AbsenceOrigin::HostEnvironment,
        Recoverability::Retryable,
    ));
    interp.update_stack(vec![nil, Value::from_int(10)]);

    interp.execute("+").await.unwrap();

    let absence = interp.get_stack()[0]
        .absence_metadata()
        .expect("passthrough NIL keeps absence metadata");
    assert_eq!(absence.reason, Some(NilReason::ExecutionFailure));
    assert_eq!(absence.origin, AbsenceOrigin::HostEnvironment);
    assert_eq!(absence.recoverability, Recoverability::Retryable);
}
/// A written NIL carries the reason `literal`.
///
/// It used to carry none, which made it the counterexample to
/// `LANG.VALUES.NIL` ("the reason is the entire observable content of a NIL"):
/// the value had no observable content at all, and `NIL NIL-REASON` answered
/// NIL (audit finding D24). Nothing failed to produce it, so the reason names
/// the only thing true of it — it was written down.
#[tokio::test]
async fn bare_nil_literal_is_reasoned_as_literal() {
    let mut interp = Interpreter::new();
    interp.execute("NIL").await.unwrap();
    let stack = interp.get_stack();
    assert_eq!(stack.len(), 1);
    assert!(stack[0].is_nil());
    assert_eq!(
        stack[0].nil_reason(),
        Some(&NilReason::Literal),
        "a written NIL must carry the `literal` reason"
    );
}

#[tokio::test]
async fn or_nil_consumes_direct_bubble_nil_and_substitutes_fallback() {
    let mut interp = Interpreter::new();
    interp.execute("1 0 /").await.unwrap();
    interp.execute("42 ^").await.unwrap();
    let stack = interp.get_stack();
    assert!(
        !stack.last().unwrap().is_nil(),
        "top should not be NIL after VENT fallback"
    );
    assert_eq!(format!("{}", stack.last().unwrap()), "42/1");
}

mod division_bubble_rule {
    use super::*;

    #[tokio::test]
    async fn left_nil_propagates_to_nil() {
        let mut interp = Interpreter::new();
        interp.execute("NIL 5 /").await.unwrap();
        assert!(interp.get_stack().last().unwrap().is_nil());
    }

    #[tokio::test]
    async fn right_nil_propagates_to_nil() {
        let mut interp = Interpreter::new();
        interp.execute("5 NIL /").await.unwrap();
        assert!(interp.get_stack().last().unwrap().is_nil());
    }

    #[tokio::test]
    async fn right_zero_produces_nil_with_division_by_zero() {
        let mut interp = Interpreter::new();
        interp.execute("5 0 /").await.unwrap();
        let reason = last_nil_reason(&interp).expect("must carry reason");
        assert_eq!(reason, NilReason::DivisionByZero);
    }

    #[tokio::test]
    async fn all_valid_succeeds_and_no_reason() {
        let mut interp = Interpreter::new();
        interp.execute("10 2 /").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(!stack[0].is_nil());
        assert_eq!(format!("{}", stack[0]), "5/1");
        assert!(stack[0].nil_reason().is_none());
    }
}

#[tokio::test]
async fn bubble_rule_division_by_zero_without_safe_has_direct_reason() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 /").await.unwrap();
    let top = interp.get_stack().last().expect("top value");
    assert!(top.is_nil());
    assert_eq!(top.nil_reason(), Some(&NilReason::DivisionByZero));
}

#[tokio::test]
async fn bubble_rule_division_by_zero_recovers_with_or_nil() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 / ^ 99").await.unwrap();
    let top = interp.get_stack().last().expect("top value");
    assert!(!top.is_nil());
    assert_eq!(format!("{}", top), "99/1");
}

#[tokio::test]
async fn bubble_rule_get_out_of_range_without_safe_has_direct_reason() {
    let mut interp = Interpreter::new();
    interp.execute("[ 10 20 ] [ 99 ] GET").await.unwrap();
    let top = interp.get_stack().last().expect("top value");
    assert!(top.is_nil());
    assert_eq!(top.nil_reason(), Some(&NilReason::IndexOutOfBounds));
}

#[tokio::test]
async fn bubble_rule_get_out_of_range_recovers_with_or_nil() {
    let mut interp = Interpreter::new();
    interp.execute("[ 10 20 ] [ 99 ] GET ^ 0").await.unwrap();
    let top = interp.get_stack().last().expect("top value");
    assert!(!top.is_nil());
    assert_eq!(format!("{}", top), "0/1");
}

#[tokio::test]
async fn bubble_rule_contract_violations_remain_errors() {
    let mut interp = Interpreter::new();
    assert!(interp.execute("10 'x' /").await.is_err());

    let mut interp = Interpreter::new();
    assert!(interp.execute("123 [ 0 ] GET").await.is_err());

    let mut interp = Interpreter::new();
    assert!(interp.execute("[ 10 20 ] 'x' GET").await.is_err());
}

#[tokio::test]
async fn bubble_rule_num_parse_failure_has_direct_reason_and_fallback() {
    let mut interp = Interpreter::new();
    interp.execute("'abc' NUM").await.unwrap();
    let top = interp.get_stack().last().expect("top value");
    assert!(top.is_nil());
    assert_eq!(top.nil_reason(), Some(&NilReason::InvalidEncoding));

    let mut interp = Interpreter::new();
    interp.execute("'abc' NUM ^ 0").await.unwrap();
    assert_eq!(format!("{}", interp.get_stack().last().unwrap()), "0/1");
}

/// **Every NIL a program can put on the stack carries a reason.**
///
/// `LANG.VALUES.NIL` makes the reason a NIL's entire observable content — "two
/// NILs with the same reason are the same value" — so a reasonless NIL is a
/// value with no observable content, indistinguishable from every other
/// reasonless NIL while claiming to be a distinct absence. Several were
/// reachable: the written `NIL` literal itself, every emptiness (`FILTER` that
/// keeps nothing, a zero-length `TAKE`, a `REMOVE` that empties its vector, a
/// zero-sized `SPLIT` chunk), and `MAP`/`FILTER` over a NIL target, which
/// minted a fresh blank absence instead of passing the incoming reason along
/// (audit finding D24).
///
/// This walks one program per production route rather than trusting the call
/// sites, because the defect was never in one place — it was `Value::nil()`
/// being the convenient constructor everywhere.
#[tokio::test]
async fn every_reachable_nil_carries_a_reason() {
    // (program, the reason its NIL must carry)
    let cases: &[(&str, NilReason)] = &[
        // Written, not computed.
        ("NIL", NilReason::Literal),
        // Emptiness used to be here, by four routes (FILTER, TAKE, REMOVE,
        // `''`). None of them reaches NIL now: an empty result is an empty
        // Vector and `''` is a String, so `EmptySequence` has nothing left to
        // describe.
        // Projections that already had reasons — pinned so the sweep is a
        // statement about all NILs, not only the ones this change touched.
        ("1 0 /", NilReason::DivisionByZero),
        ("-1 SQRT", NilReason::DomainMiss),
        ("'abc' NUM", NilReason::InvalidEncoding),
        // A NIL target keeps its reason through the higher-order Words rather
        // than being replaced by a blank one.
        ("1 0 / [ 1 ] MAP", NilReason::DivisionByZero),
        ("1 0 / [ TRUE ] FILTER", NilReason::DivisionByZero),
        ("1 0 / STR", NilReason::DivisionByZero),
    ];

    for (code, expected) in cases {
        let mut interp = Interpreter::new();
        interp
            .execute(code)
            .await
            .unwrap_or_else(|e| panic!("`{code}` must not error: {e}"));
        let result = interp.stack.last().expect("a result value");
        assert!(result.is_nil(), "`{code}` must answer NIL, got {result:?}");
        assert_eq!(
            result.nil_reason(),
            Some(expected),
            "`{code}` carries the wrong reason"
        );
    }

    // The sweep above names each reason; this one is the invariant itself, over
    // every NIL the program leaves anywhere — nested inside a vector included,
    // which is how a written `NIL` reaches the stack without being the result.
    fn assert_all_nils_reasoned(value: &Value, code: &str) {
        if value.is_nil() {
            assert!(
                value.nil_reason().is_some(),
                "`{code}` left a reasonless NIL: {value:?}"
            );
        }
        if let Some(children) = value.as_vector_view() {
            for child in children.iter() {
                assert_all_nils_reasoned(child, code);
            }
        }
    }

    for code in [
        "NIL NIL",
        "1 0 / -1 SQRT",
        "[ 1 NIL 2 ]",
        "[ 1 NIL 2 ] [ 1 ] GET",
        "[ 1 2 3 ] [ 9 ] GET",
    ] {
        let mut interp = Interpreter::new();
        interp
            .execute(code)
            .await
            .unwrap_or_else(|e| panic!("`{code}` must not error: {e}"));
        for value in interp.stack.iter() {
            assert_all_nils_reasoned(value, code);
        }
    }
}
