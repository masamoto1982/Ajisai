use crate::error::{ErrorCategory, NilReason};
use crate::interpreter::Interpreter;

fn last_nil_reason(interp: &Interpreter) -> Option<NilReason> {
    interp.get_stack().last().and_then(|v| v.nil_reason().cloned())
}

#[tokio::test]
async fn safe_division_by_zero_creates_nil_with_safe_caught_division_by_zero() {
    let mut interp = Interpreter::new();
    interp.execute("1 0 ~ /").await.unwrap();
    let stack = interp.get_stack();
    assert!(
        stack.last().map(|v| v.is_nil()).unwrap_or(false),
        "top of stack must be NIL after safe-guarded division by zero"
    );
    let reason = last_nil_reason(&interp).expect("safe-projected NIL must carry a reason");
    match reason {
        NilReason::SafeCaught(category) => assert_eq!(*category, ErrorCategory::DivisionByZero),
        other => panic!("expected SafeCaught(DivisionByZero), got {:?}", other),
    }
}

#[tokio::test]
async fn safe_index_out_of_bounds_creates_nil_with_safe_caught_index_out_of_bounds() {
    let mut interp = Interpreter::new();
    interp.execute("[ 10 20 ] [ 99 ] ~ GET").await.unwrap();
    let stack = interp.get_stack();
    assert!(
        stack.last().map(|v| v.is_nil()).unwrap_or(false),
        "top of stack must be NIL after safe-guarded out-of-bounds GET"
    );
    let reason = last_nil_reason(&interp).expect("safe-projected NIL must carry a reason");
    match reason {
        NilReason::SafeCaught(category) => assert_eq!(*category, ErrorCategory::IndexOutOfBounds),
        other => panic!("expected SafeCaught(IndexOutOfBounds), got {:?}", other),
    }
}

#[tokio::test]
async fn safe_unknown_word_creates_nil_with_safe_caught_unknown_word() {
    let mut interp = Interpreter::new();
    interp.execute("~ __NO_SUCH_WORD__").await.unwrap();
    let stack = interp.get_stack();
    assert_eq!(stack.len(), 1);
    assert!(stack[0].is_nil());
    let reason = last_nil_reason(&interp).expect("safe-projected NIL must carry a reason");
    match reason {
        NilReason::SafeCaught(category) => assert_eq!(*category, ErrorCategory::UnknownWord),
        other => panic!("expected SafeCaught(UnknownWord), got {:?}", other),
    }
}

#[tokio::test]
async fn safe_failure_restores_stack_to_pre_call_snapshot() {
    let mut interp = Interpreter::new();
    interp.execute("1 2 3 0 ~ /").await.unwrap();
    let stack = interp.get_stack();
    assert_eq!(
        stack.len(),
        5,
        "stack restored to pre-call snapshot (4 operands) plus a single NIL"
    );
    assert_eq!(format!("{}", stack[0]), "1");
    assert_eq!(format!("{}", stack[1]), "2");
    assert_eq!(format!("{}", stack[2]), "3");
    assert_eq!(format!("{}", stack[3]), "0");
    assert!(stack[4].is_nil());
}

#[tokio::test]
async fn nil_passthrough_preserves_reason_through_arithmetic_pipeline() {
    let mut interp = Interpreter::new();
    interp.execute("1 0 ~ /").await.unwrap();
    interp.execute(", 10 +").await.unwrap();
    interp.execute(", 2 *").await.unwrap();
    assert!(
        interp.get_stack().last().map(|v| v.is_nil()).unwrap_or(false),
        "top of stack should still be NIL after passthrough pipeline"
    );
    let reason = last_nil_reason(&interp).expect("reason must propagate through passthrough");
    match reason {
        NilReason::SafeCaught(category) => assert_eq!(*category, ErrorCategory::DivisionByZero),
        other => panic!("expected reason to survive passthrough, got {:?}", other),
    }
}

#[tokio::test]
async fn bare_nil_literal_has_no_reason() {
    let mut interp = Interpreter::new();
    interp.execute("NIL").await.unwrap();
    let stack = interp.get_stack();
    assert_eq!(stack.len(), 1);
    assert!(stack[0].is_nil());
    assert!(
        stack[0].nil_reason().is_none(),
        "a NIL literal must not carry a reason"
    );
}

#[tokio::test]
async fn or_nil_consumes_safe_caught_nil_and_substitutes_fallback() {
    let mut interp = Interpreter::new();
    interp.execute("1 0 ~ /").await.unwrap();
    interp.execute("42 =>").await.unwrap();
    let stack = interp.get_stack();
    assert!(
        !stack.last().unwrap().is_nil(),
        "top should not be NIL after OR-NIL fallback"
    );
    assert_eq!(format!("{}", stack.last().unwrap()), "42");
}

mod mcdc_safe_division {
    use super::*;

    #[tokio::test]
    async fn aq_ver_safe_div_a_left_nil_safe_engaged_projects_to_nil() {
        let mut interp = Interpreter::new();
        interp.execute("NIL 5 ~ /").await.unwrap();
        assert!(interp.get_stack().last().unwrap().is_nil());
    }

    #[tokio::test]
    async fn aq_ver_safe_div_b_right_nil_safe_engaged_projects_to_nil() {
        let mut interp = Interpreter::new();
        interp.execute("5 NIL ~ /").await.unwrap();
        assert!(interp.get_stack().last().unwrap().is_nil());
    }

    #[tokio::test]
    async fn aq_ver_safe_div_c_right_zero_safe_engaged_projects_to_nil_with_division_by_zero() {
        let mut interp = Interpreter::new();
        interp.execute("5 0 ~ /").await.unwrap();
        let reason = last_nil_reason(&interp).expect("must carry reason");
        match reason {
            NilReason::SafeCaught(c) => assert_eq!(*c, ErrorCategory::DivisionByZero),
            other => panic!("expected DivisionByZero, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn aq_ver_safe_div_d_safe_disengaged_zero_propagates_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("5 0 /").await;
        assert!(
            result.is_err(),
            "without SAFE, division by zero must propagate as an error"
        );
    }

    #[tokio::test]
    async fn aq_ver_safe_div_e_all_valid_safe_engaged_succeeds_and_no_reason() {
        let mut interp = Interpreter::new();
        interp.execute("10 2 ~ /").await.unwrap();
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 1);
        assert!(!stack[0].is_nil());
        assert_eq!(format!("{}", stack[0]), "5");
        assert!(stack[0].nil_reason().is_none());
    }
}
