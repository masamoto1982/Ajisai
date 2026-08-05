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

impl Interpreter {
    /// Full reset: clears every trace of the previous program.
    pub fn execute_reset(&mut self) -> Result<()> {
        self.reset_session_state();
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
        self.definition_to_load = None;
        self.documentation_to_show = None;
        self.reset_execution_modes();
        self.pending_tokens = None;
        self.pending_token_index = 0;
        self.runtime_scratch.clear();
        self.call_stack.clear();
        self.call_depth = 0;
        self.tail_self_word = None;
        self.in_tail_context = false;
        self.tail_jump_pending = false;
        // `cond_dispatch_enabled` is a configuration flag, not run state, so it
        // is intentionally not reset here.
        self.word_identities.clear();
        self.body_store.clear();
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
