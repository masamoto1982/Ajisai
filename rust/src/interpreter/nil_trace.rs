//! Tracing the NIL a Word *produced*, as distinct from one it carried.
//!
//! A reasoned NIL flows like any other value (LANG.FAILURE.PASSTHROUGH): an
//! absent data operand is the result, reason unchanged, and the primitive does
//! not run; an element operand's absence travels with the element. Reading the
//! result alone, every Word downstream of `1 0 DIV` looked like the one that
//! answered `divisionByZero` — `1 0 DIV 2 ADD 3 MUL` recorded three
//! `nilProduced` events, the last for `MUL` — and a host that shows the latest
//! event blamed the last Word the NIL passed. The Word that produced an absence
//! is the one that *minted* it (`crate::semantic::minted_absence_count`), and a
//! Word inside whose run another Word minted it is the frame it was produced
//! *in*, recorded as the enclosing context the error path already keeps
//! (`insideWords=`), never as a second producer.

use crate::error::NilReason;
use crate::types::{Value, ValueData};

use super::debug_diagnosis::{DebugDiagnosis, ErrorPhase};
use super::error_flow_trace::{ErrorFlowEvent, ErrorFlowEventKind};
use super::Interpreter;

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
/// result: `6 0 DIV` was traced and `[ 6 ] [ 0 ] DIV` was not, and `[ 4 -1 ] SQRT`
/// never was, though all three project for a reason the Word can name.
///
/// `Literal` is excluded because it is the absence a Word *received*, not one
/// it made: a `NIL` written in source, and — since a dense lane carries
/// presence but no reason — any absence that has passed through a tensor. So
/// `[ 1 NIL 3 ] [ 2 ] MUL` records nothing, which is right: `MUL` propagated that
/// NIL, it did not produce it.
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
    // A dense tensor already keeps *why* each absent lane is absent, in a map
    // holding only the absent ones. Materializing every lane to look for it
    // read the rare fact out of the common one: `as_vector_view` on a `Tensor`
    // rebuilds the whole buffer as boxed `Value`s, and this runs after every
    // Word. The map is the same evidence, in lane order, sized to the failures
    // rather than to the data.
    if let ValueData::Tensor { data, .. } = &value.data {
        return dense_projected_nil_reason(data);
    }
    let lanes = value.as_vector_view()?;
    lanes.iter().find_map(projected_nil_reason)
}

/// [`projected_nil_reason`] for a dense tensor, read from its absence map.
///
/// Equivalent to the materialized walk lane by lane: `absences()` yields the
/// absent lanes in ascending lane order and screens each against the presence
/// sentinel, which is exactly the order and the filter a walk over the boxed
/// lanes applied. A lane the tensor was never told a reason for carries none,
/// and is skipped here as `with_reasonless_unknown` was skipped there.
fn dense_projected_nil_reason(data: &crate::types::DenseTensor) -> Option<NilReason> {
    data.absences()
        .find_map(|(_, metadata)| match metadata.reason {
            Some(NilReason::Literal) | None => None,
            Some(reason) => Some(reason),
        })
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
    // Same lane, same map, same reason as `projected_nil_reason` picked — see
    // `dense_projected_nil_reason`. The two must agree on *which* lane they
    // describe, which is why both read the absence map in its lane order.
    if let ValueData::Tensor { data, .. } = &value.data {
        return data
            .absences()
            .find_map(|(_, metadata)| match metadata.reason {
                Some(NilReason::Literal) | None => None,
                Some(_) => Some(metadata.clone()),
            });
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
        let mut diagnosis = DebugDiagnosis::from_error_category(
            ErrorPhase::ExecuteWord,
            Some(word),
            None,
            Some(&reason),
            stack_len_before,
            stack_len_after,
            Some(message.clone()),
        );
        // A User Word that answered the NIL is named as one; only the live
        // dictionary knows it is.
        diagnosis.with_user_vocabulary(self.user_words.keys().map(String::as_str));
        // The absence envelope belongs to the value that actually carries the
        // projection, which for a lifted Word is a lane rather than the result.
        let absence = self.stack.last().and_then(projected_absence_metadata);
        // The ceiling facts behind a resource projection are decided at the
        // projection site — the only place that knows which limit fired and at
        // what size — so they are carried over rather than rebuilt from the
        // category here, which could only say that *a* limit was crossed.
        diagnosis.resource_limit = absence
            .as_ref()
            .and_then(|metadata| metadata.diagnosis.as_ref())
            .and_then(|d| d.resource_limit.clone());
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
