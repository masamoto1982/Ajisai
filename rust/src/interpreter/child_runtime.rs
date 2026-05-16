use crate::error::AjisaiError;
use crate::types::{DisplayHint, Value};

use super::interpreter_core::{
    ChildRuntime, ChildState, ExitReason, RuntimeDictionarySnapshot,
};
use super::value_extraction_helpers::extract_integer_from_value;
use super::Interpreter;

impl Interpreter {
    fn capture_runtime_snapshot(&self) -> RuntimeDictionarySnapshot {
        RuntimeDictionarySnapshot {
            user_words: self.user_words.clone(),
            user_dictionaries: self.user_dictionaries.clone(),
            dependents: self.dependents.clone(),
            import_table: self.import_table.clone(),
            module_vocabulary: self.module_vocabulary.clone(),
            dictionary_dependencies: self.dictionary_dependencies.clone(),
            next_registration_order: self.next_registration_order,
            active_user_dictionary: self.active_user_dictionary.clone(),
        }
    }

    fn build_exit_result(reason: ExitReason, stack: Option<Vec<Value>>) -> Vec<Value> {
        let status = match reason {
            ExitReason::Normal => "completed",
            ExitReason::Killed => "killed",
            ExitReason::Timeout => "timeout",
            ExitReason::Error(_) => "failed",
        };
        vec![
            Value::from_string(status),
            Value::from_vector(stack.unwrap_or_default()),
        ]
    }

    fn map_error_to_exit_reason(error: AjisaiError) -> ExitReason {
        match error {
            AjisaiError::ExecutionLimitExceeded { .. } => ExitReason::Timeout,
            AjisaiError::DivisionByZero => ExitReason::Error("DivisionByZero".to_string()),
            AjisaiError::StackUnderflow => ExitReason::Error("StackUnderflow".to_string()),
            AjisaiError::UnknownWord(_) => ExitReason::Error("UnknownWord".to_string()),
            other => ExitReason::Error(other.to_string()),
        }
    }

    pub(crate) fn op_spawn(&mut self) -> crate::error::Result<()> {
        let block = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        self.semantic_registry.pop_hint();
        let code_block = block
            .as_code_block()
            .ok_or_else(|| AjisaiError::from("SPAWN requires a code block"))?
            .clone();

        self.bump_execution_epoch();
        let spawn_epoch = self.current_epoch_snapshot();
        let id = self.next_child_id;
        self.next_child_id += 1;
        self.child_runtimes.insert(
            id,
            ChildRuntime {
                code_block,
                dictionary_snapshot: self.capture_runtime_snapshot(),
                state: ChildState::Running,
                exit_reason: None,
                result_snapshot: None,
                monitored: false,
                spawn_epoch,
            },
        );
        self.stack.push(Value::from_process_handle(id));
        self.semantic_registry.push_hint(DisplayHint::Auto);
        Ok(())
    }

    pub(crate) fn op_status(&mut self) -> crate::error::Result<()> {
        let handle = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        self.semantic_registry.pop_hint();
        let id = handle
            .as_process_handle()
            .ok_or_else(|| AjisaiError::from("STATUS requires a process handle"))?;
        let child = self
            .child_runtimes
            .get(&id)
            .ok_or_else(|| AjisaiError::from("Unknown process handle"))?;
        let status = match child.state {
            ChildState::Running => "running",
            ChildState::Completed => "completed",
            ChildState::Failed => "failed",
            ChildState::Killed => "killed",
            ChildState::Timeout => "timeout",
        };
        self.stack.push(Value::from_string(status));
        self.semantic_registry.push_hint(DisplayHint::String);
        Ok(())
    }

    pub(crate) fn op_kill(&mut self) -> crate::error::Result<()> {
        let handle = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        self.semantic_registry.pop_hint();
        let id = handle
            .as_process_handle()
            .ok_or_else(|| AjisaiError::from("KILL requires a process handle"))?;
        let child = self
            .child_runtimes
            .get_mut(&id)
            .ok_or_else(|| AjisaiError::from("Unknown process handle"))?;
        child.state = ChildState::Killed;
        child.exit_reason = Some(ExitReason::Killed);
        child.result_snapshot = Some(Self::build_exit_result(ExitReason::Killed, None));
        self.stack.push(Value::from_string("killed"));
        self.semantic_registry.push_hint(DisplayHint::String);
        Ok(())
    }

