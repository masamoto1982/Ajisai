//! Linking a downstream type failure back to the NIL that caused it.
//!
//! A NIL flows like any other value (LANG.FAILURE.PASSTHROUGH), so the Word
//! that *fails* is routinely not the Word that went wrong. `[ 0 100001 ] RANGE
//! LENGTH` is the canonical shape: `RANGE` correctly answers
//! `NIL(spaceExhausted)`, `LENGTH` is handed a Nil where it declares a Vector,
//! and the run ends as `LENGTH: expected a Vector, got Nil`.
//!
//! That top-level message is true and useless. The cause — a materialization
//! ceiling crossed two Words earlier — was reachable only by walking
//! `errorFlowTrace` and reading `absence.reason` off an earlier node, which the
//! README had to tell readers to do. A diagnosis that needs a README to point
//! past it is not doing the job the diagnosis exists for, and "read the other
//! field" is exactly the instruction an agent following the top-level
//! `nextChecks` never receives.
//!
//! So the link is materialized as one more check on the failing diagnosis. It
//! adds no facts: everything it says is already in the trace it reads.

use super::debug_diagnosis::{CauseClass, DebugCheck, DebugDiagnosis, LocalizedText};
use super::error_flow_trace::{ErrorFlowEvent, ErrorFlowEventKind};

/// Stable code for the added check, matched by repair scorers.
pub const UPSTREAM_NIL_CHECK: &str = "checkUpstreamNil";

/// Whether a failure is the kind a stray NIL explains.
///
/// `ValueShape` is the class a Word raises when the value it got is not the
/// shape it declares, which is what receiving a NIL looks like from the
/// receiving Word's side. Other classes are not extended: a stack underflow or
/// an unknown name is not caused by an upstream absence, and an unconditional
/// link would attach a plausible-looking wrong cause to failures that have
/// their own.
fn accepts_upstream_link(why: &CauseClass) -> bool {
    matches!(why, CauseClass::ValueShape | CauseClass::NilFlow)
}

/// The most recent NIL produced before the failure, if the trace holds one.
///
/// Most recent rather than first: with several absences in flight, the one that
/// reached the failing Word is the last to be produced. A NIL the program wrote
/// down itself (`NilReason::Literal`) is skipped — it is not a fault to trace
/// back to, and pointing at it would make "the cause is elsewhere" the answer to
/// a program that simply pushed NIL.
fn producing_event(trace: &[ErrorFlowEvent]) -> Option<(&str, &str)> {
    trace.iter().rev().find_map(|event| {
        if event.kind != ErrorFlowEventKind::NilProduced {
            return None;
        }
        let reason = event.absence.as_ref()?.reason.as_ref()?;
        if matches!(reason, crate::error::NilReason::Literal) {
            return None;
        }
        Some((event.word.as_deref()?, reason.as_protocol_str()))
    })
}

/// Append the upstream-NIL check to `diagnosis` when the trace explains it.
///
/// Idempotent, and a no-op when the trace holds no produced NIL — a genuine
/// type error keeps exactly the diagnosis it had.
pub fn link_upstream_nil(diagnosis: &mut DebugDiagnosis, trace: &[ErrorFlowEvent]) {
    if !accepts_upstream_link(&diagnosis.why) {
        return;
    }
    if diagnosis
        .next_checks
        .iter()
        .any(|check| check.code == UPSTREAM_NIL_CHECK)
    {
        return;
    }
    let Some((producer, reason)) = producing_event(trace) else {
        return;
    };

    diagnosis.next_checks.insert(
        0,
        DebugCheck {
            code: UPSTREAM_NIL_CHECK,
            title: LocalizedText::new("Check the upstream NIL", "上流の NIL を確認する"),
            detail: LocalizedText::new(
                format!(
                    "The value that failed here is a NIL produced earlier by {producer}, with \
                     reason `{reason}`. That is the cause to repair; this Word only reported it. \
                     The full record is the {producer} node in errorFlowTrace.",
                ),
                format!(
                    "ここで失敗した値は、上流の {producer} が理由 `{reason}` で生成した NIL である。\
                     修正すべきはそちらで、この word はそれを報告しただけ。詳細は errorFlowTrace の \
                     {producer} ノードにある。",
                ),
            ),
        },
    );

    // Evidence is what a machine matches on, so the cause belongs there too and
    // not only in the prose above.
    diagnosis
        .evidence
        .push(format!("upstreamNilProducer={producer}"));
    diagnosis
        .evidence
        .push(format!("upstreamNilReason={reason}"));
}
