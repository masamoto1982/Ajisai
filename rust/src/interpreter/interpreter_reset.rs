use super::Interpreter;
use crate::error::Result;
use crate::interpreter::RuntimeMetrics;

impl Interpreter {
    /// Clear per-run/session state while preserving definitions, imports, and
    /// reusable artifacts derived from them.
    ///
    /// Phase 5 separates the lifetime of session state from artifact state: a
    /// GUI worker can use this boundary before replaying a snapshot without
    /// discarding compiled plans, body content, word identities, or inferred
    /// contracts that remain valid for the unchanged dictionaries/imports.
    pub fn execute_session_reset(&mut self) -> Result<()> {
        self.stack.clear();
        self.output_buffer.clear();
        self.host_effects.clear();
        self.definition_to_load = None;
        self.reset_execution_modes();
        self.force_flag = false;
        self.pending_tokens = None;
        self.pending_token_index = 0;
        self.call_stack.clear();
        self.call_depth = 0;
        self.tail_self_word = None;
        self.in_tail_context = false;
        self.tail_jump_pending = false;
        // `cond_dispatch_enabled` is a configuration flag, not run state, so it
        // is intentionally not reset here.
        self.semantic_registry.clear();
        self.child_runtimes.clear();
        self.next_child_id = 1;
        self.monitor_notifications.clear();
        self.next_supervisor_id = 1;
        self.runtime_metrics = RuntimeMetrics::default();
        self.hedged_trace_log.clear();
        self.error_flow_trace_log.clear();
        self.serial_inbox.clear();
        self.serial_disconnected.clear();
        Ok(())
    }

    pub fn execute_reset(&mut self) -> Result<()> {
        self.execute_session_reset()?;
        self.core_vocabulary.clear();
        self.user_words.clear();
        self.user_dictionaries.clear();
        self.dependents.clear();
        self.module_state.clear();
        self.owning_dictionary_context = None;
        self.word_identities.clear();
        self.body_store.clear();
        self.defer_identity_recompute = false;
        self.import_table.modules.clear();
        self.module_vocabulary.clear();
        self.dictionary_dependencies.clear();
        self.next_registration_order = 1;
        self.active_user_dictionary = "EXAMPLE".to_string();
        crate::builtins::register_builtins(&mut self.core_vocabulary);
        Ok(())
    }
}