    fn run_child_to_completion(&self, child: &mut ChildRuntime) {
        if !matches!(child.state, ChildState::Running) {
            return;
        }

        let mut child_interpreter = Interpreter::new();
        child_interpreter.user_words = child.dictionary_snapshot.user_words.clone();
        child_interpreter.user_dictionaries = child.dictionary_snapshot.user_dictionaries.clone();
        child_interpreter.dependents = child.dictionary_snapshot.dependents.clone();
        child_interpreter.import_table = child.dictionary_snapshot.import_table.clone();
        child_interpreter.module_vocabulary = child.dictionary_snapshot.module_vocabulary.clone();
        child_interpreter.dictionary_dependencies =
            child.dictionary_snapshot.dictionary_dependencies.clone();
        child_interpreter.next_registration_order = child.dictionary_snapshot.next_registration_order;
        child_interpreter.active_user_dictionary = child.dictionary_snapshot.active_user_dictionary.clone();
        child_interpreter.max_execution_steps = self.max_execution_steps;

        let lines = vec![crate::types::ExecutionLine {
            body_tokens: child.code_block.clone().into(),
        }];

        match child_interpreter.execute_guard_structure_sync(&lines) {
            Ok(()) => {
                child.state = ChildState::Completed;
                child.exit_reason = Some(ExitReason::Normal);
                let stack = child_interpreter.stack.clone();
                child.result_snapshot = Some(Self::build_exit_result(ExitReason::Normal, Some(stack)));
            }
            Err(err) => {
                let exit_reason = Self::map_error_to_exit_reason(err);
                child.state = match exit_reason {
                    ExitReason::Timeout => ChildState::Timeout,
                    _ => ChildState::Failed,
                };
                child.exit_reason = Some(exit_reason.clone());
                let stack = child_interpreter.stack.clone();
                child.result_snapshot = Some(Self::build_exit_result(exit_reason, Some(stack)));
            }
        }
    }

    pub(crate) fn op_await(&mut self) -> crate::error::Result<()> {
        let handle = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        self.semantic_registry.pop_hint();
        let id = handle
            .as_process_handle()
            .ok_or_else(|| AjisaiError::from("AWAIT requires a process handle"))?;
        let mut child = self
            .child_runtimes
            .remove(&id)
            .ok_or_else(|| AjisaiError::from("Unknown process handle"))?;

        self.run_child_to_completion(&mut child);
        let result = child
            .result_snapshot
            .clone()
            .unwrap_or_else(|| vec![Value::from_string("failed"), Value::from_vector(vec![])]);

        if child.monitored {
            self.monitor_notifications.push(result.clone());
        }
        self.child_runtimes.insert(id, child);
        self.stack.push(Value::from_vector(result));
        self.semantic_registry.push_hint(DisplayHint::Auto);
        Ok(())
    }

    pub(crate) fn op_monitor(&mut self) -> crate::error::Result<()> {
        let handle = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        self.semantic_registry.pop_hint();
        let id = handle
            .as_process_handle()
            .ok_or_else(|| AjisaiError::from("MONITOR requires a process handle"))?;
        let child = self
            .child_runtimes
            .get_mut(&id)
            .ok_or_else(|| AjisaiError::from("Unknown process handle"))?;
        child.monitored = true;
        self.stack.push(Value::from_process_handle(id));
        self.semantic_registry.push_hint(DisplayHint::Auto);
        Ok(())
    }

    pub(crate) fn op_supervise(&mut self) -> crate::error::Result<()> {
        let retry_value = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        self.semantic_registry.pop_hint();
        let block = self.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
        self.semantic_registry.pop_hint();
        let max_restarts = extract_integer_from_value(&retry_value)?.max(0) as usize;
        let code_block = block
            .as_code_block()
            .ok_or_else(|| AjisaiError::from("SUPERVISE requires a code block"))?
            .clone();

        self.next_supervisor_id += 1;

        let mut attempt = 0usize;
        loop {
            self.bump_execution_epoch();
        let spawn_epoch = self.current_epoch_snapshot();
        let id = self.next_child_id;
            self.next_child_id += 1;
            let mut child = ChildRuntime {
                code_block: code_block.clone(),
                dictionary_snapshot: self.capture_runtime_snapshot(),
                state: ChildState::Running,
                exit_reason: None,
                result_snapshot: None,
                monitored: false,
                spawn_epoch,
            };
            self.run_child_to_completion(&mut child);
            let ok = matches!(child.state, ChildState::Completed);
            let result = child.result_snapshot.clone().unwrap_or_default();
            self.child_runtimes.insert(id, child);

            if ok {
                self.stack.push(Value::from_vector(result));
                self.semantic_registry.push_hint(DisplayHint::Auto);
                return Ok(());
            }
            if attempt >= max_restarts {
                self.stack.push(Value::from_vector(vec![
                    Value::from_string("failed"),
                    Value::from_vector(vec![]),
                ]));
                self.semantic_registry.push_hint(DisplayHint::Auto);
                return Ok(());
            }
            self.bump_execution_epoch();
            attempt += 1;
        }
    }
}
