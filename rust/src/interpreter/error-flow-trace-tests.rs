use crate::error::{ErrorCategory, NilReason};
use crate::interpreter::error_flow_trace::ErrorFlowEventKind;
use crate::interpreter::Interpreter;

#[tokio::test]
async fn error_flow_trace_records_safe_caught_division_by_zero() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 ~ /").await.unwrap();
    let trace = interp.drain_error_flow_trace();
    assert!(
        trace.iter().any(|e| e.kind == ErrorFlowEventKind::SafeEnter),
        "expected SafeEnter event, got {:?}",
        trace
    );
    assert!(
        trace.iter().any(|e| e.kind == ErrorFlowEventKind::SafeCaught
            && e.word.as_deref() == Some("DIV")
            && e.error_category == Some(ErrorCategory::DivisionByZero)),
        "expected SafeCaught(DIV, DivisionByZero), got {:?}",
        trace
    );
    assert!(
        trace.iter().any(|e| e.kind == ErrorFlowEventKind::NilProduced
            && matches!(e.nil_reason, Some(NilReason::SafeCaught(_)))),
        "expected NilProduced with SafeCaught reason, got {:?}",
        trace
    );
}

#[tokio::test]
async fn error_flow_trace_records_safe_success() {
    let mut interp = Interpreter::new();
    interp.execute("10 2 ~ /").await.unwrap();
    let trace = interp.drain_error_flow_trace();
    assert!(
        trace.iter().any(|e| e.kind == ErrorFlowEventKind::SafeSuccess
            && e.word.as_deref() == Some("DIV")),
        "expected SafeSuccess(DIV), got {:?}",
        trace
    );
    assert!(
        !trace.iter().any(|e| e.kind == ErrorFlowEventKind::SafeCaught),
        "should not record SafeCaught on success, got {:?}",
        trace
    );
}

#[tokio::test]
async fn error_flow_trace_records_uncaught_word_error() {
    let mut interp = Interpreter::new();
    let result = interp.execute("10 0 /").await;
    assert!(result.is_err());
    let trace = interp.drain_error_flow_trace();
    assert!(
        trace.iter().any(|e| e.kind == ErrorFlowEventKind::WordError
            && e.word.as_deref() == Some("DIV")
            && e.error_category == Some(ErrorCategory::DivisionByZero)),
        "expected WordError(DIV, DivisionByZero), got {:?}",
        trace
    );
}

#[tokio::test]
async fn error_flow_trace_drain_clears_log() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 ~ /").await.unwrap();
    let first = interp.drain_error_flow_trace();
    assert!(!first.is_empty());
    let second = interp.drain_error_flow_trace();
    assert!(second.is_empty());
}

#[tokio::test]
async fn safe_semantics_are_unchanged() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 ~ /").await.unwrap();
    let stack = interp.get_stack();
    assert_eq!(
        stack.len(),
        3,
        "stack after `10 0 ~ /` should be [10, 0, NIL] (snapshot restored + NIL)"
    );
    let top = stack.last().unwrap();
    assert!(top.is_nil());
    let reason = top.nil_reason().cloned();
    assert!(matches!(reason, Some(NilReason::SafeCaught(_))));
}
