//! Tests for the diagnostic absence accessors (SPEC §4.5.0 / §7.15):
//! `NIL?`, `NIL-REASON`, `NIL-ORIGIN`, `NIL-RECOVERABLE?`, `NIL-DIAGNOSIS`.
//!
//! Coverage follows the §15 discipline: success paths, the non-NIL path, the
//! reason-present vs reason-absent split, protocol-string (not Rust `Debug`)
//! output, the U firewall, source retention, and MC/DC over the two governing
//! decisions (`is_operational_nil` and reason `Some`/`None`).

use crate::error::NilReason;
use crate::interpreter::debug_diagnosis::DebugDiagnosis;
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::Interpreter;
use crate::semantic::{AbsenceOrigin, Recoverability};
use crate::types::{Value, ValueData};

async fn run(code: &str) -> Interpreter {
    let mut interp = Interpreter::new();
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("execute({:?}) failed: {}", code, e));
    interp
}

/// The top-of-stack value, as a protocol string, or `None` when it is NIL.
fn top_text(interp: &Interpreter) -> Option<String> {
    let top = interp.get_stack().last().expect("stack must be non-empty");
    if top.is_nil() {
        return None;
    }
    Some(value_as_string(top).expect("top must be a Text value"))
}

fn top_is_nil(interp: &Interpreter) -> bool {
    interp
        .get_stack()
        .last()
        .map(|v| v.is_nil())
        .unwrap_or(false)
}

fn top_is_true(interp: &Interpreter) -> bool {
    interp
        .get_stack()
        .last()
        .and_then(|v| v.as_truth())
        .unwrap_or(false)
}

fn record_field_string(record: &Value, key: &str) -> Option<String> {
    let ValueData::Record { pairs, shape } = &record.data else {
        return None;
    };
    let pos = shape.slot(key)?;
    let ValueData::Vector(kv) = &pairs.get(pos)?.data else {
        return None;
    };
    value_as_string(kv.get(1)?)
}

// ── NIL? ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn nil_check_is_true_for_operational_nil_and_retains_source() {
    let interp = run("1 0 / NIL?").await;
    let stack = interp.get_stack();
    assert_eq!(stack.len(), 2, "NIL? must retain the inspected value");
    assert!(stack[0].is_nil(), "the inspected NIL is retained below");
    assert!(top_is_true(&interp), "NIL? on an operational NIL is TRUE");
}

#[tokio::test]
async fn nil_check_is_false_for_present_value() {
    let interp = run("5 NIL?").await;
    let stack = interp.get_stack();
    assert_eq!(stack.len(), 2, "NIL? retains the inspected value");
    assert_eq!(
        stack[1].as_truth(),
        Some(false),
        "NIL? on a present value is FALSE"
    );
}

/// MC/DC: the logical Unknown (U) shares NIL storage but is NOT an operational
/// absence, so `NIL?` must report FALSE. This flips only the `is_operational_nil`
/// condition relative to the DIV case above, isolating its independent effect
/// and proving the firewall (SPEC §2.3 / §7.5).
#[tokio::test]
async fn nil_check_is_false_for_logical_unknown() {
    // ((√2+1) − (√2+1)) == 0 is undecidable within the budget, so EQ yields U.
    let interp = run("'math' IMPORT 2 SQRT 1 ADD 2 SQRT 1 ADD SUB 0 EQ NIL?").await;
    assert_eq!(
        interp.get_stack()[1].as_truth(),
        Some(false),
        "NIL? on the logical Unknown must be FALSE (firewall)"
    );
}

// ── NIL-REASON ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn nil_reason_reports_division_by_zero_protocol_string() {
    let interp = run("1 0 / NIL-REASON").await;
    let stack = interp.get_stack();
    assert_eq!(stack.len(), 2, "NIL-REASON must retain the inspected value");
    assert!(stack[0].is_nil(), "the inspected NIL is retained below");
    assert_eq!(
        top_text(&interp).as_deref(),
        Some("divisionByZero"),
        "NIL-REASON must be the lowerCamelCase protocol string, not a Debug name"
    );
}

/// The output must be the protocol string, never the Rust `Debug` rendering of
/// the `NilReason` enum (`DivisionByZero`).
#[tokio::test]
async fn nil_reason_is_protocol_string_not_debug_name() {
    let interp = run("1 0 / NIL-REASON").await;
    let text = top_text(&interp).expect("reason must be Text");
    assert_eq!(text, "divisionByZero");
    assert_ne!(text, format!("{:?}", NilReason::DivisionByZero));
}

#[tokio::test]
async fn nil_reason_reports_index_out_of_bounds() {
    let interp = run("[ 1 2 3 ] [ 9 ] GET NIL-REASON").await;
    assert_eq!(top_text(&interp).as_deref(), Some("indexOutOfBounds"));
}

/// MC/DC: reason `None`. A literal NIL is an operational NIL with no reason, so
/// `NIL-REASON` yields NIL. This flips only the reason `Some`/`None` condition
/// relative to the DIV case.
#[tokio::test]
async fn nil_reason_is_nil_when_no_reason() {
    let interp = run("NIL NIL-REASON").await;
    assert!(
        top_is_nil(&interp),
        "reasonless NIL yields NIL for NIL-REASON"
    );
}

