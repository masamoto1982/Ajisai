//! Test suite for `crate::interpreter::error_flow_trace`.

use crate::error::{ErrorCategory, NilReason};
use crate::interpreter::error_flow_trace::ErrorFlowEventKind;
use crate::interpreter::Interpreter;

#[tokio::test]
async fn nil_produced_event_has_execute_word_diagnosis() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 /").await.unwrap();

    let trace = interp.drain_error_flow_trace();
    let event = trace
        .iter()
        .find(|e| e.kind == ErrorFlowEventKind::NilProduced)
        .expect("expected NilProduced event");

    let diagnosis = event.diagnosis.as_ref().expect("expected diagnosis");
    assert_eq!(diagnosis.when.as_protocol_str(), "executeWord");
    assert_eq!(diagnosis.why.as_protocol_str(), "domain");
    assert_eq!(
        event.absence.as_ref().and_then(|a| a.reason.as_ref()),
        Some(&NilReason::DivisionByZero)
    );
}

#[tokio::test]
async fn bubble_produced_by_word_has_execute_word_diagnosis() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 /").await.unwrap();

    let trace = interp.drain_error_flow_trace();
    let event = trace
        .iter()
        .find(|e| e.kind == ErrorFlowEventKind::NilProduced)
        .expect("expected NilProduced event");

    let diagnosis = event.diagnosis.as_ref().expect("expected diagnosis");

    assert_eq!(diagnosis.when.as_protocol_str(), "executeWord");
    assert_eq!(diagnosis.why.as_protocol_str(), "domain");
    assert_eq!(diagnosis.where_.word.as_deref(), Some("DIV"));
    assert_eq!(
        event.absence.as_ref().and_then(|a| a.reason.as_ref()),
        Some(&NilReason::DivisionByZero)
    );
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

    assert_eq!(diagnosis.why.as_protocol_str(), "stackShape");
    assert!(!diagnosis.next_checks.is_empty());
}

#[tokio::test]
async fn nil_produced_event_carries_structured_absence_protocol_metadata() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 /").await.unwrap();

    let trace = interp.drain_error_flow_trace();
    let event = trace
        .iter()
        .find(|e| e.kind == ErrorFlowEventKind::NilProduced)
        .expect("expected NilProduced event");
    let absence = event
        .absence
        .as_ref()
        .expect("NilProduced event must carry absence metadata");
    let reason = absence.reason.as_ref().expect("Bubble/NIL has a reason");

    assert_eq!(event.kind.as_protocol_str(), "nilProduced");
    assert_eq!(reason.as_protocol_str(), "divisionByZero");
    assert_eq!(absence.origin.as_protocol_str(), "executionFailure");
    assert_eq!(absence.recoverability.as_protocol_str(), "recoverable");
    assert!(absence.diagnosis.is_none());
}

#[tokio::test]
async fn nil_produced_event_exposes_ai_structured_diagnosis_payload() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 /").await.unwrap();

    let trace = interp.drain_error_flow_trace();
    let event = trace
        .iter()
        .find(|e| e.kind == ErrorFlowEventKind::NilProduced)
        .expect("expected NilProduced event");

    let payload = event
        .ai_diagnostic_payload()
        .expect("NilProduced event should expose AI diagnostic payload");
    assert_eq!(payload.kind.as_deref(), Some("divisionByZero"));
    assert_eq!(payload.recoverability, "fixInput");
    assert_eq!(payload.semantic_area, "exact-real-arithmetic");
    assert_eq!(payload.word.as_deref(), Some("DIV"));
    assert_eq!(payload.semantic_role, "Derived");
    assert_eq!(payload.algebraic_family, "exact-arithmetic");
    assert_eq!(payload.nil_reason.as_deref(), Some("divisionByZero"));
    assert!(payload.truth_value.is_none());
    assert!(payload.effect.is_none());
    assert!(payload
        .next_checks
        .iter()
        .any(|check| check.label == "Check divisor"));
}

#[tokio::test]
async fn error_flow_trace_records_direct_bubble_from_word() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 /").await.unwrap();
    let trace = interp.drain_error_flow_trace();
    assert!(
        trace
            .iter()
            .any(|e| e.kind == ErrorFlowEventKind::NilProduced
                && e.word.as_deref() == Some("DIV")
                && e.error_category == Some(ErrorCategory::DivisionByZero)),
        "expected NilProduced(DIV, DivisionByZero), got {:?}",
        trace
    );
}

#[tokio::test]
async fn error_flow_trace_drain_clears_log() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 /").await.unwrap();
    let first = interp.drain_error_flow_trace();
    assert!(!first.is_empty());
    let second = interp.drain_error_flow_trace();
    assert!(second.is_empty());
}

#[tokio::test]
async fn direct_bubble_carries_division_by_zero_reason() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 /").await.unwrap();
    let stack = interp.get_stack();
    assert_eq!(
        stack.len(),
        1,
        "stack after `10 0 /` should follow DIV's normal Bubble/NIL stack effect"
    );
    let top = stack.last().unwrap();
    assert!(top.is_nil());
    let reason = top.nil_reason().cloned();
    assert_eq!(reason, Some(NilReason::DivisionByZero));
}
