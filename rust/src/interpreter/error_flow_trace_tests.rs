//! Test suite for `crate::interpreter::error_flow_trace`.

use crate::error::NilReason;
use crate::interpreter::error_flow_trace::ErrorFlowEventKind;
use crate::interpreter::Interpreter;

#[tokio::test]
async fn nil_produced_event_has_execute_word_diagnosis() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 DIV").await.unwrap();

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
async fn projection_produced_by_word_has_execute_word_diagnosis() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 DIV").await.unwrap();

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
    let result = interp.execute("ADD").await;
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
    interp.execute("10 0 DIV").await.unwrap();

    let trace = interp.drain_error_flow_trace();
    let event = trace
        .iter()
        .find(|e| e.kind == ErrorFlowEventKind::NilProduced)
        .expect("expected NilProduced event");
    let absence = event
        .absence
        .as_ref()
        .expect("NilProduced event must carry absence metadata");
    let reason = absence
        .reason
        .as_ref()
        .expect("the reasoned NIL has a reason");

    assert_eq!(event.kind.as_protocol_str(), "nilProduced");
    assert_eq!(reason.as_protocol_str(), "divisionByZero");
    assert_eq!(absence.origin.as_protocol_str(), "divisionByZero");
    assert_eq!(absence.recoverability.as_protocol_str(), "recoverable");
    assert!(absence.diagnosis.is_none());
}

#[tokio::test]
async fn nil_produced_event_exposes_ai_structured_diagnosis_payload() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 DIV").await.unwrap();

    let trace = interp.drain_error_flow_trace();
    let event = trace
        .iter()
        .find(|e| e.kind == ErrorFlowEventKind::NilProduced)
        .expect("expected NilProduced event");

    let diagnosis = event
        .diagnosis
        .as_ref()
        .expect("NilProduced event should carry a diagnosis");
    // A NIL is not an error: it names no error category, and so no repair.
    // `divisionByZero` is a NIL reason in spec/outcomes.json and never a
    // category, which is what this event used to report it as.
    assert_eq!(event.error_category, None);
    let payload = diagnosis.ai_payload(event.error_category.as_ref());
    assert_eq!(payload.category, None);
    assert_eq!(payload.repair, None);
    assert_eq!(payload.word.as_deref(), Some("DIV"));
    assert_eq!(payload.family.as_deref(), Some("exactArithmetic"));
    // The NIL's reason is the event's absence, where every host reads it.
    assert_eq!(
        event
            .absence
            .as_ref()
            .and_then(|absence| absence.reason.as_ref())
            .map(|reason| reason.as_protocol_str()),
        Some("divisionByZero")
    );
    assert!(diagnosis
        .next_checks
        .iter()
        .any(|check| check.code == "checkDivisor"));
    assert!(
        diagnosis
            .summary
            .starts_with("executeWord / DIV / domain (nil:divisionByZero)"),
        "{}",
        diagnosis.summary
    );
}

#[tokio::test]
async fn error_flow_trace_records_direct_projection_from_word() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 DIV").await.unwrap();
    let trace = interp.drain_error_flow_trace();
    assert!(
        trace
            .iter()
            .any(|e| e.kind == ErrorFlowEventKind::NilProduced
                && e.word.as_deref() == Some("DIV")
                && e.error_category.is_none()
                && e.absence.as_ref().and_then(|a| a.reason.as_ref())
                    == Some(&NilReason::DivisionByZero)),
        "expected NilProduced(DIV) with reason divisionByZero, got {:?}",
        trace
    );
}

#[tokio::test]
async fn error_flow_trace_drain_clears_log() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 DIV").await.unwrap();
    let first = interp.drain_error_flow_trace();
    assert!(!first.is_empty());
    let second = interp.drain_error_flow_trace();
    assert!(second.is_empty());
}

#[tokio::test]
async fn direct_projection_carries_division_by_zero_reason() {
    let mut interp = Interpreter::new();
    interp.execute("10 0 DIV").await.unwrap();
    let stack = interp.get_stack();
    assert_eq!(
        stack.len(),
        1,
        "stack after `10 0 /` should follow DIV's normal NIL-projection stack effect"
    );
    let top = stack.last().unwrap();
    assert!(top.is_nil());
    let reason = top.nil_reason().cloned();
    assert_eq!(reason, Some(NilReason::DivisionByZero));
}

/// A failure is attributed to the Word that raised it, and the Words it
/// happened *inside* are context rather than the answer.
///
/// The trace records every frame the error unwound through, and the outermost
/// one used to win: `[ 1 2 ] [ 'x' 1 ADD ] MAP` reported `MAP`, and every
/// next-check line asked about `MAP`'s expected shape for a failure about
/// `ADD`'s operand. Ten-line blocks made that the whole of debugging.
#[cfg(test)]
mod attribution_tests {
    use crate::interpreter::debug_diagnosis::DebugDiagnosis;
    use crate::interpreter::Interpreter;

