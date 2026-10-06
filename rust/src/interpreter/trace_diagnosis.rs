//! The diagnosis an `errorFlowTrace` event carries, built when it is read.
//!
//! Every Word that mints a reasoned NIL records a `nilProduced` event, and
//! each event carries a full [`DebugDiagnosis`]: a summary, evidence, and the
//! localized next checks the registry declares for the Word. Building that
//! cost about 2 µs per event. A block that answers a NIL per element records
//! one event per element, so `[ 'X' BIND 'x' NUM ] MAP` over 100,000 numbers
//! spent 205 ms of its 253 ms writing diagnoses that the run never read. A
//! diagnosis is only read when the trace is (a report, the GUI, a test), and
//! much of the time nothing reads it.
//!
//! So a `nilProduced` event records the facts its diagnosis is built from,
//! and builds the diagnosis the first time something reads it. Those facts
//! are everything the eager construction read, captured at the moment it
//! read them: the Word, the reason, the stack heights, the message, whether
//! the live dictionary held the Word as a User Word, the resource ceiling the
//! absence carried, and the frames it was later found to be inside. The
//! dictionary is the one input that can change before the trace is read, so
//! its answer is captured rather than looked up later. The diagnosis built
//! from these facts is the one the eager route built, field for field
//! (`trace_diagnosis_tests`), so when it is built is unobservable.

#[cfg(test)]
#[path = "trace_diagnosis_tests.rs"]
mod tests;

use std::sync::OnceLock;

use super::debug_diagnosis::{DebugDiagnosis, ErrorPhase, ResourceLimitFacts};
use crate::error::NilReason;

/// The facts a `nilProduced` diagnosis is built from.
#[derive(Clone)]
pub(crate) struct NilProduction {
    pub(crate) word: String,
    pub(crate) reason: NilReason,
    pub(crate) stack_len_before: usize,
    pub(crate) stack_len_after: usize,
    pub(crate) message: String,
    /// The Word's canonical name, when the live dictionary held it as a User
    /// Word at the moment the NIL was produced.
    pub(crate) user_word: Option<String>,
    pub(crate) resource_limit: Option<ResourceLimitFacts>,
    /// The Words this production was later found to have happened inside,
    /// innermost first — `DebugDiagnosis::with_enclosing_word`, deferred.
    pub(crate) enclosing: Vec<String>,
}

impl NilProduction {
    /// `word`'s canonical name when `is_user_word` says the live dictionary
    /// holds it — the one fact `with_user_vocabulary` reads from a reasoned
    /// absence's vocabulary, captured while the dictionary is the one the NIL
    /// was produced under.
    pub(crate) fn user_word_of(word: &str, is_user_word: impl Fn(&str) -> bool) -> Option<String> {
        let canonical = crate::word_name::canonical_word_name(word);
        is_user_word(canonical.as_ref()).then(|| canonical.into_owned())
    }

    fn build(&self) -> DebugDiagnosis {
        let mut diagnosis = DebugDiagnosis::from_error_category(
            ErrorPhase::ExecuteWord,
            Some(&self.word),
            None,
            Some(&self.reason),
            self.stack_len_before,
            self.stack_len_after,
            Some(self.message.clone()),
        );
        // `with_user_vocabulary` reads the vocabulary for two things: whether
        // the locus is a User Word, and — for a misspelled name only — the
        // spelling candidates. A reasoned absence never classifies as a
        // misspelling, so the first is all it reads, and the one name that
        // answers it is the vocabulary it is given.
        diagnosis.with_user_vocabulary(self.user_word.as_deref().into_iter());
        diagnosis.resource_limit = self.resource_limit.clone();
        for word in &self.enclosing {
            diagnosis.with_enclosing_word(word);
        }
        diagnosis
    }
}

/// A trace event's diagnosis: built already, or built on first read.
#[derive(Clone)]
pub struct EventDiagnosis {
    built: OnceLock<DebugDiagnosis>,
    pending: Option<Box<NilProduction>>,
}

impl EventDiagnosis {
    /// A diagnosis built now, for the events whose construction is not
    /// deferred (a Word's failure, which ends the run).
    pub(crate) fn built(diagnosis: DebugDiagnosis) -> Self {
        Self {
            built: OnceLock::from(diagnosis),
            pending: None,
        }
    }

    pub(crate) fn nil_produced(production: NilProduction) -> Self {
        Self {
            built: OnceLock::new(),
            pending: Some(Box::new(production)),
        }
    }

    fn get(&self) -> &DebugDiagnosis {
        self.built.get_or_init(|| {
            self.pending
                .as_ref()
                .expect("an unbuilt diagnosis keeps the facts it is built from")
                .build()
        })
    }

    /// Record the frame this event happened inside. Before the diagnosis is
    /// built the frame joins the facts it will be built from, in order.
    pub(crate) fn with_enclosing_word(&mut self, word: &str) {
        match self.built.get_mut() {
            Some(diagnosis) => diagnosis.with_enclosing_word(word),
            None => self
                .pending
                .as_mut()
                .expect("an unbuilt diagnosis keeps the facts it is built from")
                .enclosing
                .push(word.to_string()),
        }
    }

    /// The diagnosis, owned.
    pub fn to_diagnosis(&self) -> DebugDiagnosis {
        self.get().clone()
    }
}

impl std::ops::Deref for EventDiagnosis {
    type Target = DebugDiagnosis;

    fn deref(&self) -> &DebugDiagnosis {
        self.get()
    }
}

impl std::ops::DerefMut for EventDiagnosis {
    fn deref_mut(&mut self) -> &mut DebugDiagnosis {
        self.get();
        self.pending = None;
        self.built.get_mut().expect("built just above")
    }
}

impl std::fmt::Debug for EventDiagnosis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(f)
    }
}

impl PartialEq for EventDiagnosis {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl Eq for EventDiagnosis {}
