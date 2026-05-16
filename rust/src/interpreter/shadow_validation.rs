use crate::error::Result;

use super::compiled_plan::execute_compiled_plan;
use super::execution_plan_set::ExecutionPlanSet;
use super::Interpreter;

pub struct ValidationOutcome {
    pub result: Result<()>,
    pub used_plain_fallback: bool,
}

impl Interpreter {
    pub(crate) fn should_shadow_validate(
        &self,
        plan_set: &ExecutionPlanSet,
        stack_len: usize,
    ) -> bool {
        if self.is_hedged_mode() {
            return true;
        }

        self.validation_policy.enable_shadow_validation
            && stack_len <= self.validation_policy.max_validation_input_len
            && plan_set.validation_failures == 0
            && plan_set.validated_until_epoch < self.execution_epoch
    }

    pub(crate) fn run_compiled_with_shadow_validation(
        &mut self,
        resolved_name: &str,
        def: &std::sync::Arc<crate::types::WordDefinition>,
        plan_set: &ExecutionPlanSet,
    ) -> ValidationOutcome {
        let compiled = plan_set.compiled.as_ref().expect("compiled plan required");

        self.runtime_metrics.shadow_validation_started_count += 1;
        if self.is_hedged_mode() {
            self.runtime_metrics.hedged_race_started_count += 1;
            self.push_hedged_trace(format!("race:start compiled-vs-plain word={}", resolved_name));
        }

        let saved_stack = self.stack.clone();
        let saved_hints = self.semantic_registry.stack_hints.clone();
        let saved_target = self.operation_target_mode;
        let saved_consumption = self.consumption_mode;
        let saved_safe_mode = self.safe_mode;
        let saved_output = std::mem::take(&mut self.output_buffer);
        let saved_io_output = std::mem::take(&mut self.io_output_buffer);

        let fast_result = execute_compiled_plan(self, compiled);
        let fast_stack = self.stack.clone();
        let fast_hints = self.semantic_registry.stack_hints.clone();
        let fast_output = std::mem::take(&mut self.output_buffer);
        let fast_io_output = std::mem::take(&mut self.io_output_buffer);

        self.stack = saved_stack;
        self.semantic_registry.stack_hints = saved_hints;
        self.operation_target_mode = saved_target;
        self.consumption_mode = saved_consumption;
        self.safe_mode = saved_safe_mode;

        let plain_result = self.execute_guard_structure(&def.lines);
        let plain_stack = self.stack.clone();
        let plain_hints = self.semantic_registry.stack_hints.clone();
        let plain_output = std::mem::take(&mut self.output_buffer);
        let plain_io_output = std::mem::take(&mut self.io_output_buffer);

        self.output_buffer = saved_output;
        self.io_output_buffer = saved_io_output;

        match (&fast_result, &plain_result) {
            (Ok(()), Ok(())) if fast_stack == plain_stack => {
                self.runtime_metrics.shadow_validation_success_count += 1;
                self.stack = fast_stack;
                self.semantic_registry.stack_hints = fast_hints;
                self.output_buffer.push_str(&fast_output);
                self.io_output_buffer.push_str(&fast_io_output);
                ValidationOutcome {
                    result: Ok(()),
                    used_plain_fallback: false,
                }
            }
            (_, Ok(())) => {
                self.runtime_metrics.shadow_validation_fallback_count += 1;
                self.push_hedged_trace(format!("shadow:fallback word={} -> plain", resolved_name));
                self.stack = plain_stack;
                self.semantic_registry.stack_hints = plain_hints;
                self.output_buffer.push_str(&plain_output);
                self.io_output_buffer.push_str(&plain_io_output);
                ValidationOutcome {
                    result: Ok(()),
                    used_plain_fallback: true,
                }
            }
            (Err(e), Err(_)) => ValidationOutcome {
                result: Err(crate::error::AjisaiError::from(format!("{}", e))),
                used_plain_fallback: false,
            },
            (Ok(()), Err(_)) => {
                self.stack = fast_stack;
                self.semantic_registry.stack_hints = fast_hints;
                self.output_buffer.push_str(&fast_output);
                self.io_output_buffer.push_str(&fast_io_output);
                ValidationOutcome {
                    result: Ok(()),
                    used_plain_fallback: false,
                }
            }
        }
    }
}