    async fn diagnose(source: &str) -> DebugDiagnosis {
        let mut interp = Interpreter::new();
        assert!(
            interp.execute(source).await.is_err(),
            "`{source}` was expected to fail"
        );
        interp
            .drain_error_flow_trace()
            .iter()
            .rev()
            .find_map(|event| event.diagnosis.clone())
            .expect("a failed run records a diagnosis")
    }

    fn evidence<'a>(diagnosis: &'a DebugDiagnosis, key: &str) -> Option<&'a str> {
        diagnosis
            .evidence
            .iter()
            .find_map(|e| e.strip_prefix(key)?.strip_prefix('='))
    }

    #[tokio::test]
    async fn a_block_failure_names_the_word_in_the_block() {
        let diagnosis = diagnose("[ 1 2 ] [ 'x' 1 ADD ] MAP").await;
        assert_eq!(diagnosis.where_.word.as_deref(), Some("ADD"));
        assert_eq!(evidence(&diagnosis, "insideWords"), Some("MAP"));
        assert!(
            diagnosis
                .next_checks
                .iter()
                .any(|c| c.detail.en.contains("ADD") && c.detail.ja.contains("ADD")),
            "the repair checklist should be about the Word that failed: {:?}",
            diagnosis.next_checks
        );
    }

    #[tokio::test]
    async fn nested_higher_order_words_chain_innermost_first() {
        let diagnosis = diagnose("[ [ 1 ] ] [ [ 'x' 1 ADD ] MAP ] MAP").await;
        assert_eq!(diagnosis.where_.word.as_deref(), Some("ADD"));
        assert_eq!(evidence(&diagnosis, "insideWords"), Some("MAP,MAP"));
    }

    /// Attribution stops at a User Word: from the caller's side the Word is
    /// what failed. This is also what keeps the compiled and interpreted body
    /// routes reporting the same Word (LANG.AUTHORITY.FREEDOM).
    #[tokio::test]
    async fn a_user_word_body_failure_names_the_user_word() {
        let diagnosis = diagnose("[ SORT ] 'S' DEF 5 S").await;
        assert_eq!(diagnosis.where_.word.as_deref(), Some("S"));
        assert_eq!(evidence(&diagnosis, "insideWords"), None);
    }

    #[tokio::test]
    async fn a_user_word_applied_by_a_higher_order_word_is_still_the_locus() {
        let diagnosis = diagnose("[ SORT ] 'S' DEF [ 1 2 ] [ S ] MAP").await;
        assert_eq!(diagnosis.where_.word.as_deref(), Some("S"));
        assert_eq!(evidence(&diagnosis, "insideWords"), Some("MAP"));
    }
}

/// Which Word *produced* a reasoned NIL, as against which Words it passed
/// through. A NIL flows like any other value (LANG.FAILURE.PASSTHROUGH): an
/// absent data operand is the result, reason unchanged, and the primitive does
/// not run. Every Word downstream of the projection used to record the NIL as
/// its own, so a host showing the latest event blamed the last Word the NIL
/// passed.
#[cfg(test)]
mod production_attribution_tests {
    use crate::interpreter::error_flow_trace::{ErrorFlowEvent, ErrorFlowEventKind};
    use crate::interpreter::Interpreter;

    async fn productions(source: &str) -> Vec<ErrorFlowEvent> {
        let mut interp = Interpreter::new();
        interp
            .execute(source)
            .await
            .unwrap_or_else(|e| panic!("`{source}` must compute, got: {e:?}"));
        interp
            .drain_error_flow_trace()
            .into_iter()
            .filter(|event| event.kind == ErrorFlowEventKind::NilProduced)
            .collect()
    }

    fn producers(events: &[ErrorFlowEvent]) -> Vec<&str> {
        events
            .iter()
            .filter_map(|event| event.word.as_deref())
            .collect()
    }

    fn inside(event: &ErrorFlowEvent) -> Option<&str> {
        event
            .diagnosis
            .as_ref()?
            .evidence
            .iter()
            .find_map(|e| e.strip_prefix("insideWords="))
    }

    #[tokio::test]
    async fn a_nil_passed_through_data_operands_is_produced_once() {
        let events = productions("1 0 DIV 2 ADD 3 MUL 1 EQ").await;
        assert_eq!(producers(&events), vec!["DIV"]);
    }

