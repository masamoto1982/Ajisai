use crate::error::{AjisaiError, Result};
use crate::semantic::AbsenceMetadata;
use crate::types::{SemanticStack};

use super::compiled_plan::execute_compiled_plan;
use super::execution_plan_set::ExecutionPlanSet;
use super::{HostEffect, IntegrityMode, Interpreter};

pub struct ValidationOutcome {
    pub result: Result<()>,
    pub used_plain_fallback: bool,
}

/// Compare the two absences for the parts that carry *meaning* rather than
/// human/AI prose. The `reason`, `origin`, and `recoverability` define what the
/// absence semantically is; the `diagnosis` text is intentionally excluded so a
/// wording difference between the compiled and plain paths is not mistaken for a
/// semantic divergence.
fn absence_core_eq(a: &Option<AbsenceMetadata>, b: &Option<AbsenceMetadata>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => {
            x.reason == y.reason && x.origin == y.origin && x.recoverability == y.recoverability
        }
        _ => false,
    }
}

/// Whether two result stacks agree for integrity purposes. `Value`'s own
/// equality compares `data` and `hint` but deliberately ignores `absence`
/// (so language-level `EQ` is unaffected). Shadow validation needs the stricter
/// view: two values that print alike but carry different absence reasons are a
/// divergence we must catch, so absence is compared here explicitly.
fn stacks_integrity_agree(fast: &SemanticStack, plain: &SemanticStack) -> bool {
    fast.len() == plain.len() && fast.iter().zip(plain.iter()).all(|(f, p)| f.role() == p.role() && f.value() == p.value() && absence_core_eq(&f.value().absence, &p.value().absence))
}

/// Full agreement: same stack (value + hint + absence core) and the same
/// ordered sequence of host effects. Host effects are the externally observable
/// channel; the compiled and plain paths must emit an identical effect列 or the
/// optimization has changed observable behavior.
fn paths_integrity_agree(
    fast_stack: &SemanticStack,
    plain_stack: &SemanticStack,
    fast_effects: &[HostEffect],
    plain_effects: &[HostEffect],
) -> bool {
    stacks_integrity_agree(fast_stack, plain_stack) && fast_effects == plain_effects
}

