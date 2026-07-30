use crate::builtins::{lookup_builtin_spec, BuiltinExecutorKey};
use crate::error::{AjisaiError, Result};
use crate::types::{Interpretation, Token, Value};

use super::compiled_plan::{execute_compiled_plan, is_plan_valid};

use super::{
    algo_ops, arithmetic, cast, comparison, control, control_cond, execute_def, execute_del,
    execute_lookup, higher_order, higher_order_fold, io, logic, math_ops, nil_diagnostics, sort,
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
}

impl Interpreter {
    /// Public entry point for word execution.
    pub(crate) fn execute_word_core(&mut self, name: &str) -> Result<()> {
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

        // Provenance (Phase 6): record the resolved word for the execution
        // receipt. No-op unless receipt recording is enabled.

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

        // The dispatch is inlined into the loop rather than extracted into a
        // helper so the legacy (non-trampolined) recursion path adds no extra
        // native-stack frame per call — important because that path is still
        // bounded only by `MAX_USER_WORD_DEPTH`, and a deeper per-frame cost
        // would lower the effective depth ceiling.
        let result = loop {
            self.in_tail_context = false;
            self.tail_jump_pending = false;

            // Compiling a body is unobservable (LANG.AUTHORITY.FREEDOM): a run
            // produces the same result whether it went through the compiled
            // plan or the plain guard structure.
            let body_result = match plan_set.as_ref().and_then(|set| set.compiled.as_ref()) {
                Some(compiled) => execute_compiled_plan(self, compiled),
                None => self.execute_guard_structure(&def.lines),
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
        Err(AjisaiError::UnknownWord(name.to_string()))
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
            BuiltinExecutorKey::Map => higher_order::op_map(self),
            BuiltinExecutorKey::Filter => higher_order::op_filter(self),
            BuiltinExecutorKey::Fold => higher_order_fold::op_fold(self),
            BuiltinExecutorKey::Any => higher_order::op_any(self),
            BuiltinExecutorKey::All => higher_order::op_all(self),
            BuiltinExecutorKey::Get => vector_ops::op_get(self),
            BuiltinExecutorKey::Length => vector_ops::op_length(self),
            BuiltinExecutorKey::Concat => vector_ops::op_concat(self),
            BuiltinExecutorKey::And => logic::op_and(self),
            BuiltinExecutorKey::Or => logic::op_or(self),
            BuiltinExecutorKey::Not => logic::op_not(self),
            BuiltinExecutorKey::True => {
                self.stack
                    .push_with_role(Value::from_bool(true), Interpretation::TruthValue);
                Ok(())
            }
            BuiltinExecutorKey::False => {
                self.stack
                    .push_with_role(Value::from_bool(false), Interpretation::TruthValue);
                Ok(())
            }
            BuiltinExecutorKey::Nil => {
                self.stack.push_with_role(Value::nil(), Interpretation::Nil);
                Ok(())
            }
            BuiltinExecutorKey::Exec => control::op_exec(self),
            BuiltinExecutorKey::Cond => control_cond::op_cond(self),
            BuiltinExecutorKey::Def => execute_def::op_def(self),
            BuiltinExecutorKey::Del => execute_del::op_del(self),
            BuiltinExecutorKey::Lookup => execute_lookup::op_lookup(self),
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
            BuiltinExecutorKey::Fill => tensor_cmds::op_fill(self),
            BuiltinExecutorKey::Floor => tensor_cmds::op_floor(self),
            BuiltinExecutorKey::Ceil => tensor_cmds::op_ceil(self),
            BuiltinExecutorKey::Round => tensor_cmds::op_round(self),
            BuiltinExecutorKey::Mod => tensor_cmds::op_mod(self),
            BuiltinExecutorKey::Str => cast::op_str(self),
            BuiltinExecutorKey::Num => cast::op_num(self),
            BuiltinExecutorKey::Chr => cast::op_chr(self),
            BuiltinExecutorKey::Chars => cast::op_chars(self),
            BuiltinExecutorKey::Join => cast::op_join(self),
            BuiltinExecutorKey::Trim => cast::op_trim(self),
            BuiltinExecutorKey::Tokenize => cast::op_tokenize(self),
            BuiltinExecutorKey::Substitute => cast::op_substitute(self),
            BuiltinExecutorKey::StartsWith => cast::op_starts_with(self),
            BuiltinExecutorKey::EndsWith => cast::op_ends_with(self),
            BuiltinExecutorKey::NilCheck => nil_diagnostics::op_nil_check(self),
            BuiltinExecutorKey::NilReason => nil_diagnostics::op_nil_reason(self),
            BuiltinExecutorKey::Abs => math_ops::op_abs(self),
            BuiltinExecutorKey::Neg => math_ops::op_neg(self),
            BuiltinExecutorKey::Sign => math_ops::op_sign(self),
            BuiltinExecutorKey::Min => math_ops::op_min(self),
            BuiltinExecutorKey::Max => math_ops::op_max(self),
            BuiltinExecutorKey::Sqrt => math_ops::op_sqrt(self),
            BuiltinExecutorKey::Sort => sort::op_sort(self),
            BuiltinExecutorKey::Unique => algo_ops::op_unique(self),
            BuiltinExecutorKey::Contains => algo_ops::op_contains(self),
            BuiltinExecutorKey::IndexOf => algo_ops::op_index_of(self),
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

            if compiled_valid {
                self.runtime_metrics.compiled_plan_cache_hit_count += 1;
                return Some(existing.clone());
            }
        }

        self.runtime_metrics.compiled_plan_cache_miss_count += 1;

        let mut set =
            super::execution_plan_set::ExecutionPlanSet::new(self.current_epoch_snapshot());

        set.compiled = self.build_or_reuse_compiled_plan(resolved_name, def);

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
