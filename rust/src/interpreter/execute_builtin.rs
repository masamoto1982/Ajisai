use crate::error::{AjisaiError, Result};
use crate::kernel::generated::{generated_word, WordId};
use crate::types::{Interpretation, Token, Value};

use super::compiled_plan::{execute_compiled_plan, is_plan_valid};

use super::{
    algo_ops, arithmetic, bindings, cast, comparison, control, control_cond, execute_def,
    execute_del, higher_order, higher_order_fold, io, logic, math_ops, nil_diagnostics,
    ordering_ops, probe, shape_ops, sort, tensor_cmds, vector_ops, ConsumptionMode, Interpreter,
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

        // A local binding is read before the dictionary. Nothing can be in both
        // — `BIND` refuses a Word's name and `DEF` refuses a live binding's —
        // so the order settles no contest; it is here because a binding is the
        // cheaper lookup and the more local fact.
        if let Some((value, role)) = self.lookup_binding(&name.to_uppercase()) {
            self.stack.push_with_role(value, role);
            return Ok(());
        }

        let (resolved_name, def) = self.resolve_word_entry(name).ok_or_else(|| {
            let ambiguous = self.check_ambiguity(name);
            if !ambiguous.is_empty() {
                AjisaiError::from(format!(
                    "Ambiguous word '{}': found in {}. Use a qualified path to specify which one you mean.",
                    name.to_uppercase(),
                    ambiguous.join(", ")
                ))
            } else if self.binding_exists_beyond_barrier(&name.to_uppercase()) {
                // The reader can see the name in their own source, so the bare
                // "unknown word" is the least useful true thing to say. What
                // went wrong is the scope, and naming it is the difference
                // between reading the rule and rediscovering it.
                AjisaiError::from(format!(
                    "'{}' is bound in another frame. A binding is reachable in the frame that made it \
                     and in the blocks written there, never inside a Word it calls — pass the value \
                     as an operand instead.",
                    name.to_uppercase()
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

        // Call-depth guard: catches blown Rust stack before WASM traps. Guards
        // a pathologically long acyclic call chain, not recursion — the
        // DEF-time acyclicity check (SPEC §8.7) makes recursion impossible to
        // construct. The matching decrement is just before the return below;
        // there are no `?` early returns between this point and the decrement.
        if self.call_depth + 1 > super::interpreter_core::MAX_USER_WORD_DEPTH {
            return Err(AjisaiError::RecursionLimitExceeded {
                limit: super::interpreter_core::MAX_USER_WORD_DEPTH,
                word: resolved_name.clone(),
            });
        }
        self.call_depth += 1;

        // The caller's stack as the call begins, for the failure record below.
        let stack_len_at_call: usize = self.stack.len();

        let plan_set = self.get_execution_plan_set(&resolved_name, &def);

        self.call_stack.push(resolved_name.clone());

        // `KEEP` modifies the *call*, not the first consuming Word inside the
        // body (SPEC §5.2). Both readings agree for a Core Word, because a Core
        // Word has no inside; they disagree for a User Word, and the body
        // reading is the wrong one — `{ 2 * } 'TWICE' DEF` under `5 KEEP TWICE`
        // let the modifier reach `*`, which then preserved the body's own
        // literal `2` as if the caller had written it. The answer was `5 2 10`
        // with no error and no NIL: a silently wrong result from the one
        // modifier the language has, which is exactly what
        // LANG.FAILURE.TRICHOTOMY rules out.
        //
        // So the modifier is settled here, at the boundary it names. The body
        // runs in the default consuming mode, a depth watch records how far
        // into the stack the call reached, and the operands it ate are put back
        // underneath its results.
        let keep_call = self.consumption_mode == ConsumptionMode::Keep;
        let kept_operands: Option<Vec<(Value, Interpretation)>> = keep_call.then(|| {
            self.stack
                .iter_slots()
                .map(|(value, role)| (value.clone(), role))
                .collect()
        });
        self.consumption_mode = ConsumptionMode::Consume;
        let enclosing_watch = self.stack.begin_depth_watch();

        // A Word call is a barrier frame: its body names its own locals and
        // reads none of the caller's, so what a Word means depends on its
        // operands and its dictionary and nothing else.
        self.open_binding_scope(true);

        // Compiling a body is unobservable (LANG.AUTHORITY.FREEDOM): a run
        // produces the same result whether it went through the compiled plan
        // or the plain guard structure.
        let result = match plan_set.as_ref().and_then(|set| set.compiled.as_ref()) {
            Some(compiled) => execute_compiled_plan(self, compiled),
            None => self.execute_guard_structure(&def.lines),
        };

        self.close_binding_scope();

        let operand_floor = self.stack.end_depth_watch(enclosing_watch);
        if let (Some(operands), true) = (kept_operands, result.is_ok()) {
            self.restore_kept_operands(operands, operand_floor);
        }

        self.call_stack.pop();
        self.call_depth -= 1;

        // A User Word call is where attribution stops. Its body is its own
        // business: from the caller's side the Word is what failed, and that
        // has to read the same whether the body ran compiled or interpreted —
        // the compiled route records nothing from inside a body, so without
        // this the two routes would name different Words for the same failure
        // (LANG.AUTHORITY.FREEDOM: compiling a body is unobservable). Any
        // record the interpreted route already made stays in the trace as
        // detail; this one is the answer.
        if let Err(err) = &result {
            self.record_word_failure(&resolved_name, err, stack_len_at_call);
        }

        result
    }

    /// Put a `KEEP`-ed call's operands back underneath its results.
    ///
    /// After the call the stack is `survivors ++ results`, where `survivors` is
    /// the part below `operand_floor` — the shallowest depth the call reached.
    /// `operands` is the whole stack as it stood before the call, so everything
    /// from `operand_floor` up is what the call ate. Splicing that region back
    /// in leaves `operands ++ results`: operands preserved, result appended.
    fn restore_kept_operands(
        &mut self,
        operands: Vec<(Value, Interpretation)>,
        operand_floor: usize,
    ) {
        if operand_floor >= operands.len() {
            return;
        }
        let results = self.stack.split_off(operand_floor.min(self.stack.len()));
        for (value, role) in operands.into_iter().skip(operand_floor) {
            self.stack.push_with_role(value, role);
        }
        let (values, roles) = results.into_parts();
        for (value, role) in values.into_iter().zip(roles) {
            self.stack.push_with_role(value, role);
        }
    }

    pub(crate) fn execute_builtin(&mut self, name: &str) -> Result<()> {
        let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
        self.execute_builtin_direct(canonical.as_ref())
    }

    pub(crate) fn execute_builtin_direct(&mut self, name: &str) -> Result<()> {
        let Some(word) = generated_word(name) else {
            return Err(AjisaiError::UnknownWord(name.to_string()));
        };
        if let Some(decided) = self.apply_declared_nil_contract(word) {
            return decided;
        }
        self.execute_builtin_by_id(word.id)
    }

    /// Run the primitive for a Word's canonical identity.
    ///
    /// `WordId` is generated from `spec/words.json`, so this match is total
    /// over the Words the specification declares: adding one to the canon
    /// fails the build here rather than at runtime, which is the registration
    /// gap the old hand-written key enum could not catch.
    pub(crate) fn execute_builtin_by_id(&mut self, id: WordId) -> Result<()> {
        match id {
            WordId::Add => arithmetic::op_add(self),
            WordId::Sub => arithmetic::op_sub(self),
            WordId::Mul => arithmetic::op_mul(self),
            WordId::Div => arithmetic::op_div(self),
            WordId::Eq => comparison::op_eq(self),
            WordId::Lt => comparison::op_lt(self),
            WordId::Le => comparison::op_le(self),
            WordId::Gt => comparison::op_gt(self),
            WordId::Gte => comparison::op_gte(self),
            WordId::Neq => comparison::op_neq(self),
            WordId::Map => higher_order::op_map(self),
            WordId::Filter => higher_order::op_filter(self),
            WordId::Fold => higher_order_fold::op_fold(self),
            WordId::Any => higher_order::op_any(self),
            WordId::All => higher_order::op_all(self),
            WordId::Get => vector_ops::op_get(self),
            WordId::Length => vector_ops::op_length(self),
            WordId::Concat => vector_ops::op_concat(self),
            WordId::And => logic::op_and(self),
            WordId::Or => logic::op_or(self),
            WordId::Not => logic::op_not(self),
            WordId::True => {
                self.stack
                    .push_with_role(Value::from_bool(true), Interpretation::TruthValue);
                Ok(())
            }
            WordId::False => {
                self.stack
                    .push_with_role(Value::from_bool(false), Interpretation::TruthValue);
                Ok(())
            }
            WordId::Nil => {
                self.stack.push_with_role(Value::nil(), Interpretation::Nil);
                Ok(())
            }
            WordId::Exec => control::op_exec(self),
            WordId::Probe => probe::op_probe(self),
            WordId::Cond => control_cond::op_cond(self),
            WordId::Bind => bindings::op_bind(self),
            WordId::Def => execute_def::op_def(self),
            WordId::Del => execute_del::op_del(self),
            WordId::Print => io::op_print(self),
            WordId::Take => vector_ops::op_take(self),
            WordId::Reverse => vector_ops::op_reverse(self),
            WordId::Range => vector_ops::op_range(self),
            WordId::Collect => vector_ops::op_collect(self),
            WordId::Fill => tensor_cmds::op_fill(self),
            WordId::Floor => tensor_cmds::op_floor(self),
            WordId::Round => tensor_cmds::op_round(self),
            WordId::Quantize => tensor_cmds::op_quantize(self),
            WordId::Mod => tensor_cmds::op_mod(self),
            WordId::Str => cast::op_str(self),
            WordId::Num => cast::op_num(self),
            WordId::Chars => cast::op_chars(self),
            WordId::Join => cast::op_join(self),
            WordId::Trim => cast::op_trim(self),
            WordId::Tokenize => cast::op_tokenize(self),
            WordId::NilCheck => nil_diagnostics::op_nil_check(self),
            WordId::NilReason => nil_diagnostics::op_nil_reason(self),
            WordId::Abs => math_ops::op_abs(self),
            WordId::Neg => math_ops::op_neg(self),
            WordId::Min => math_ops::op_min(self),
            WordId::Max => math_ops::op_max(self),
            WordId::Sqrt => math_ops::op_sqrt(self),
            WordId::Pi => math_ops::op_pi(self),
            WordId::Sort => sort::op_sort(self),
            WordId::Order => ordering_ops::op_order(self),
            WordId::Unique => ordering_ops::op_unique(self),
            WordId::Tally => ordering_ops::op_tally(self),
            WordId::Group => ordering_ops::op_group(self),
            WordId::Zip => shape_ops::op_zip(self),
            WordId::Sum => shape_ops::op_sum(self),
            WordId::Put => shape_ops::op_put(self),
            WordId::Random => shape_ops::op_random(self),
            WordId::IndexOf => algo_ops::op_index_of(self),
            // The positional control directives of SPEC §6.4. The execution
            // loop interprets these against the source stream — `VENT` decides
            // whether the *following source unit* is evaluated and `KEEP`
            // sets the non-default consumption mode — so they are never dispatched by name
            // and have no primitive. Reaching one here means a caller bypassed
            // the loop, which is exactly the unknown-word answer the old
            // `executor_key: None` path gave.
            WordId::LazyNextUnitFallback | WordId::SetConsumptionKeep => {
                Err(AjisaiError::UnknownWord(self.word_name_for(id).to_string()))
            }
        }
    }

    /// The canonical name of a Word, for a diagnostic that only holds its id.
    fn word_name_for(&self, id: WordId) -> &'static str {
        crate::kernel::generated::GENERATED_WORDS
            .iter()
            .find(|word| word.id == id)
            .map(|word| word.name)
            .unwrap_or("")
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
        if let Some(old_def) = self.user_words.get(resolved_name).cloned() {
            let mut updated = (*old_def).clone();
            updated.execution_plans = Some(plan_set.clone());
            self.user_words
                .insert(resolved_name.to_string(), std::sync::Arc::new(updated));
        }
    }

    pub(crate) fn format_token_to_string(&self, token: &Token) -> String {
        match token {
            Token::Number(n) => n.to_string(),
            Token::String(s) => format!("'{}'", s),
            Token::Symbol(s) => s.to_string(),
            Token::VectorStart => "[".to_string(),
            Token::VectorEnd => "]".to_string(),
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
