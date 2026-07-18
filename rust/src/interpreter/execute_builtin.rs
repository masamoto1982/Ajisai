use crate::builtins::{lookup_builtin_spec, BuiltinExecutorKey};
#[cfg(feature = "elastic-engine")]
use crate::elastic::ElasticMode;
use crate::error::{AjisaiError, Result};
use crate::types::{Interpretation, Token, Value};

use super::compiled_plan::{execute_compiled_plan, is_plan_valid};

use super::{
    arithmetic, cast, comparison, control, control_cond, execute_def, execute_del, execute_lookup,
    higher_order, higher_order_fold, interval_ops, io, logic, modules, nil_diagnostics,
    tensor_cmds, vector_ops, Interpreter,
};

#[cfg(feature = "trace-compile")]
fn trace_compile_metrics(interp: &Interpreter) {
    let m = interp.runtime_metrics();
    eprintln!(
        "[metrics] plan_build={} plan_hit={} plan_miss={}",
        m.compiled_plan_build_count,
        m.compiled_plan_cache_hit_count,
        m.compiled_plan_cache_miss_count
    );
    eprintln!(
        "[metrics] quant_build={} quant_use={}",
        m.quantized_block_build_count, m.quantized_block_use_count
    );
}

impl Interpreter {
    #[cfg(feature = "elastic-engine")]
    pub(crate) fn is_hedged_mode(&self) -> bool {
        matches!(
            self.elastic_mode(),
            ElasticMode::HedgedSafe | ElasticMode::HedgedTrace
        )
    }

    /// Without the `elastic-engine` feature the mode is pinned to `Greedy`,
    /// so hedged classification is statically false and every hedged branch
    /// guarded by it folds away.
    #[cfg(not(feature = "elastic-engine"))]
    pub(crate) fn is_hedged_mode(&self) -> bool {
        false
    }