impl Interpreter {
    pub(crate) fn should_shadow_validate(
        &self,
        plan_set: &ExecutionPlanSet,
        stack_len: usize,
    ) -> bool {
        // Hedged modes (elastic engine only) force validation on every call so
        // the compiled-vs-plain race is always observable.
        #[cfg(feature = "elastic-engine")]
        {
            if self.is_hedged_mode() {
                return true;
            }
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
        #[cfg(feature = "elastic-engine")]
        {
            if self.is_hedged_mode() {
                self.runtime_metrics.hedged_race_started_count += 1;
                self.push_hedged_trace(format!(
                    "race:start compiled-vs-plain word={}",
                    resolved_name
                ));
            }
        }

        let saved_stack = self.semantic_stack_snapshot().expect("stack values and semantic roles must remain position-aligned");
        let saved_target = self.operation_target_mode;
        let saved_consumption = self.consumption_mode;
        let saved_output = std::mem::take(&mut self.output_buffer);
        let saved_io_output = std::mem::take(&mut self.io_output_buffer);
        let saved_host_effects = std::mem::take(&mut self.host_effects);

        // Each path runs as one trampolined body pass; a guarded tail self-call
        // defers by raising `tail_jump_pending`. Capture each path's flag so the
        // outer trampoline (in `execute_word_core_inner`) re-runs the next
        // iteration only for the path actually committed, and so a per-step
        // residual stack from one path is compared against the other's.
        self.tail_jump_pending = false;
        let fast_result = execute_compiled_plan(self, compiled);
        let fast_jumped = self.tail_jump_pending;
        let fast_stack = self.semantic_stack_snapshot().expect("stack values and semantic roles must remain position-aligned");
        let fast_output = std::mem::take(&mut self.output_buffer);
        let fast_io_output = std::mem::take(&mut self.io_output_buffer);
        let fast_host_effects = std::mem::take(&mut self.host_effects);

        self.replace_semantic_stack(saved_stack.clone());
        self.operation_target_mode = saved_target;
        self.consumption_mode = saved_consumption;

        self.tail_jump_pending = false;
        let plain_result = self.execute_guard_structure(&def.lines);
        let plain_jumped = self.tail_jump_pending;
        // Default to the fast path's decision; commit_plain arms below override.
        self.tail_jump_pending = fast_jumped;
        let plain_stack = self.semantic_stack_snapshot().expect("stack values and semantic roles must remain position-aligned");
        let plain_output = std::mem::take(&mut self.output_buffer);
        let plain_io_output = std::mem::take(&mut self.io_output_buffer);
        let plain_host_effects = std::mem::take(&mut self.host_effects);

        self.output_buffer = saved_output;
        self.io_output_buffer = saved_io_output;
        self.host_effects = saved_host_effects;

        let mode = self.validation_policy.integrity_mode;

        match (&fast_result, &plain_result) {
            (Ok(()), Ok(())) => {
                // Under `Off` we keep the historical, cheaper check (stack value
                // equality only). Every other mode performs the enriched
                // comparison so absence/effect divergences are visible.
                let agree = if mode == IntegrityMode::Off {
                    fast_stack == plain_stack
                } else {
                    paths_integrity_agree(
                        &fast_stack,
                        &plain_stack,
                        &fast_host_effects,
                        &plain_host_effects,
                    )
                };

                if agree {
                    self.runtime_metrics.shadow_validation_success_count += 1;
                    self.commit_fast(
                        fast_stack,
                        fast_output,
                        fast_io_output,
                        fast_host_effects,
                    );
                    return ValidationOutcome {
                        result: Ok(()),
                        used_plain_fallback: false,
                    };
                }

                // The two paths produced different observable results. Record it
                // and react per the active mode.
                self.runtime_metrics
                    .shadow_validation_integrity_mismatch_count += 1;

                match mode {
                    // Legacy / characterization: keep the optimized result.
                    IntegrityMode::Off | IntegrityMode::Observe => {
                        self.commit_fast(
                            fast_stack,
                                fast_output,
                            fast_io_output,
                            fast_host_effects,
                        );
                        ValidationOutcome {
                            result: Ok(()),
                            used_plain_fallback: false,
                        }
                    }
                    // Safe default: the reference path is authoritative.
                    IntegrityMode::Fallback => {
                        self.runtime_metrics.shadow_validation_fallback_count += 1;
                        self.push_hedged_trace(format!(
                            "shadow:integrity-mismatch word={} -> plain",
                            resolved_name
                        ));
                        self.commit_plain(
                            plain_stack,
                            plain_output,
                            plain_io_output,
                            plain_host_effects,
                        );
                        self.tail_jump_pending = plain_jumped;
                        ValidationOutcome {
                            result: Ok(()),
                            used_plain_fallback: true,
                        }
                    }
                    // Refuse the result outright.
                    IntegrityMode::Strict => {
                        self.push_hedged_trace(format!(
                            "shadow:integrity-failure word={}",
                            resolved_name
                        ));
                        ValidationOutcome {
                            result: Err(integrity_failure_error(resolved_name)),
                            used_plain_fallback: false,
                        }
                    }
                }
            }
            // Compiled path errored, reference path succeeded: the reference is
            // authoritative, so adopt it. (Unchanged behavior.)
            (Err(_), Ok(())) => {
                self.runtime_metrics.shadow_validation_fallback_count += 1;
                self.push_hedged_trace(format!("shadow:fallback word={} -> plain", resolved_name));
                self.commit_plain(
                    plain_stack,
                    plain_output,
                    plain_io_output,
                    plain_host_effects,
                );
                self.tail_jump_pending = plain_jumped;
                ValidationOutcome {
                    result: Ok(()),
                    used_plain_fallback: true,
                }
            }
            // Both paths failed: surface the fast path's error as-is. It must
            // keep its typed identity (not be re-wrapped as a Custom string) so
            // the user-level category — e.g. RecursionLimitExceeded (SPEC
            // §11.1) — survives to the diagnosis and protocol surfaces.
            (Err(e), Err(_)) => ValidationOutcome {
                result: Err(e.clone()),
                used_plain_fallback: false,
            },
            // Compiled path "succeeded" where the reference path failed. The
            // optimization disagrees with the language's own semantics, so its
            // success is not trustworthy. Previously this silently adopted the
            // fast result; that is exactly the broken-result-committed case we
            // want to stop.
            (Ok(()), Err(plain_err)) => {
                self.runtime_metrics
                    .shadow_validation_integrity_mismatch_count += 1;
                match mode {
                    IntegrityMode::Off | IntegrityMode::Observe => {
                        self.commit_fast(
                            fast_stack,
                                fast_output,
                            fast_io_output,
                            fast_host_effects,
                        );
                        ValidationOutcome {
                            result: Ok(()),
                            used_plain_fallback: false,
                        }
                    }
                    IntegrityMode::Fallback => {
                        // Surface the reference path's failure rather than a
                        // success the reference never produced.
                        self.runtime_metrics.shadow_validation_fallback_count += 1;
                        self.push_hedged_trace(format!(
                            "shadow:integrity-divergence word={} -> plain-error",
                            resolved_name
                        ));
                        ValidationOutcome {
                            result: Err(plain_err.clone()),
                            used_plain_fallback: true,
                        }
                    }
                    IntegrityMode::Strict => {
                        self.push_hedged_trace(format!(
                            "shadow:integrity-failure word={}",
                            resolved_name
                        ));
                        ValidationOutcome {
                            result: Err(integrity_failure_error(resolved_name)),
                            used_plain_fallback: false,
                        }
                    }
                }
            }
        }
    }

    fn commit_fast(
        &mut self,
        fast_stack: SemanticStack,
        fast_output: String,
        fast_io_output: String,
        fast_host_effects: Vec<HostEffect>,
    ) {
        self.replace_semantic_stack(fast_stack);
        self.output_buffer.push_str(&fast_output);
        self.io_output_buffer.push_str(&fast_io_output);
        self.host_effects.extend(fast_host_effects);
    }

    fn commit_plain(
        &mut self,
        plain_stack: SemanticStack,
        plain_output: String,
        plain_io_output: String,
        plain_host_effects: Vec<HostEffect>,
    ) {
        self.replace_semantic_stack(plain_stack);
        self.output_buffer.push_str(&plain_output);
        self.io_output_buffer.push_str(&plain_io_output);
        self.host_effects.extend(plain_host_effects);
    }
}

/// Integrity failures project to a normal recoverable error so they flow through
/// the same diagnosis path as any other word failure rather than crashing. A
/// dedicated `ErrorCategory::IntegrityFailure` is a planned follow-up; for now
/// the message is self-describing.
fn integrity_failure_error(word: &str) -> AjisaiError {
    AjisaiError::from(format!(
        "integrity check rejected the optimized result of '{}': compiled and reference paths disagreed",
        word
    ))
}

#[cfg(test)]
mod integrity_comparison_tests {
    use super::{absence_core_eq, paths_integrity_agree, stacks_integrity_agree};
    use crate::error::NilReason;
    use crate::interpreter::HostEffect;
    use crate::semantic::{AbsenceMetadata, AbsenceOrigin, Recoverability};
    use crate::types::fraction::Fraction;
    use crate::types::{SemanticStack};

