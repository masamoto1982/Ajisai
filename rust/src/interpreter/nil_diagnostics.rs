//! Diagnostic absence accessors (LANG.VALUES.NIL / LANG.OBSERVATION.DIAGNOSIS).
//!
//! `NIL?` and `NIL-REASON` let a program read what a reasoned NIL carries
//! (LANG.VALUES.NIL, `NilReason`) instead of collapsing every absence with a single
//! chosen fallback. They are the whole set: `NIL-ORIGIN`,
//! `NIL-RECOVERABLE?` and `NIL-DIAGNOSIS` named the origin / recoverability /
//! diagnosis metadata that the canonical minimal-NIL model does not have, and
//! are not in `spec/words.json`.
//!
//! Two invariants hold for both:
//!
//!   * **They consume what they read**, like every Word
//!     (LANG.STACK.CONSUMPTION): the inspected value leaves the stack and the
//!     answer takes its place. A program that needs the value afterwards names
//!     it with `BIND`.
//!   * **Every absence, U included.** They key off
//!     [`Value::is_operational_nil`], which is every `Nil` value. The logical
//!     Unknown (U) is a NIL read in truth position (LANG.VALUES.TRUTH), so it
//!     is an absence: `NIL?` answers TRUE for it and `NIL-REASON` reports the
//!     reason it arrived with. Excluding U here briefly made
//!     `'x' NUM TRUE AND NIL-REASON` answer that it had no reason while the
//!     protocol published `absence.reason = invalidEncoding` for the same
//!     value.
//!
//! Applied to a value that is not an operational NIL, `NIL?` yields `FALSE` —
//! a predicate answers its question — and `NIL-REASON` projects a NIL whose
//! reason is `domainMiss`: a well-formed operand outside the accessor's domain,
//! the reason `SQRT` gives a negative radicand (LANG.FAILURE.PROJECT), never an
//! error.

use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::Interpreter;
use crate::semantic::AbsenceMetadata;
use crate::types::{Value, ValueData};

use super::debug_diagnosis::{CauseClass, DebugCheck, DebugDiagnosis, LocalizedText};
use super::error_flow_trace::{ErrorFlowEvent, ErrorFlowEventKind};
use super::trace_diagnosis::{EventDiagnosis, NilProduction};

/// The operational-NIL metadata of a value, or `None` when it is not an
/// operational NIL.
fn operational_absence(value: &Value) -> Option<&AbsenceMetadata> {
    if !value.is_operational_nil() {
        return None;
    }
    // Every operational NIL has metadata; `absence_metadata` is `Some` for a
    // reasoned NIL and the literal-NIL constructor. Fall back defensively.
    value.absence_metadata()
}

/// A protocol-string Text result, or a `domainMiss` NIL when the accessor
/// found no value.
///
/// The projected NIL is *reasoned*. It used to be `Value::nil()`, a bare
/// literal NIL, which left `NIL-REASON`'s declared projection reason
/// unobservable: `5 NIL-REASON NIL-REASON` answered NIL rather
/// than the registered reason. `LANG.FAILURE.PROJECT` says a projection
/// produces "NIL with the reason its contract registers", and
/// `LANG.VALUES.NIL` makes the reason a NIL's entire observable content — a
/// reasonless projection would have no content to observe.
fn push_protocol_string_or_nil(interp: &mut Interpreter, value: Option<&str>) {
    match value {
        Some(protocol) => interp.stack.push(Value::from_string(protocol)),
        None => interp
            .stack
            .push(Value::nil_with_reason_unknown(NilReason::DomainMiss)),
    }
}

/// `NIL?` — consume the value and push `TRUE` when it was an operational NIL,
/// `FALSE` otherwise. It checks absence only and never branches on the reason
/// (LANG.VALUES.NIL).
pub fn op_nil_check(interp: &mut Interpreter) -> Result<()> {
    let value = interp.stack.pop().ok_or(AjisaiError::stack_underflow())?;
    interp
        .stack
        .push(Value::from_bool(value.is_operational_nil()));
    Ok(())
}

/// `NIL-REASON` — the direct reason as a lowerCamelCase protocol-string Text,
/// or a `domainMiss` NIL when the value is not an operational NIL.
///
/// A `userDeclared` NIL (one `ABSENT` made) answers the text it was declared
/// with rather than the reason id: that text is the reason's parameter and,
/// under `LANG.VALUES.NIL`, the NIL's entire observable content.
pub fn op_nil_reason(interp: &mut Interpreter) -> Result<()> {
    let value = interp.stack.pop().ok_or(AjisaiError::stack_underflow())?;
    let protocol: Option<String> = operational_absence(&value).and_then(|absence| {
        let reason = absence.reason.as_ref()?;
        Some(match (reason, absence.detail_text()) {
            (NilReason::UserDeclared, Some(detail)) => detail.to_string(),
            _ => reason.as_protocol_str().to_string(),
        })
    });
    push_protocol_string_or_nil(interp, protocol.as_deref());
    Ok(())
}

