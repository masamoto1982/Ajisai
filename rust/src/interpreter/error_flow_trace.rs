use super::debug_diagnosis::DebugDiagnosis;
use super::trace_diagnosis::EventDiagnosis;
use crate::error::ErrorCategory;
use crate::semantic::AbsenceMetadata;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorFlowEventKind {
    WordError,
    NilProduced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorFlowEvent {
    pub kind: ErrorFlowEventKind,
    pub word: Option<String>,
    pub error_category: Option<ErrorCategory>,
    pub absence: Option<AbsenceMetadata>,
    pub stack_len_before: usize,
    pub stack_len_after: usize,
    pub message: String,
    /// Built on first read for a `nilProduced` event (`trace_diagnosis`).
    pub diagnosis: Option<EventDiagnosis>,
    /// The raised error as it renders, for a `WordError`; empty otherwise.
    ///
    /// An error unwinds through every frame that reached it, and each frame
    /// used to record it again under its own name. Comparing what the frame is
    /// about to record against what the frame below already did is how the
    /// outer frames are recognised as *enclosing* rather than *failing* — see
    /// `Interpreter::attribute_enclosing_word`. Matching on the rendered
    /// `message` instead would be matching on prose that already embeds a word
    /// name, so the error text is kept as its own field.
    pub error_text: String,
}

impl ErrorFlowEventKind {
    pub fn as_protocol_str(&self) -> &'static str {
        match self {
            ErrorFlowEventKind::WordError => "wordError",
            ErrorFlowEventKind::NilProduced => "nilProduced",
        }
    }
}

use crate::error::AjisaiError;

impl crate::interpreter::Interpreter {
    /// What a User Word owes at its call boundary when its body failed.
    ///
    /// Attribution stops at a User Word: from the caller's side the Word is
    /// what failed, since what its body was given is what the caller wrote.
    /// So the failure is recorded again under the Word's own name, and that
    /// record is the answer; whatever the body recorded stays in the trace as
    /// detail. Recording here rather than at the dispatch site is what keeps
    /// the compiled and interpreted body routes naming one Word
    /// (LANG.AUTHORITY.FREEDOM).
    ///
    /// The one failure the caller cannot have caused is a name in the body
    /// that resolved to nothing: no operand explains it, the repair is at the
    /// name (the spelling candidates exist only there), and reporting it as
    /// this Word's failure put the misspelled name in the message alone. So a
    /// resolution failure keeps the record the name made, and the site that
    /// dispatched this Word adds it as the frame the name was written in —
    /// the same reading a failure inside a block gets.
    pub(crate) fn record_user_word_failure(
        &mut self,
        word: &str,
        err: &AjisaiError,
        stack_len_before: usize,
    ) {
        if matches!(
            ErrorCategory::from_error(err),
            Some(ErrorCategory::UnknownWord)
        ) {
            return;
        }
        self.push_word_error_event(word, err, stack_len_before);
    }

    /// The `WordError` event a failed dispatch of `word` leaves, with its
    /// diagnosis: built here once for the User Word frame and the general
    /// dispatch record, so the two cannot drift in what a reader is told.
    ///
    /// The position is the top-level token that reached this failure. A block
    /// and a Word body are each their own token stream with no source of
    /// their own, so the position a reader is sent to is the top-level token
    /// they actually wrote. A misspelled *user* Word is only knowable here:
    /// the compiled-in registry has never heard of it, and this is the frame
    /// that holds the live dictionary.
    fn push_word_error_event(&mut self, word: &str, err: &AjisaiError, stack_len_before: usize) {
        let mut diagnosis =
            DebugDiagnosis::from_error(err, Some(word), stack_len_before, self.stack.len())
                .with_source_position(self.current_source_span);
        diagnosis.with_user_vocabulary(self.user_words.keys().map(String::as_str));
        self.push_error_flow_trace(ErrorFlowEvent {
            kind: ErrorFlowEventKind::WordError,
            word: Some(word.to_string()),
            error_category: ErrorCategory::from_error(err),
            absence: None,
            stack_len_before,
            stack_len_after: self.stack.len(),
            message: format!("word error word={} error={}", word, err),
            diagnosis: Some(EventDiagnosis::built(diagnosis)),
            error_text: err.to_string(),
        });
    }

    /// What a Word dispatch owes when it fails, regardless of how the caller
    /// reached it.
    ///
    /// Extracted so the interpreted loop and a compiled plan record the *same*
    /// thing rather than each recording its own: a compiled `CallBuiltin` used
    /// to propagate its error with nothing recorded at all, so a failure inside
    /// a compiled block or body lost the answer to "which Word failed" — the
    /// diagnosis named the enclosing frame or nothing. Compiling is required to
    /// be unobservable (LANG.AUTHORITY.FREEDOM), and a diagnosis is observable.
    pub(crate) fn record_word_dispatch_failure(
        &mut self,
        word: &str,
        err: &AjisaiError,
        stack_len_before: usize,
    ) {
        // A failure raised inside a block this Word applied, or inside a User
        // Word's body, is already recorded under the name of the Word that
        // raised it. This frame is the one it happened *inside*, so it adds
        // itself as context and leaves the answer to "which Word failed" alone.
        if self.attribute_enclosing_word(word, &err.to_string()) {
            return;
        }
        self.push_word_error_event(word, err, stack_len_before);
    }
}