    fn scalar(n: i64) -> Value {
        Value::from_fraction(Fraction::from(n))
    }

    fn nil_with(reason: NilReason, origin: AbsenceOrigin) -> Value {
        let mut v = Value::nil();
        v.absence = Some(AbsenceMetadata::with_reason(
            reason,
            origin,
            Recoverability::Recoverable,
        ));
        v
    }

    #[test]
    fn identical_stacks_and_effects_agree() {
        let fast = vec![scalar(1), scalar(2)];
        let plain = vec![scalar(1), scalar(2)];
        let effects = vec![HostEffect::Print("hi".into())];
        assert!(paths_integrity_agree(&fast, &plain, &effects, &effects));
    }

    #[test]
    fn differing_host_effects_are_a_divergence() {
        let stack = vec![scalar(1)];
        let fast_effects = vec![HostEffect::Print("a".into())];
        let plain_effects = vec![HostEffect::Print("b".into())];
        assert!(stacks_integrity_agree(&stack, &stack));
        assert!(!paths_integrity_agree(
            &stack,
            &stack,
            &fast_effects,
            &plain_effects
        ));
    }

    #[test]
    fn missing_host_effect_is_a_divergence() {
        let stack = vec![scalar(1)];
        let fast_effects = vec![HostEffect::Serial("x".into())];
        let plain_effects: Vec<HostEffect> = vec![];
        assert!(!paths_integrity_agree(
            &stack,
            &stack,
            &fast_effects,
            &plain_effects
        ));
    }

    #[test]
    fn same_nil_shape_different_reason_is_a_divergence() {
        // Both stacks print as a single NIL with identical data+hint, so
        // `Value`-level equality alone would call them equal. The absence
        // reason differs, which is exactly the silently-broken-meaning case.
        let fast = vec![nil_with(
            NilReason::DivisionByZero,
            AbsenceOrigin::DivisionByZero,
        )];
        let plain = vec![nil_with(
            NilReason::IndexOutOfBounds,
            AbsenceOrigin::IndexOutOfBounds,
        )];
        assert_eq!(
            fast[0], plain[0],
            "Value equality ignores absence by design"
        );
        assert!(
            !stacks_integrity_agree(&fast, &plain),
            "integrity comparison must catch the absence-reason divergence"
        );
    }

    #[test]
    fn same_nil_reason_agrees() {
        let fast = vec![nil_with(
            NilReason::EmptySequence,
            AbsenceOrigin::EmptySequence,
        )];
        let plain = vec![nil_with(
            NilReason::EmptySequence,
            AbsenceOrigin::EmptySequence,
        )];
        assert!(stacks_integrity_agree(&fast, &plain));
    }

    #[test]
    fn absence_core_eq_ignores_diagnosis_text() {
        let a = Some(AbsenceMetadata::with_reason(
            NilReason::MissingField,
            AbsenceOrigin::MissingField,
            Recoverability::Recoverable,
        ));
        let b = Some(AbsenceMetadata::with_reason(
            NilReason::MissingField,
            AbsenceOrigin::MissingField,
            Recoverability::Recoverable,
        ));
        assert!(absence_core_eq(&a, &b));
        assert!(!absence_core_eq(&a, &None));
    }
}