/// The non-NIL path: `NIL-REASON` on a present value yields NIL, not an error.
#[tokio::test]
async fn nil_reason_is_nil_for_present_value() {
    let interp = run("5 NIL-REASON").await;
    assert!(top_is_nil(&interp));
    assert_eq!(interp.get_stack()[0].as_truth(), None);
    assert!(!interp.get_stack()[0].is_nil(), "the 5 is retained below");
}

/// The U firewall for the reason accessor: `NIL-REASON` applied to the logical
/// Unknown (U) must yield NIL, never a reason. Since CS4, U is a distinct
/// `ValueData::Unknown` value carrying no NIL reason (the `logicallyUnknown`
/// reason was retired in PR-3), so `NIL-REASON` — which keys off operational
/// NIL only — reports nothing for U.
#[tokio::test]
async fn nil_reason_does_not_leak_for_unknown() {
    let interp = run("'math' IMPORT 2 SQRT 1 ADD 2 SQRT 1 ADD SUB 0 EQ NIL-REASON").await;
    assert!(
        top_is_nil(&interp),
        "NIL-REASON on U must be NIL, never a reason string"
    );
}

// ── NIL-ORIGIN ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn nil_origin_reports_execution_failure_for_division() {
    let interp = run("1 0 / NIL-ORIGIN").await;
    assert_eq!(top_text(&interp).as_deref(), Some("executionFailure"));
}

#[tokio::test]
async fn nil_origin_reports_literal_for_literal_nil() {
    let interp = run("NIL NIL-ORIGIN").await;
    assert_eq!(
        top_text(&interp).as_deref(),
        Some("literal"),
        "origin is a required field, present even on a reasonless literal NIL"
    );
}

#[tokio::test]
async fn nil_origin_is_nil_for_present_value() {
    let interp = run("5 NIL-ORIGIN").await;
    assert!(top_is_nil(&interp));
}

// ── NIL-RECOVERABLE? ────────────────────────────────────────────────────────

#[tokio::test]
async fn nil_recoverable_reports_protocol_string() {
    let interp = run("1 0 / NIL-RECOVERABLE?").await;
    assert_eq!(
        top_text(&interp).as_deref(),
        Some("recoverable"),
        "recoverability is a four-valued protocol string, returned as Text (SPEC §4.5.0)"
    );
}

#[tokio::test]
async fn nil_recoverable_reports_unknown_for_literal_nil() {
    let interp = run("NIL NIL-RECOVERABLE?").await;
    assert_eq!(top_text(&interp).as_deref(), Some("unknown"));
}

#[tokio::test]
async fn nil_recoverable_is_nil_for_present_value() {
    let interp = run("5 NIL-RECOVERABLE?").await;
    assert!(top_is_nil(&interp));
}

// ── NIL-DIAGNOSIS ───────────────────────────────────────────────────────────

#[tokio::test]
async fn nil_diagnosis_is_nil_when_no_diagnosis_attached() {
    // DIV records its diagnosis in the error-flow trace, not on the value's
    // absence metadata, so NIL-DIAGNOSIS yields NIL here.
    let interp = run("1 0 / NIL-DIAGNOSIS").await;
    assert!(top_is_nil(&interp));
}

#[tokio::test]
async fn nil_diagnosis_is_nil_for_present_value() {
    let interp = run("5 NIL-DIAGNOSIS").await;
    assert!(top_is_nil(&interp));
}

/// When an operational NIL does carry a `diagnosis`, `NIL-DIAGNOSIS` returns it
/// as a Record whose fields are the §4.5.0 protocol strings (not Debug names).
#[tokio::test]
async fn nil_diagnosis_returns_record_with_protocol_string_fields() {
    let diagnosis = DebugDiagnosis::comparison_unknown(Some("LT"), 3);
    let bubble = Value::nil_from_diagnosis(
        NilReason::DivisionByZero,
        AbsenceOrigin::ExecutionFailure,
        Recoverability::Recoverable,
        diagnosis,
    );
    let mut interp = Interpreter::new();
    interp.stack.push(bubble);
    interp
        .execute("NIL-DIAGNOSIS")
        .await
        .expect("NIL-DIAGNOSIS must not raise");

    let stack = interp.get_stack();
    assert_eq!(stack.len(), 2, "NIL-DIAGNOSIS retains the inspected value");
    let record = &stack[1];
    assert!(
        matches!(record.data, ValueData::Record { .. }),
        "diagnosis is a Record"
    );
    assert_eq!(
        record_field_string(record, "when").as_deref(),
        Some("executeWord"),
        "when must be the protocol string, not the Debug name ExecuteWord"
    );
    assert_eq!(
        record_field_string(record, "why").as_deref(),
        Some("nilFlow"),
        "why is a lowerCamelCase protocol string, not the Debug name NilFlow"
    );
    // agreedPrefix is carried through as a machine-readable integer.
    let ValueData::Record { shape, .. } = &record.data else {
        panic!("record");
    };
    assert!(
        shape.contains_key("agreedPrefix"),
        "a CF-comparison diagnosis surfaces agreedPrefix"
    );
}