// Tracing the NIL a Word *produced*, as distinct from one it carried.
//
// A reasoned NIL flows like any other value (LANG.FAILURE.PASSTHROUGH): an
// absent data operand is the result, reason unchanged, and the primitive does
// not run; an element operand's absence travels with the element. Reading the
// result alone, every Word downstream of `-1 SQRT` looked like the one that
// answered `domainMiss` — `-1 SQRT 2 ADD 3 MUL` recorded three
// `nilProduced` events, the last for `MUL` — and a host that shows the latest
// event blamed the last Word the NIL passed. The Word that produced an absence
// is the one that *minted* it (`crate::semantic::minted_absence_count`), and a
// Word inside whose run another Word minted it is the frame it was produced
// *in*, recorded as the enclosing context the error path already keeps
// (`insideWords=`), never as a second producer.
/// What a dispatch site captures before running a Word, so the outcome can be
/// attributed afterwards: how deep the stack stood, how far the trace had got,
/// and how many absences had been minted.
pub(crate) struct DispatchWitness {
    pub(crate) stack_len_before: usize,
    trace_len_before: usize,
    minted_before: u64,
}

/// The reason a Word's result records for an absence *it produced*, or `None`.
///
/// Looking only at the value itself missed every lifted projection. A Word
/// lifted over a collection projects per lane (`LANG.COLLECTIONS.LIFT`), so
/// the absence it produced sits inside the result rather than being the
/// result: `-1 SQRT` was traced and `[ 4 -1 ] SQRT` was not, though both
/// project for a reason the Word can name.
///
/// `Literal` is excluded because it is the absence a Word *received*, not one
/// it made: a `NIL` written in source, whether it stands alone or as a lane
/// of a tensor. So `[ 1 NIL 3 ] [ 2 ] MUL` records nothing, which is right:
/// `MUL` propagated that NIL, it did not produce it.
///
/// The first reasoned absence in reading order names the event, keeping one
/// event per Word call as the trace's shape requires.
pub(crate) fn projected_nil_reason(value: &Value) -> Option<NilReason> {
    // UNKNOWN is a NIL (LANG.VALUES.TRUTH), so it is traced like any other.
    if value.is_nil() {
        return match value.nil_reason() {
            Some(NilReason::Literal) | None => None,
            Some(reason) => Some(*reason),
        };
    }
    // A dense tensor holds numbers alone, so it carries no absence to find;
    // this runs after every Word, and materializing a million lanes to learn
    // that would read the rare fact out of the common one.
    if matches!(value.data, ValueData::Tensor { .. }) {
        return None;
    }
    let lanes = value.as_vector_view()?;
    lanes.iter().find_map(projected_nil_reason)
}

/// The absence envelope of the same value [`projected_nil_reason`] answered
/// for, so the traced `absence` and the traced reason always describe one
/// value rather than two.
fn projected_absence_metadata(value: &Value) -> Option<crate::semantic::AbsenceMetadata> {
    if value.is_nil() {
        return match value.nil_reason() {
            Some(NilReason::Literal) | None => None,
            Some(_) => value.normalized_absence_metadata(),
        };
    }
    // As `projected_nil_reason`: a dense tensor carries no absence.
    if matches!(value.data, ValueData::Tensor { .. }) {
        return None;
    }
    let lanes = value.as_vector_view()?;
    lanes.iter().find_map(projected_absence_metadata)
}

impl Interpreter {
    /// Capture the facts a dispatch site needs before running a Word.
    pub(crate) fn begin_dispatch(&self) -> DispatchWitness {
        DispatchWitness {
            stack_len_before: self.stack.len(),
            trace_len_before: self.error_flow_trace_log.len(),
            minted_before: crate::semantic::minted_absence_count(),
        }
    }

    /// One Word dispatch with the records it owes: the NIL it answered
    /// (`trace_nil_outcome`), or the failure, attributed to this Word
    /// (`record_word_dispatch_failure`) before it propagates.
    ///
    /// `name` is already canonical. The interpreted token walk, a compiled
    /// User Word call and a compiled fallback Symbol all dispatch through
    /// here, so the three record the same thing rather than each its own —
    /// compiling is required to be unobservable (LANG.AUTHORITY.FREEDOM), and
    /// a diagnosis is observable.
    pub(crate) fn dispatch_word(&mut self, name: &str) -> crate::error::Result<()> {
        let witness = self.begin_dispatch();
        match self.execute_word_core(name) {
            Ok(()) => {
                self.trace_nil_outcome(name, &witness);
                Ok(())
            }
            Err(err) => {
                self.record_word_dispatch_failure(name, &err, witness.stack_len_before);
                Err(err)
            }
        }
    }

