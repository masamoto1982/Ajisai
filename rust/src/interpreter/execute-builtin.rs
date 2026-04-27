use crate::builtins::{lookup_builtin_spec, BuiltinExecutorKey};
use crate::elastic::ElasticMode;
use crate::error::{AjisaiError, Result};
use crate::types::fraction::Fraction;
use crate::types::{DisplayHint, FlowToken, Token, Value};

use super::compiled_plan::{
    arc_plan, compile_word_definition, execute_compiled_plan, is_plan_valid, plan_is_all_fallback,
};

use super::{
    arithmetic, cast, comparison, control, control_cond, execute_def, execute_del, execute_lookup,
    higher_order, higher_order_fold, interval_ops, io, logic, modules, tensor_cmds, vector_ops,
    Interpreter,
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
    pub(crate) fn is_hedged_mode(&self) -> bool {
        matches!(
            self.elastic_mode(),
            ElasticMode::HedgedSafe | ElasticMode::HedgedTrace
        )
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
        let name = canonical_name.as_str();
        if self.resolve_word_entry(name).is_none() {
            if let Some(alias) = super::deprecated_core_aliases::lookup_deprecated_core_alias(name)
            {
                self.execution_step_count += 1;
                if self.execution_step_count > self.max_execution_steps {
                    return Err(AjisaiError::ExecutionLimitExceeded {
                        limit: self.max_execution_steps,
                    });
                }
                self.call_stack
                    .push(alias.replacement_qualified.to_string());
                self.output_buffer.push_str(&format!(
                    "Warning: '{}' is deprecated. Use {} instead.\n",
                    alias.old_name, alias.import_hint
                ));
                let alias_result = self.execute_builtin(alias.replacement_qualified);
                self.call_stack.pop();
                return alias_result;
            }
        }

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

        let plan_set = self.get_execution_plan_set(&resolved_name, &def);

        self.call_stack.push(resolved_name.clone());
        let result = if let Some(plan_set) = plan_set {
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
                    if self.should_shadow_validate(&plan_set, self.stack.len()) {
                        let outcome = self.run_compiled_with_shadow_validation(
                            &resolved_name,
                            &def,
                            &plan_set,
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
                if self.should_shadow_validate(&plan_set, self.stack.len()) {
                    let outcome =
                        self.run_compiled_with_shadow_validation(&resolved_name, &def, &plan_set);
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
        self.call_stack.pop();
        result
    }

    pub(crate) fn execute_builtin(&mut self, name: &str) -> Result<()> {
        let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
        if canonical != "DEL" && canonical != "DEF" && canonical != "FORC" {
            self.force_flag = false;
        }

        let pre_snapshot = if self.flow_tracking {
            Some(self.collect_stack_totals_snapshot())
        } else {
            None
        };

        let result = self.execute_builtin_with_conservation(canonical.as_str());

        if let Some(pre) = pre_snapshot {
            if result.is_ok() {
                let post = self.collect_stack_totals_snapshot();
                let _delta = post.sub(&pre);
            }
        }

        result
    }

    pub(crate) fn collect_stack_totals_snapshot(&self) -> Fraction {
        let mut total = Fraction::from(0);
        for val in &self.stack {
            let token = FlowToken::from_value(val);
            total = total.add(&token.total);
        }
        total
    }

    pub(crate) fn execute_builtin_with_conservation(&mut self, name: &str) -> Result<()> {
        if let Some(spec) = lookup_builtin_spec(name) {
            if let Some(executor_key) = spec.executor_key {
                return self.execute_builtin_by_key(executor_key);
            }
        }

        modules::execute_module_word(self, name)
            .unwrap_or_else(|| Err(AjisaiError::UnknownWord(name.to_string())))
    }

    fn execute_builtin_by_key(&mut self, key: BuiltinExecutorKey) -> Result<()> {
        match key {
            BuiltinExecutorKey::Add => arithmetic::op_add(self),
            BuiltinExecutorKey::Sub => arithmetic::op_sub(self),
            BuiltinExecutorKey::Mul => arithmetic::op_mul(self),
            BuiltinExecutorKey::Div => arithmetic::op_div(self),
            BuiltinExecutorKey::Eq => comparison::op_eq(self),
            BuiltinExecutorKey::Lt => comparison::op_lt(self),
            BuiltinExecutorKey::Le => comparison::op_le(self),
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
                self.semantic_registry.push_hint(DisplayHint::Boolean);
                Ok(())
            }
            BuiltinExecutorKey::False => {
                self.stack.push(Value::from_bool(false));
                self.semantic_registry.push_hint(DisplayHint::Boolean);
                Ok(())
            }
            BuiltinExecutorKey::Nil => {
                self.stack.push(Value::nil());
                self.semantic_registry.push_hint(DisplayHint::Nil);
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
            BuiltinExecutorKey::Sqrt => interval_ops::op_sqrt(self),
            BuiltinExecutorKey::SqrtEps => interval_ops::op_sqrt_eps(self),
            BuiltinExecutorKey::Interval => interval_ops::op_interval(self),
            BuiltinExecutorKey::Lower => interval_ops::op_lower(self),
            BuiltinExecutorKey::Upper => interval_ops::op_upper(self),
            BuiltinExecutorKey::Width => interval_ops::op_width(self),
            BuiltinExecutorKey::IsExact => interval_ops::op_is_exact(self),
            BuiltinExecutorKey::Mod => tensor_cmds::op_mod(self),
            BuiltinExecutorKey::Str => cast::op_str(self),
            BuiltinExecutorKey::Num => cast::op_num(self),
            BuiltinExecutorKey::Bool => cast::op_bool(self),
            BuiltinExecutorKey::Chr => cast::op_chr(self),
            BuiltinExecutorKey::Chars => cast::op_chars(self),
            BuiltinExecutorKey::Join => cast::op_join(self),
            BuiltinExecutorKey::Spawn => self.op_spawn(),
            BuiltinExecutorKey::Await => self.op_await(),
            BuiltinExecutorKey::Status => self.op_status(),
            BuiltinExecutorKey::Kill => self.op_kill(),
            BuiltinExecutorKey::Monitor => self.op_monitor(),
            BuiltinExecutorKey::Supervise => self.op_supervise(),
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

        let compiled = compile_word_definition(def, self);
        if !plan_is_all_fallback(&compiled) {
            self.bump_execution_epoch();
            self.runtime_metrics.compiled_plan_build_count += 1;
            set.compiled = Some(arc_plan(compiled));
        }

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
                if let Some(old_def) = module.sample_words.get(word).cloned() {
                    let mut updated = (*old_def).clone();
                    updated.execution_plans = Some(plan_set.clone());
                    module
                        .sample_words
                        .insert(word.to_string(), std::sync::Arc::new(updated));
                    return;
                }
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
            Token::Pipeline => "==".to_string(),
            Token::NilCoalesce => "=>".to_string(),
            Token::CondClauseSep => "$".to_string(),
            Token::SafeMode => "~".to_string(),
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
