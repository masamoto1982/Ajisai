//! Session lifecycle.
//!
//! `execute_reset` returns the interpreter to a clean state: stack,
//! dictionary and output. The epochs keep counting, so nothing compiled
//! before the reset can match a dictionary made after it. Compiling a word
//! body is an unobservable
//! implementation detail (LANG.AUTHORITY.FREEDOM), so nothing here changes what
//! a program produces.

use std::sync::Arc;

use crate::error::Result;
use crate::types::WordDefinition;

use super::compiled_plan::{arc_plan, compile_word_definition, CompiledPlan};
use super::interpreter_core::RuntimeMetrics;
use super::Interpreter;

/// One saved definition that could not be restored, and why.
///
/// A restore reports these rather than raising: see
/// [`Interpreter::restore_user_word_definitions`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedRestore {
    pub name: String,
    pub reason: String,
}

impl Interpreter {
    /// Full reset: clears every trace of the previous program.
    pub fn execute_reset(&mut self) -> Result<()> {
        self.reset_session_state();
        Ok(())
    }

    /// Restore saved User Word definitions, skipping the ones that cannot be
    /// restored instead of abandoning the ones that can.
    ///
    /// Each entry is `(name, definition source, description)`. A saved
    /// definition is source text, so restoring it re-runs the lexer and `DEF`
    /// against *today's* rules — and those rules are not frozen. A dictionary
    /// saved before a lexical or naming rule changed can therefore contain an
    /// entry this build no longer accepts, which is not a reason to lose the
    /// rest of it: one unreadable definition used to abort the whole restore
    /// and leave the session holding whichever words happened to precede it.
    ///
    /// The skipped entries are returned instead, so the host can say which
    /// words did not come back. This is the same contract the host already
    /// states for a partially corrupt import — "valid words in a
    /// partially-corrupt file still import" (`interpreter-state-persistence.ts`)
    /// — which it could only honour for entries malformed structurally enough
    /// to spot without the lexer.
    ///
    /// An entry with an empty definition is passed over without a report:
    /// there is nothing to restore and nothing went wrong.
    pub fn restore_user_word_definitions<I>(&mut self, words: I) -> Result<Vec<SkippedRestore>>
    where
        I: IntoIterator<Item = (String, String, Option<String>)>,
    {
        let mut skipped = Vec::new();

        // Defer per-word identity recomputation during the bulk restore and
        // recompute once below via rebuild_dependencies. This turns O(N^2)
        // identity hashing on import into O(N). The flag is always cleared,
        // even on error, so later interactive definitions recompute normally.
        self.defer_identity_recompute = true;
        for (name, definition, description) in words {
            if definition.is_empty() {
                continue;
            }
            match self.restore_one_word(&name, &definition, description) {
                Ok(()) => {}
                Err(reason) => skipped.push(SkippedRestore { name, reason }),
            }
        }
        self.defer_identity_recompute = false;

        self.rebuild_dependencies()?;
        Ok(skipped)
    }

    /// Restore one saved definition, reporting why it could not be as a string.
    fn restore_one_word(
        &mut self,
        name: &str,
        definition: &str,
        description: Option<String>,
    ) -> std::result::Result<(), String> {
        let tokens = crate::tokenizer::tokenize(definition)?;
        super::execute_def::op_def_inner(self, name, &tokens).map_err(|e| e.to_string())?;
        if description.is_some() {
            super::execute_def::set_word_description(self, name, description);
        }
        Ok(())
    }

    /// Clear all ephemeral session state and re-register the core vocabulary.
    fn reset_session_state(&mut self) {
        self.stack.clear();
        self.core_vocabulary.clear();
        self.user_words.clear();
        self.dependents.clear();
        self.output_buffer.clear();
        self.host_effects.clear();
        self.pending_tokens = None;
        self.pending_token_index = 0;
        self.pending_def_body_tokens = None;
        self.pending_word_descriptions.clear();
        self.runtime_scratch.clear();
        self.call_stack.clear();
        self.call_depth = 0;
        self.source_spans.clear();
        self.section_depth = 0;
        self.current_source_span = None;
        self.word_identities.clear();
        self.body_store.clear();
        self.defer_identity_recompute = false;
        self.next_registration_order = 1;
        self.monitor_notifications.clear();
        self.next_supervisor_id = 1;
        self.runtime_metrics = RuntimeMetrics::default();
        self.error_flow_trace_log.clear();
        crate::builtins::register_builtins(&mut self.core_vocabulary);
    }

    /// Compile a word body into a `CompiledPlan`. Compilation is unobservable:
    /// a run produces the same result whether it went through a plan or the
    /// plain path.
    ///
    /// Every body gets a plan, including one the compiler could lower none
    /// of. Such a plan runs its source tokens through the interpreter, exactly
    /// as a body with no plan would — `execute_compiled_plan` re-interprets
    /// any line holding a fallback token whole — so declining it bought
    /// nothing, and cost a recompile and a copy of the definition on every
    /// call, since nothing remembered that the body had been declined.
    pub(crate) fn build_compiled_plan(&mut self, def: &Arc<WordDefinition>) -> Arc<CompiledPlan> {
        let compiled = compile_word_definition(def, self);
        self.bump_execution_epoch();
        self.runtime_metrics.compiled_plan_build_count += 1;
        arc_plan(compiled)
    }
}