    /// Record what a Word that returned normally did about absence, if
    /// anything — the same thing whichever route dispatched it, since a
    /// diagnosis is observable and compiling a body is not
    /// (LANG.AUTHORITY.FREEDOM).
    ///
    /// Three outcomes, decided in this order:
    /// - Words that ran *inside* this one (a block it applied, a User Word's
    ///   body) recorded their own productions while it ran. Those records are
    ///   the answer to "which Word produced it"; this Word is the frame they
    ///   happened in, and is added to each as enclosing context.
    /// - No absence was minted while it ran: whatever reasoned NIL its result
    ///   carries arrived in an operand, and passing it on is not producing it
    ///   (LANG.FAILURE.PASSTHROUGH). Nothing is recorded.
    /// - Otherwise the reasoned NIL on top is its own, and is recorded.
    pub(crate) fn trace_nil_outcome(&mut self, word: &str, witness: &DispatchWitness) {
        if self.enclose_nil_productions_since(witness.trace_len_before, word) {
            return;
        }
        if crate::semantic::minted_absence_count() == witness.minted_before {
            return;
        }
        let Some(reason) = self.stack.last().and_then(projected_nil_reason) else {
            return;
        };
        self.record_nil_produced(word, reason, witness.stack_len_before);
    }

    /// Mark every `nilProduced` event recorded since `trace_len` as having
    /// happened inside `word`; `true` when there was at least one.
    fn enclose_nil_productions_since(&mut self, trace_len: usize, word: &str) -> bool {
        let mut enclosed = false;
        for event in self.error_flow_trace_log.iter_mut().skip(trace_len) {
            if event.kind != ErrorFlowEventKind::NilProduced {
                continue;
            }
            if let Some(diagnosis) = event.diagnosis.as_mut() {
                diagnosis.with_enclosing_word(word);
            }
            enclosed = true;
        }
        enclosed
    }

    fn record_nil_produced(&mut self, word: &str, reason: NilReason, stack_len_before: usize) {
        // A NIL is reported by its reason and by nothing else: an error
        // category beside it would name an outcome the run did not have.
        let stack_len_after = self.stack.len();
        let message = format!(
            "NIL produced by {} reason={}",
            word,
            reason.as_protocol_str()
        );
        // The absence envelope belongs to the value that actually carries the
        // projection, which for a lifted Word is a lane rather than the result.
        let absence = self.stack.last().and_then(projected_absence_metadata);
        // The diagnosis is built when the trace is read, from these facts
        // (`trace_diagnosis`). Two of them can only be read here:
        // - a User Word that answered the NIL is named as one, and only the
        //   live dictionary knows it is;
        // - the ceiling facts behind a resource projection are decided at the
        //   projection site — the only place that knows which limit fired and
        //   at what size — so they are carried over rather than rebuilt from
        //   the category, which could only say that *a* limit was crossed.
        let user_word =
            NilProduction::user_word_of(word, |name| self.user_words.contains_key(name));
        let resource_limit = absence
            .as_ref()
            .and_then(|metadata| metadata.diagnosis.as_ref())
            .and_then(|d| d.resource_limit.clone());
        let diagnosis = EventDiagnosis::nil_produced(NilProduction {
            word: word.to_string(),
            reason,
            stack_len_before,
            stack_len_after,
            message: message.clone(),
            user_word,
            resource_limit,
            enclosing: Vec::new(),
        });
        self.push_error_flow_trace(ErrorFlowEvent {
            kind: ErrorFlowEventKind::NilProduced,
            word: Some(word.to_string()),
            error_category: None,
            absence,
            stack_len_before,
            stack_len_after,
            message,
            diagnosis: Some(diagnosis),
            error_text: String::new(),
        });
    }
}

// Linking a downstream type failure back to the NIL that caused it.
//
// A NIL flows like any other value (LANG.FAILURE.PASSTHROUGH), so the Word
// that *fails* is routinely not the Word that went wrong. `0 100001 RANGE
// LENGTH` is the canonical shape: `RANGE` correctly answers
// `NIL(spaceExhausted)`, `LENGTH` is handed a Nil where it declares a Vector,
// and the run ends as `LENGTH: expected a Vector, got Nil`.
//
// That top-level message is true and useless. The cause — a materialization
// ceiling crossed two Words earlier — was reachable only by walking
// `errorFlowTrace` and reading `absence.reason` off an earlier node, which the
// README had to tell readers to do. A diagnosis that needs a README to point
// past it is not doing the job the diagnosis exists for, and "read the other
// field" is exactly the instruction an agent following the top-level
// `nextChecks` never receives.
//
// So the link is materialized as one more check on the failing diagnosis. It
// adds no facts: everything it says is already in the trace it reads.
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
