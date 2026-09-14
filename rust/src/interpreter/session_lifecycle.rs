//! Session lifecycle.
//!
//! `execute_reset` returns the interpreter to a clean state: stack,
//! dictionary, output, and epochs. Compiling a word body is an unobservable
//! implementation detail (LANG.AUTHORITY.FREEDOM), so nothing here changes what
//! a program produces.

use std::sync::Arc;

use crate::error::Result;
use crate::types::WordDefinition;

use super::compiled_plan::{arc_plan, compile_word_definition, plan_is_all_fallback, CompiledPlan};
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
    /// The `Err` case is reserved for a failure of the restore itself rather
    /// than of one entry: the dependency rebuild below sees the whole
    /// dictionary, so nothing partial can be salvaged from it.
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
        self.reset_execution_modes();
        self.pending_tokens = None;
        self.pending_token_index = 0;
        self.pending_word_descriptions.clear();
        self.runtime_scratch.clear();
        self.call_stack.clear();
        self.call_depth = 0;
        self.source_spans.clear();
        self.section_depth = 0;
        self.current_source_span = None;
        // `cond_dispatch_enabled` is a configuration flag, not run state, so it
        // is intentionally not reset here.
        self.word_identities.clear();
        self.body_store.clear();
        // A reset is documented as clearing every trace of the previous program,
        // and a resolved-name cache is such a trace. Every *other* way the
        // dictionary changes goes through `bump_dictionary_epoch`, which clears
        // this cache as it moves the epoch; a reset moves neither, so its
        // entries were the one kind that outlived the dictionary they described
        // and still answered at a matching epoch. Nothing observable depended on
        // it — `resolve_word_entry` re-checks the live vocabulary on every hit,
        // and a name whose word the reset cleared falls through to a fresh
        // resolution — but that re-check was the only thing standing between a
        // stale entry and a wrong answer, which is a load none of the other
        // clears here are asked to carry.
        self.clear_resolve_cache();
        self.defer_identity_recompute = false;
        self.next_registration_order = 1;
        // Top-level roles live on the stack now and were cleared with it above
        // (`self.stack.clear()`); the registry keeps only value-id-keyed flow
        // state, which session reset leaves untouched, as before.
        self.monitor_notifications.clear();
        self.next_supervisor_id = 1;
        self.runtime_metrics = RuntimeMetrics::default();
        self.error_flow_trace_log.clear();
        // Provenance recording flag persists across a reset; only its data is
        // cleared (Phase 6).
        crate::builtins::register_builtins(&mut self.core_vocabulary);
    }

    /// Compile a word body into a `CompiledPlan`, or decline when the compiled
    /// form would be all-fallback. Compilation is unobservable: a run produces
    /// the same result whether it went through a plan or the plain path.
    pub(crate) fn build_or_reuse_compiled_plan(
        &mut self,
        _resolved_name: &str,
        def: &Arc<WordDefinition>,
    ) -> Option<Arc<CompiledPlan>> {
        let compiled = compile_word_definition(def, self);
        if plan_is_all_fallback(&compiled) {
            return None;
        }

        self.bump_execution_epoch();
        self.runtime_metrics.compiled_plan_build_count += 1;
        Some(arc_plan(compiled))
    }
}
