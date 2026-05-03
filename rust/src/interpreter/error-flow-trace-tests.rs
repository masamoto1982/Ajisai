use crate::error::{ErrorCategory, NilReason};
use crate::interpreter::error_flow_trace::ErrorFlowEventKind;
use crate::interpreter::Interpreter;

#[tokio::test]
async fn safe_caught_division_by_zero_has_three_question_diagnosis() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 ~ /").await.unwrap();

    let trace = interp.drain_error_flow_trace();
    let event = trace
        .iter()
        .find(|e| e.kind == ErrorFlowEventKind::SafeCaught)
        .expect("expected SafeCaught event");

    let diagnosis = event.diagnosis.as_ref().expect("expected diagnosis");

    assert_eq!(format!("{:?}", diagnosis.when), "SafeProjection");
    assert_eq!(format!("{:?}", diagnosis.why), "Domain");
    assert!(
        diagnosis.summary.contains("DivisionByZero") || diagnosis.summary.contains("division"),
        "summary should mention DivisionByZero, got: {}",
        diagnosis.summary
    );
    assert!(!diagnosis.next_checks.is_empty());
    assert_eq!(diagnosis.where_.word.as_deref(), Some("DIV"));
}

#[tokio::test]
async fn nil_produced_event_has_diagnosis() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 ~ /").await.unwrap();

    let trace = interp.drain_error_flow_trace();
    let event = trace
        .iter()
        .find(|e| e.kind == ErrorFlowEventKind::NilProduced)
        .expect("expected NilProduced event");

    let diagnosis = event.diagnosis.as_ref().expect("expected diagnosis");
    assert_eq!(format!("{:?}", diagnosis.when), "SafeProjection");
    assert_eq!(format!("{:?}", diagnosis.why), "Domain");
}

#[tokio::test]
async fn uncaught_word_error_has_execute_word_diagnosis() {
    let mut interp = Interpreter::new();
    let result = interp.execute("10 0 /").await;
    assert!(result.is_err());

    let trace = interp.drain_error_flow_trace();
    let event = trace
        .iter()
        .find(|e| e.kind == ErrorFlowEventKind::WordError)
        .expect("expected WordError event");

    let diagnosis = event.diagnosis.as_ref().expect("expected diagnosis");

    assert_eq!(format!("{:?}", diagnosis.when), "ExecuteWord");
    assert_eq!(format!("{:?}", diagnosis.why), "Domain");
    assert_eq!(diagnosis.where_.word.as_deref(), Some("DIV"));
}

#[tokio::test]
async fn stack_underflow_has_stack_shape_diagnosis() {
    let mut interp = Interpreter::new();
    let result = interp.execute("+").await;
    assert!(result.is_err());

    let trace = interp.drain_error_flow_trace();
    let event = trace
        .iter()
        .find(|e| e.kind == ErrorFlowEventKind::WordError)
        .expect("expected WordError event");

    let diagnosis = event.diagnosis.as_ref().expect("expected diagnosis");

    assert_eq!(format!("{:?}", diagnosis.why), "StackShape");
    assert!(!diagnosis.next_checks.is_empty());
}

#[tokio::test]
async fn safe_enter_and_success_have_no_diagnosis() {
    let mut interp = Interpreter::new();
    interp.execute("10 2 ~ /").await.unwrap();

    let trace = interp.drain_error_flow_trace();
    let enter = trace
        .iter()
        .find(|e| e.kind == ErrorFlowEventKind::SafeEnter)
        .expect("expected SafeEnter event");
    let success = trace
        .iter()
        .find(|e| e.kind == ErrorFlowEventKind::SafeSuccess)
        .expect("expected SafeSuccess event");
    assert!(enter.diagnosis.is_none());
    assert!(success.diagnosis.is_none());
}

#[tokio::test]
async fn error_flow_trace_records_safe_caught_division_by_zero() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 ~ /").await.unwrap();
    let trace = interp.drain_error_flow_trace();
    assert!(
        trace
            .iter()
            .any(|e| e.kind == ErrorFlowEventKind::SafeEnter),
        "expected SafeEnter event, got {:?}",
        trace
    );
    assert!(
        trace
            .iter()
            .any(|e| e.kind == ErrorFlowEventKind::SafeCaught
                && e.word.as_deref() == Some("DIV")
                && e.error_category == Some(ErrorCategory::DivisionByZero)),
        "expected SafeCaught(DIV, DivisionByZero), got {:?}",
        trace
    );
    assert!(
        trace
            .iter()
            .any(|e| e.kind == ErrorFlowEventKind::NilProduced
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
        trace
            .iter()
            .any(|e| e.kind == ErrorFlowEventKind::SafeSuccess && e.word.as_deref() == Some("DIV")),
        "expected SafeSuccess(DIV), got {:?}",
        trace
    );
    assert!(
        !trace
            .iter()
            .any(|e| e.kind == ErrorFlowEventKind::SafeCaught),
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