    /// An element operand carries a NIL as an ordinary value, so a Word that
    /// collects, joins or extracts it produced nothing either.
    #[tokio::test]
    async fn a_nil_carried_as_an_element_is_not_produced_again() {
        assert_eq!(
            producers(&productions("1 0 DIV 1 COLLECT [ 2 ] CONCAT 0 GET").await),
            vec!["DIV"]
        );
    }

    /// A NIL that entered a User Word through its operand and left through its
    /// result was passed through the body's Words and through the Word.
    #[tokio::test]
    async fn a_user_word_that_passes_a_nil_through_did_not_produce_it() {
        let events = productions("[ 2 DIV ] 'HALVE' DEF 1 0 DIV HALVE").await;
        assert_eq!(producers(&events), vec!["DIV"]);
        assert_eq!(inside(&events[0]), None);
    }

    /// A NIL a body's Word projected is that Word's, and the User Word is the
    /// frame it happened in — innermost first, as a failure's frames are.
    /// `HALVE`'s body compiles; `TWICE`'s holds a nested quotation the
    /// compiler leaves to the interpreter, so both body routes are covered and
    /// must read alike (LANG.AUTHORITY.FREEDOM).
    #[tokio::test]
    async fn a_nil_produced_inside_a_user_word_names_the_producer_and_the_frames() {
        let compiled = productions("[ 0 DIV ] 'HALVE' DEF [ HALVE ] 'OUTER' DEF 1 OUTER").await;
        assert_eq!(producers(&compiled), vec!["DIV"]);
        assert_eq!(inside(&compiled[0]), Some("HALVE,OUTER"));

        let interpreted =
            productions("[ 0 DIV [ ADD ] DROP ] 'TWICE' DEF [ TWICE ] 'OUTER' DEF 1 OUTER").await;
        assert_eq!(producers(&interpreted), vec!["DIV"]);
        assert_eq!(inside(&interpreted[0]), Some("TWICE,OUTER"));
    }

    /// A NIL a block's Word projected under a higher-order Word is the block
    /// Word's, once per application, inside the higher-order Word.
    #[tokio::test]
    async fn a_nil_produced_in_an_applied_block_is_the_block_words() {
        let events = productions("[ 1 2 ] [ 0 DIV ] MAP").await;
        assert_eq!(producers(&events), vec!["DIV", "DIV"]);
        assert!(events.iter().all(|event| inside(event) == Some("MAP")));
    }
}

/// An error carries where in the source it happened, so a reader is sent to a
/// line rather than left to bisect the program. The evidence channel already
/// carries `key=value` facts (`stackLenBefore=`), so the position needed no new
/// protocol field.
#[cfg(test)]
mod source_position_tests {
    use crate::interpreter::Interpreter;

    fn evidence_of(interp: &mut Interpreter, key: &str) -> Option<String> {
        interp
            .drain_error_flow_trace()
            .iter()
            .rev()
            .find_map(|event| event.diagnosis.as_ref())
            .and_then(|d| {
                d.evidence
                    .iter()
                    .find_map(|e| e.strip_prefix(key)?.strip_prefix('=').map(str::to_string))
            })
    }

    #[tokio::test]
    async fn an_unknown_word_reports_the_line_it_is_written_on() {
        let mut interp = Interpreter::new();
        let err = interp.execute("1 PRINT\n2 PRINT\nNOSUCHWORD").await;
        assert!(err.is_err());
        assert_eq!(evidence_of(&mut interp, "sourceLine").as_deref(), Some("3"));
    }

    #[tokio::test]
    async fn a_failure_inside_a_word_body_reports_the_call_site() {
        // The body has no source of its own — it was stored as tokens — so the
        // position a reader can act on is the top-level token that reached it.
        let mut interp = Interpreter::new();
        interp.execute("[ 1 BADWORD ] 'BROKEN' DEF").await.unwrap();
        let _ = interp.drain_error_flow_trace();
        assert!(interp.execute("1 PRINT\n2 PRINT\nBROKEN").await.is_err());
        assert_eq!(evidence_of(&mut interp, "sourceLine").as_deref(), Some("3"));
    }

    #[tokio::test]
    async fn the_column_locates_the_token_within_its_line() {
        let mut interp = Interpreter::new();
        assert!(interp.execute("1 2 ADD\n5 5 NOPE").await.is_err());
        // One drain: reading the trace consumes it.
        let evidence: Vec<String> = interp
            .drain_error_flow_trace()
            .iter()
            .rev()
            .find_map(|event| event.diagnosis.as_ref())
            .map(|d| d.evidence.clone())
            .unwrap_or_default();
        assert!(
            evidence.contains(&"sourceLine=2".to_string()),
            "{evidence:?}"
        );
        assert!(
            evidence.contains(&"sourceColumn=5".to_string()),
            "{evidence:?}"
        );
    }
}
