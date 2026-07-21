//! Session lifecycle and cross-reset artifact reuse (Phase 5).
//!
//! Splits the interpreter's reset into two operations with different lifetimes:
//! `execute_reset` wipes everything (including the compiled-artifact cache),
//! while `execute_session_reset` clears only ephemeral *session* state and keeps
//! the cache alive so an unchanged user word's `CompiledPlan` survives a GUI
//! run's reset instead of being recompiled. Reuse is content-identity keyed
//! (Section 8.6) and observationally transparent, so the outcome of a run is
//! identical whether its plan was reused or freshly compiled.

use std::sync::Arc;

use crate::error::Result;
use crate::types::WordDefinition;

use super::artifact_store::{ArtifactKey, CompileFlags};
use super::compiled_plan::{
    arc_plan, compile_word_definition, plan_is_all_fallback, CompiledPlan,
    COMPILED_PLAN_SCHEMA_VERSION,
};
use super::interpreter_core::RuntimeMetrics;
use super::Interpreter;

impl Interpreter {
    /// Full reset: clears every trace of the previous program, including the
    /// cross-reset artifact cache. Use when a genuinely clean interpreter is
    /// wanted (CLI between programs, tests). The GUI worker instead uses
    /// `execute_session_reset`, which keeps the artifact cache alive.
    pub fn execute_reset(&mut self) -> Result<()> {
        self.reset_session_state();
        self.artifact_store.clear();
        Ok(())
    }

    /// Session reset (Phase 5): clears all *session* state — stack, dictionaries,
    /// output, effects, epochs, child runtimes — exactly like `execute_reset`,
    /// but preserves `artifact_store` so an unchanged user word's compiled plan
    /// is not rebuilt on the next run. Reuse stays gated on content identity, so
    /// a changed body or dependency produces a different artifact key and never
    /// reuses a stale plan. The observable outcome of a run is identical whether
    /// the plan was reused or freshly compiled.
    pub fn execute_session_reset(&mut self) -> Result<()> {
        self.reset_session_state();
        Ok(())
    }

    /// Clear all ephemeral session state and re-register the core vocabulary.
    /// Shared by `execute_reset` (which additionally drops the artifact cache)
    /// and `execute_session_reset` (which keeps it).
    fn reset_session_state(&mut self) {
        self.stack.clear();
        self.core_vocabulary.clear();
        self.user_words.clear();
        self.user_dictionaries.clear();
        self.dependents.clear();
        self.output_buffer.clear();
        self.host_effects.clear();
        self.definition_to_load = None;
        self.reset_execution_modes();
        self.force_flag = false;
        self.pending_tokens = None;
        self.pending_token_index = 0;
        self.module_state.clear();
        self.call_stack.clear();
        self.call_depth = 0;
        self.tail_self_word = None;
        self.in_tail_context = false;
        self.tail_jump_pending = false;
        // `cond_dispatch_enabled` is a configuration flag, not run state, so it
        // is intentionally not reset here.
        self.owning_dictionary_context = None;
        self.word_identities.clear();
        self.body_store.clear();
        self.defer_identity_recompute = false;
        self.import_table.modules.clear();
        self.module_vocabulary.clear();
        self.dictionary_dependencies.clear();
        self.next_registration_order = 1;
        self.active_user_dictionary = "EXAMPLE".to_string();
        // Top-level roles live on the stack now and were cleared with it above
        // (`self.stack.clear()`); the registry keeps only value-id-keyed flow
        // state, which session reset leaves untouched, as before.
        self.child_runtimes.clear();
        self.next_child_id = 1;
        self.monitor_notifications.clear();
        self.next_supervisor_id = 1;
        self.runtime_metrics = RuntimeMetrics::default();
        self.hedged_trace_log.clear();
        self.error_flow_trace_log.clear();
        // Provenance recording flag persists across a reset; only its data is
        // cleared (Phase 6).
        self.receipt_recorder.clear();
        crate::builtins::register_builtins(&mut self.core_vocabulary);
    }

    /// Compile-time flags that affect `CompiledPlan` shape, forming part of the
    /// cross-reset artifact key (Phase 5).
    pub(crate) fn compile_flags(&self) -> CompileFlags {
        CompileFlags {
            cond_dispatch: self.cond_dispatch_enabled,
            vector_literal: self.vector_literal_enabled,
            compiled_clause: self.compiled_clause_enabled,
        }
    }

    /// Cross-reset artifact key for a user word, if it has a content identity
    /// (Section 8.6). Module words and anonymous blocks have no such identity
    /// and therefore never participate in cross-reset reuse.
    pub(crate) fn artifact_key_for(&self, resolved_name: &str) -> Option<ArtifactKey> {
        let identity = self.word_identities.get(resolved_name)?;
        Some(ArtifactKey::new(
            identity.clone(),
            self.compile_flags(),
            COMPILED_PLAN_SCHEMA_VERSION,
        ))
    }

    /// Obtain the compiled plan for a word body, reusing a cross-reset artifact
    /// when one is available (Phase 5). Returns the plan to store in the word's
    /// `ExecutionPlanSet`, or `None` when the body lowered to all fallbacks.
    ///
    /// On a store hit the reused plan is re-stamped with the current epoch so the
    /// per-def epoch cache accepts it and later calls in this session take the
    /// fast path instead of re-consulting the store; cloning a compiled plan is
    /// far cheaper than recompiling it, and first use is still shadow-validated
    /// against the plain path. On a miss the body is compiled, counted, and
    /// inserted so the next session can reuse it.
    pub(crate) fn build_or_reuse_compiled_plan(
        &mut self,
        resolved_name: &str,
        def: &Arc<WordDefinition>,
    ) -> Option<Arc<CompiledPlan>> {
        let artifact_key = if self.artifact_reuse_enabled {
            self.artifact_key_for(resolved_name)
        } else {
            None
        };

        if let Some(plan) = artifact_key
            .as_ref()
            .and_then(|key| self.artifact_store.get(key))
        {
            let mut restamped = (*plan).clone();
            restamped.compiled_at = self.current_epoch_snapshot();
            return Some(arc_plan(restamped));
        }

        let compiled = compile_word_definition(def, self);
        if plan_is_all_fallback(&compiled) {
            return None;
        }

        self.bump_execution_epoch();
        self.runtime_metrics.compiled_plan_build_count += 1;
        let compiled_arc = arc_plan(compiled);
        if let Some(key) = artifact_key {
            self.artifact_store.insert(key, compiled_arc.clone());
        }
        Some(compiled_arc)
    }

    /// Enable or disable cross-reset artifact reuse (Phase 5). Reuse is
    /// content-identity keyed and observationally transparent, so this only
    /// affects how often plans are rebuilt. In-process equivalent of
    /// `AJISAI_NO_ARTIFACT_REUSE`.
    pub fn set_artifact_reuse_enabled(&mut self, enabled: bool) {
        self.artifact_reuse_enabled = enabled;
    }

    /// Number of compiled plans currently retained in the cross-reset store.
    pub fn artifact_store_len(&self) -> usize {
        self.artifact_store.len()
    }

    /// Override the cross-reset artifact store capacity (Phase 5). Exposed for
    /// eviction testing and long-lived-worker tuning.
    pub fn set_artifact_store_capacity(&mut self, capacity: usize) {
        self.artifact_store.set_capacity(capacity);
    }
}