    /// Public entry point for word execution.
    ///
    /// When `AJISAI_TRACE=1` (or `set_trace_enabled(true)`) is active this
    /// wraps the call with timing instrumentation.  All existing greedy
    /// semantics are preserved unchanged.
    pub(crate) fn execute_word_core(&mut self, name: &str) -> Result<()> {
        if crate::elastic::tracer::is_enabled() {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let t0 = std::time::Instant::now();
                let result = self.execute_word_core_inner(name);
                let nanos = t0.elapsed().as_nanos() as u64;
                crate::elastic::tracer::record(name, nanos);
                return result;
            }
            #[cfg(target_arch = "wasm32")]
            {
                let result = self.execute_word_core_inner(name);
                crate::elastic::tracer::record(name, 0);
                return result;
            }
        }
        self.execute_word_core_inner(name)
    }

    /// Core word-execution logic (greedy, always).
    ///
    /// Never call directly — use `execute_word_core` so tracing applies.
    fn execute_word_core_inner(&mut self, name: &str) -> Result<()> {
        let canonical_name = crate::core_word_aliases::canonicalize_core_word_name(name);
        let name = canonical_name.as_ref();
        let (resolved_name, def) = self.resolve_word_entry(name).ok_or_else(|| {
            let ambiguous = self.check_ambiguity(name);
            if !ambiguous.is_empty() {
                AjisaiError::from(format!(
                    "Ambiguous word '{}': found in {}. Use a qualified path to specify which one you mean.",
                    name.to_uppercase(),
                    ambiguous.join(", ")
                ))
            } else {
                AjisaiError::UnknownWord(name.to_string())
            }
        })?;

        self.execution_step_count += 1;
        if self.execution_step_count > self.max_execution_steps {
            return Err(AjisaiError::ExecutionLimitExceeded {
                limit: self.max_execution_steps,
            });
        }

        if def.lines.is_empty() {
            return self.execute_builtin(&resolved_name);
        }

        // Recursion depth guard: catches blown Rust stack before WASM traps.
        // The matching decrement is just before the return below; there are
        // no `?` early returns between this point and the decrement.
        if self.call_depth + 1 > super::interpreter_core::MAX_USER_WORD_DEPTH {
            return Err(AjisaiError::RecursionLimitExceeded {
                limit: super::interpreter_core::MAX_USER_WORD_DEPTH,
                word: resolved_name.clone(),
            });
        }
        self.call_depth += 1;

        // Section 8.6: resolve this word's bare references through its own
        // dictionary first, both while compiling its execution plan and while
        // running its body. Saved and restored so nested calls into other
        // dictionaries see their own dictionary's context.
        let owning_dict = self
            .split_qualified_name(&resolved_name)
            .map(|(dict, _)| dict);
        let prev_owning = std::mem::replace(&mut self.owning_dictionary_context, owning_dict);

        let plan_set = self.get_execution_plan_set(&resolved_name, &def);

        self.call_stack.push(resolved_name.clone());

        // Internal tail-call elimination ("internal GOTO"): mark this frame as
        // the self-tail-call target, then run its body in a trampoline. A
        // guarded tail self-call (the tail of a COND clause body) sets
        // `tail_jump_pending` and unwinds to here instead of recursing, so the
        // loop re-runs the same body with the next iteration's arguments — a
        // backward jump that never grows `call_depth` or the native stack.
        let prev_tail_self = self.tail_self_word.take();
        let prev_in_tail = self.in_tail_context;
        self.tail_self_word = Some(resolved_name.clone());

        // A word's body is identical at every recursion level, so shadow
        // validation only needs to run at its outermost entry. Skipping it on
        // recursive re-entry keeps the same divergence coverage while removing
        // the heavy validation frame from the recursion chain — recovering
        // native-stack headroom (the depth guard's whole purpose) and avoiding a
        // redundant double execution of the body per level. `call_stack` already
        // holds this call's name (pushed above), so a count above one means we
        // are nested inside an earlier activation of the same word.
        let recursive_reentry = self
            .call_stack
            .iter()
            .filter(|n| n.as_str() == resolved_name)
            .count()
            > 1;

        // The dispatch is inlined into the loop rather than extracted into a
        // helper so the legacy (non-trampolined) recursion path adds no extra
        // native-stack frame per call — important because that path is still
        // bounded only by `MAX_USER_WORD_DEPTH`, and a deeper per-frame cost
        // would lower the effective depth ceiling.
        let result = loop {
            self.in_tail_context = false;
            self.tail_jump_pending = false;

            let body_result = if let Some(plan_set) = plan_set.as_ref() {
                if let Some(qb) = plan_set.quantized.as_ref() {
                    if qb.guard_signature.dictionary_epoch == self.dictionary_epoch
                        && qb.guard_signature.module_epoch == self.module_epoch
                        && qb.purity == super::quantized_block::QuantizedPurity::Pure
                        && !self.is_hedged_mode()
                    {
                        self.runtime_metrics.quantized_block_use_count += 1;
                        if let Some(compiled) = plan_set.compiled.as_ref() {
                            execute_compiled_plan(self, compiled)
                        } else {
                            self.execute_guard_structure(&def.lines)
                        }
                    } else if let Some(compiled) = plan_set.compiled.as_ref() {
                        if !recursive_reentry
                            && self.should_shadow_validate(plan_set, self.stack.len())
                        {
                            let outcome = self.run_compiled_with_shadow_validation(
                                &resolved_name,
                                &def,
                                plan_set,
                            );
                            if outcome.used_plain_fallback {
                                self.runtime_metrics.hedged_race_fallback_count += 1;
                            }
                            outcome.result
                        } else {
                            execute_compiled_plan(self, compiled)
                        }
                    } else {
                        self.execute_guard_structure(&def.lines)
                    }
                } else if let Some(compiled) = plan_set.compiled.as_ref() {
                    if !recursive_reentry && self.should_shadow_validate(plan_set, self.stack.len())
                    {
                        let outcome = self.run_compiled_with_shadow_validation(
                            &resolved_name,
                            &def,
                            plan_set,
                        );
                        if outcome.used_plain_fallback {
                            self.runtime_metrics.hedged_race_fallback_count += 1;
                        }
                        outcome.result
                    } else {
                        execute_compiled_plan(self, compiled)
                    }
                } else {
                    self.execute_guard_structure(&def.lines)
                }
            } else {
                self.execute_guard_structure(&def.lines)
            };

            if body_result.is_ok() && self.tail_jump_pending {
                self.tail_jump_pending = false;
                // The backward jump still consumes one execution step, so an
                // unbounded guarded loop terminates via `ExecutionLimitExceeded`
                // rather than running forever (SPEC §5.3 water level).
                self.runtime_metrics.tail_call_jump_count += 1;
                self.execution_step_count += 1;
                if self.execution_step_count > self.max_execution_steps {
                    break Err(AjisaiError::ExecutionLimitExceeded {
                        limit: self.max_execution_steps,
                    });
                }
                continue;
            }
            break body_result;
        };

        self.tail_jump_pending = false;
        self.in_tail_context = prev_in_tail;
        self.tail_self_word = prev_tail_self;

        self.call_stack.pop();
        self.owning_dictionary_context = prev_owning;
        self.call_depth -= 1;
        result
    }

    pub(crate) fn execute_builtin(&mut self, name: &str) -> Result<()> {
        let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
        if canonical != "DEL" && canonical != "DEF" && canonical != "FORC" {
            self.force_flag = false;
        }

        self.execute_builtin_direct(canonical.as_ref())
    }

    pub(crate) fn execute_builtin_direct(&mut self, name: &str) -> Result<()> {
        if let Some(spec) = lookup_builtin_spec(name) {
            if let Some(executor_key) = spec.executor_key {
                return self.execute_builtin_by_key(executor_key);
            }
        }

        modules::execute_module_word(self, name)
            .unwrap_or_else(|| Err(AjisaiError::UnknownWord(name.to_string())))
    }

    pub(crate) fn execute_builtin_by_key(&mut self, key: BuiltinExecutorKey) -> Result<()> {
        match key {
            BuiltinExecutorKey::Add => arithmetic::op_add(self),
            BuiltinExecutorKey::Sub => arithmetic::op_sub(self),
            BuiltinExecutorKey::Mul => arithmetic::op_mul(self),
            BuiltinExecutorKey::Div => arithmetic::op_div(self),
            BuiltinExecutorKey::Eq => comparison::op_eq(self),
            BuiltinExecutorKey::Lt => comparison::op_lt(self),
            BuiltinExecutorKey::Le => comparison::op_le(self),
            BuiltinExecutorKey::Gt => comparison::op_gt(self),
            BuiltinExecutorKey::Gte => comparison::op_gte(self),
            BuiltinExecutorKey::Neq => comparison::op_neq(self),
            BuiltinExecutorKey::CompareWithin => comparison::op_compare_within(self),
            BuiltinExecutorKey::Map => higher_order::op_map(self),
            BuiltinExecutorKey::Filter => higher_order::op_filter(self),
            BuiltinExecutorKey::Fold => higher_order_fold::op_fold(self),
            BuiltinExecutorKey::Unfold => higher_order_fold::op_unfold(self),
            BuiltinExecutorKey::Any => higher_order::op_any(self),
            BuiltinExecutorKey::All => higher_order::op_all(self),
            BuiltinExecutorKey::Count => higher_order::op_count(self),
            BuiltinExecutorKey::Scan => higher_order_fold::op_scan(self),
            BuiltinExecutorKey::Get => vector_ops::op_get(self),
            BuiltinExecutorKey::Length => vector_ops::op_length(self),
            BuiltinExecutorKey::Concat => vector_ops::op_concat(self),
            BuiltinExecutorKey::And => logic::op_and(self),
            BuiltinExecutorKey::Or => logic::op_or(self),
            BuiltinExecutorKey::Not => logic::op_not(self),
            BuiltinExecutorKey::True => {
                self.stack.push(Value::from_bool(true));
                self.semantic_registry.push_hint(Interpretation::TruthValue);
                Ok(())
            }
            BuiltinExecutorKey::False => {
                self.stack.push(Value::from_bool(false));
                self.semantic_registry.push_hint(Interpretation::TruthValue);
                Ok(())
            }
            BuiltinExecutorKey::Nil => {
                self.stack.push(Value::nil());
                self.semantic_registry.push_hint(Interpretation::Nil);
                Ok(())
            }
            BuiltinExecutorKey::Idle => Ok(()),
            BuiltinExecutorKey::Exec => control::op_exec(self),
            BuiltinExecutorKey::Eval => control::op_eval(self),
            BuiltinExecutorKey::Cond => control_cond::op_cond(self),
            BuiltinExecutorKey::Def => execute_def::op_def(self),
            BuiltinExecutorKey::Del => execute_del::op_del(self),
            BuiltinExecutorKey::Lookup => execute_lookup::op_lookup(self),
            BuiltinExecutorKey::Import => modules::op_import(self),
            BuiltinExecutorKey::ImportOnly => modules::op_import_only(self),
            BuiltinExecutorKey::Unimport => modules::op_unimport(self),
            BuiltinExecutorKey::UnimportOnly => modules::op_unimport_only(self),
            BuiltinExecutorKey::Force => {
                self.force_flag = true;
                Ok(())
            }
            BuiltinExecutorKey::Print => io::op_print(self),
            BuiltinExecutorKey::Insert => vector_ops::op_insert(self),
            BuiltinExecutorKey::Replace => vector_ops::op_replace(self),
            BuiltinExecutorKey::Remove => vector_ops::op_remove(self),
            BuiltinExecutorKey::Take => vector_ops::op_take(self),
            BuiltinExecutorKey::Split => vector_ops::op_split(self),
            BuiltinExecutorKey::Reverse => vector_ops::op_reverse(self),
            BuiltinExecutorKey::Range => vector_ops::op_range(self),
            BuiltinExecutorKey::Reorder => vector_ops::op_reorder(self),
            BuiltinExecutorKey::Collect => vector_ops::op_collect(self),
            BuiltinExecutorKey::Shape => tensor_cmds::op_shape(self),
            BuiltinExecutorKey::Rank => tensor_cmds::op_rank(self),
            BuiltinExecutorKey::Reshape => tensor_cmds::op_reshape(self),
            BuiltinExecutorKey::Transpose => tensor_cmds::op_transpose(self),
            BuiltinExecutorKey::Fill => tensor_cmds::op_fill(self),
            BuiltinExecutorKey::Floor => tensor_cmds::op_floor(self),
            BuiltinExecutorKey::Ceil => tensor_cmds::op_ceil(self),
            BuiltinExecutorKey::Round => tensor_cmds::op_round(self),
            BuiltinExecutorKey::Quantize => tensor_cmds::op_quantize(self),
            BuiltinExecutorKey::QuantizeHalfAway => tensor_cmds::op_quantize_half_away(self),
            BuiltinExecutorKey::QuantizeFloor => tensor_cmds::op_quantize_floor(self),
            BuiltinExecutorKey::QuantizeCeil => tensor_cmds::op_quantize_ceil(self),
            BuiltinExecutorKey::QuantizeTrunc => tensor_cmds::op_quantize_trunc(self),
            BuiltinExecutorKey::Conserve => tensor_cmds::op_conserve(self),
            BuiltinExecutorKey::Mod => tensor_cmds::op_mod(self),
            BuiltinExecutorKey::Str => cast::op_str(self),
            BuiltinExecutorKey::Num => cast::op_num(self),
            BuiltinExecutorKey::Bool => cast::op_bool(self),
            BuiltinExecutorKey::Chr => cast::op_chr(self),
            BuiltinExecutorKey::Chars => cast::op_chars(self),
            BuiltinExecutorKey::Join => cast::op_join(self),
            BuiltinExecutorKey::Trim => cast::op_trim(self),
            BuiltinExecutorKey::TrimLeft => cast::op_trim_left(self),
            BuiltinExecutorKey::TrimRight => cast::op_trim_right(self),
            BuiltinExecutorKey::Tokenize => cast::op_tokenize(self),
            BuiltinExecutorKey::Substitute => cast::op_substitute(self),
            BuiltinExecutorKey::StartsWith => cast::op_starts_with(self),
            BuiltinExecutorKey::EndsWith => cast::op_ends_with(self),
            BuiltinExecutorKey::Spawn => self.op_spawn(),
            BuiltinExecutorKey::Await => self.op_await(),
            BuiltinExecutorKey::Status => self.op_status(),
            BuiltinExecutorKey::Kill => self.op_kill(),
            BuiltinExecutorKey::Monitor => self.op_monitor(),
            BuiltinExecutorKey::Supervise => self.op_supervise(),
            BuiltinExecutorKey::NilCheck => nil_diagnostics::op_nil_check(self),
            BuiltinExecutorKey::NilReason => nil_diagnostics::op_nil_reason(self),
            BuiltinExecutorKey::NilOrigin => nil_diagnostics::op_nil_origin(self),
            BuiltinExecutorKey::NilRecoverable => nil_diagnostics::op_nil_recoverable(self),
            BuiltinExecutorKey::NilDiagnosis => nil_diagnostics::op_nil_diagnosis(self),
            BuiltinExecutorKey::ToCf => interval_ops::op_to_cf(self),
            BuiltinExecutorKey::Precompute => Err(AjisaiError::from(
                "PRECOMPUTE can only be used during definition-time precomputation",
            )),
        }
    }

    fn get_execution_plan_set(
        &mut self,
        resolved_name: &str,
        def: &std::sync::Arc<crate::types::WordDefinition>,
    ) -> Option<std::sync::Arc<super::execution_plan_set::ExecutionPlanSet>> {
        if def.lines.is_empty() {
            return None;
        }

        if let Some(existing) = def.execution_plans.as_ref() {
            let compiled_valid = existing
                .compiled
                .as_ref()
                .map(|p| is_plan_valid(p, self))
                .unwrap_or(false);

            let quant_valid = existing
                .quantized
                .as_ref()
                .map(|q| {
                    q.guard_signature.dictionary_epoch == self.dictionary_epoch
                        && q.guard_signature.module_epoch == self.module_epoch
                })
                .unwrap_or(false);

            if compiled_valid || quant_valid {
                self.runtime_metrics.compiled_plan_cache_hit_count += 1;
                return Some(existing.clone());
            }
        }

        self.runtime_metrics.compiled_plan_cache_miss_count += 1;

        let mut set =
            super::execution_plan_set::ExecutionPlanSet::new(self.current_epoch_snapshot());

        // Phase 5: reuse the word's compiled plan from the cross-reset artifact
        // store when its content identity matches, otherwise compile and store
        // it. See `build_or_reuse_compiled_plan` for the reuse/rebuild contract.
        set.compiled = self.build_or_reuse_compiled_plan(resolved_name, def);

        if !self.force_no_quant && def.lines.len() == 1 {
            let tokens: Vec<_> = def.lines[0].body_tokens.iter().cloned().collect();
            if let Some(qb) = super::quantized_block::quantize_code_block(&tokens, self) {
                set.quantized = Some(std::sync::Arc::new(qb));
            }
        }

        let set_arc = std::sync::Arc::new(set);
        self.store_execution_plan_set_for_word(resolved_name, set_arc.clone());
        Some(set_arc)
    }

    fn store_execution_plan_set_for_word(
        &mut self,
        resolved_name: &str,
        plan_set: std::sync::Arc<super::execution_plan_set::ExecutionPlanSet>,
    ) {
        if let Some((ns, word)) = resolved_name.split_once('@') {
            if let Some(dict) = self.user_dictionaries.get_mut(ns) {
                if let Some(old_def) = dict.words.get(word).cloned() {
                    let mut updated = (*old_def).clone();
                    updated.execution_plans = Some(plan_set.clone());
                    dict.words
                        .insert(word.to_string(), std::sync::Arc::new(updated));
                    self.sync_user_words_cache();
                    return;
                }
            }
            if let Some(module) = self.module_vocabulary.get_mut(ns) {
                let qualified = format!("{}@{}", ns, word);
                if let Some(old_def) = module.words.get(&qualified).cloned() {
                    let mut updated = (*old_def).clone();
                    updated.execution_plans = Some(plan_set);
                    module.words.insert(qualified, std::sync::Arc::new(updated));
                }
            }
        }
    }

    pub(crate) fn format_token_to_string(&self, token: &Token) -> String {
        match token {
            Token::Number(n) => n.to_string(),
            Token::String(s) => format!("'{}'", s),
            Token::Symbol(s) => s.to_string(),
            Token::VectorStart => "[".to_string(),
            Token::VectorEnd => "]".to_string(),
            Token::BlockStart => "{".to_string(),
            Token::BlockEnd => "}".to_string(),
            Token::Pipeline => "~".to_string(),
            Token::NilCoalesce => "^".to_string(),
            Token::CondClauseSep => "|".to_string(),
            Token::LineBreak => "\n".to_string(),
        }
    }

    pub fn lookup_word_definition_tokens(&self, name: &str) -> Option<String> {
        let (_, def) = self.resolve_word_entry_readonly(name)?;
        if def.is_builtin || def.lines.is_empty() {
            return None;
        }

        let mut result = String::new();
        for (i, line) in def.lines.iter().enumerate() {
            if i > 0 {
                result.push('\n');
            }
            for token in line.body_tokens.iter() {
                result.push_str(&self.format_token_to_string(token));
                result.push(' ');
            }
        }
        Some(result.trim().to_string())
    }
}
