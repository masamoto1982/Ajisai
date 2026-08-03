use crate::error::{AjisaiError, Result};
use crate::kernel::generated::{generated_word, Arity, GeneratedWord, NilPolicy, WordId};
use crate::types::{Interpretation, Token, Value};

use super::compiled_plan::{execute_compiled_plan, is_plan_valid};

use super::{
    algo_ops, arithmetic, cast, comparison, control, control_cond, execute_def, execute_del,
    execute_lookup, higher_order, higher_order_fold, io, logic, math_ops, nil_diagnostics,
    reflection, sort, tensor_cmds, vector_ops, ConsumptionMode, Interpreter,
};

/// What a Word's declared `nilPolicy` requires of the operands on the stack,
/// decided before its primitive is reached.
///
/// The passthrough arm names the bubble by its stack position rather than
/// carrying the value: the decision is made from a borrow of the stack, and a
/// position keeps the whole enum a couple of words wide.
enum NilContract {
    /// The declaration places no obligation here; run the primitive.
    Run,
    /// A NIL operand is malformed use for this Word.
    Reject,
    /// A NIL operand is the Word's result; the bubble flows through in place
    /// of running the primitive. `operands` is the declared operand window to
    /// unwind, `bubble` the stack index of the NIL that becomes the result.
    PassThrough { operands: usize, bubble: usize },
}

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
        self.call_depth -= 1;
        result
    }

    pub(crate) fn execute_builtin(&mut self, name: &str) -> Result<()> {
        let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
        self.execute_builtin_direct(canonical.as_ref())
    }

    /// What the Word's declared NIL contract dictates for the operands
    /// currently on the stack.
    ///
    /// `spec/words.json` declares, per Word, what a NIL operand means. Until
    /// recently each executor decided that for itself, so the declaration was
    /// decorative: `LENGTH` answered `0` for the length of a NIL while
    /// declaring `rejectNil`, and `SORT` raised an error while declaring
    /// `passthrough`. Both directions of that drift are settled here, in one
    /// place that reads the declaration, so no executor can quietly disagree
    /// with the canon.
    ///
    /// The guard reads the declaration and nothing else — no per-family
    /// exception table. A Word whose arity is data-dependent carries no fixed
    /// operand window, so it is left to its executor.
    fn declared_nil_contract(&self, word: &GeneratedWord) -> NilContract {
        let Arity::Fixed(arity) = word.stack_inputs else {
            return NilContract::Run;
        };
        let arity = arity as usize;
        let operands = self.stack.as_slice();

        match word.nil_policy {
            // `rejectNil` binds every operand position, not just the receiver,
            // so a NIL anywhere in the declared arity is malformed use. The
            // window is clamped rather than required: refusing to run touches
            // nothing, so a short stack can be judged on what it holds.
            NilPolicy::RejectNil => {
                let start = operands.len().saturating_sub(arity);
                if operands[start..].iter().any(|operand| operand.is_nil()) {
                    NilContract::Reject
                } else {
                    NilContract::Run
                }
            }
            // A NIL operand *is* the result: the bubble flows downstream
            // carrying its reason (SPEC §7.12, LANG.FAILURE.PASSTHROUGH).
            // `passthroughThenProject` differs only in what non-NIL operands
            // may yield, so a NIL input takes the same route — projecting an
            // absence leaves an absence.
            //
            // Unlike rejection this synthesises a result and unwinds the
            // operands, which needs the whole window present; a short stack is
            // an arity fault, left to the executor to report as underflow.
            NilPolicy::Passthrough | NilPolicy::PassthroughThenProject => {
                if operands.len() < arity {
                    return NilContract::Run;
                }
                // The leftmost bubble wins, matching left-to-right evaluation
                // order and the executor-level helpers it replaces.
                let window = operands.len() - arity;
                match operands[window..]
                    .iter()
                    .position(|operand| operand.is_nil())
                {
                    Some(offset) => NilContract::PassThrough {
                        operands: arity,
                        bubble: window + offset,
                    },
                    None => NilContract::Run,
                }
            }
            // `createsNil` and `preserveReason` describe what the Word does
            // with non-NIL operands; `consumeNil` and `inspectNil` make the NIL
            // itself the Word's subject. None of them constrain dispatch.
            NilPolicy::CreatesNil
            | NilPolicy::ConsumeNil
            | NilPolicy::InspectNil
            | NilPolicy::PreserveReason => NilContract::Run,
        }
    }

    /// Yield the NIL at stack index `bubble` as the Word's result without
    /// running its primitive, unwinding the declared operand window under the
    /// active consumption mode (SPEC §5.2): `EAT` removes the operands, `KEEP`
    /// leaves them in place. The bubble is copied out before the unwind, since
    /// the unwind is what removes it.
    fn pass_nil_through(&mut self, operands: usize, bubble: usize) {
        let result = Value::nil_inheriting_absence_from(&self.stack[bubble]);
        if self.consumption_mode == ConsumptionMode::Consume {
            let remaining = self.stack.len() - operands;
            self.stack.drain(remaining..);
        }
        self.stack.push(result);
    }

    /// Settle the Word's declared NIL contract against the current stack.
    ///
    /// `None` means the declaration places no obligation here and the primitive
    /// must run; `Some(result)` is the Word's outcome, decided without running
    /// it. Every dispatch path must consult this — a path that skips it is a
    /// path on which the declaration is decorative again.
    pub(super) fn apply_declared_nil_contract(
        &mut self,
        word: &GeneratedWord,
    ) -> Option<Result<()>> {
        match self.declared_nil_contract(word) {
            NilContract::Run => None,
            NilContract::Reject => Some(Err(AjisaiError::create_structure_error("a value", "NIL"))),
            NilContract::PassThrough { operands, bubble } => {
                self.pass_nil_through(operands, bubble);
                Some(Ok(()))
            }
        }
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
            WordId::Reflect => reflection::op_reflect(self),
            WordId::Cond => control_cond::op_cond(self),
            WordId::Def => execute_def::op_def(self),
            WordId::Del => execute_del::op_del(self),
            WordId::Lookup => execute_lookup::op_lookup(self),
            WordId::Print => io::op_print(self),
            WordId::Insert => vector_ops::op_insert(self),
            WordId::Replace => vector_ops::op_replace(self),
            WordId::Remove => vector_ops::op_remove(self),
            WordId::Take => vector_ops::op_take(self),
            WordId::Split => vector_ops::op_split(self),
            WordId::Reverse => vector_ops::op_reverse(self),
            WordId::Range => vector_ops::op_range(self),
            WordId::Reorder => vector_ops::op_reorder(self),
            WordId::Collect => vector_ops::op_collect(self),
            WordId::Fill => tensor_cmds::op_fill(self),
            WordId::Floor => tensor_cmds::op_floor(self),
            WordId::Ceil => tensor_cmds::op_ceil(self),
            WordId::Round => tensor_cmds::op_round(self),
            WordId::Mod => tensor_cmds::op_mod(self),
            WordId::Str => cast::op_str(self),
            WordId::Num => cast::op_num(self),
            WordId::Chr => cast::op_chr(self),
            WordId::Chars => cast::op_chars(self),
            WordId::Join => cast::op_join(self),
            WordId::Trim => cast::op_trim(self),
            WordId::Tokenize => cast::op_tokenize(self),
            WordId::Substitute => cast::op_substitute(self),
            WordId::StartsWith => cast::op_starts_with(self),
            WordId::EndsWith => cast::op_ends_with(self),
            WordId::NilCheck => nil_diagnostics::op_nil_check(self),
            WordId::NilReason => nil_diagnostics::op_nil_reason(self),
            WordId::Abs => math_ops::op_abs(self),
            WordId::Neg => math_ops::op_neg(self),
            WordId::Sign => math_ops::op_sign(self),
            WordId::Min => math_ops::op_min(self),
            WordId::Max => math_ops::op_max(self),
            WordId::Sqrt => math_ops::op_sqrt(self),
            WordId::Sort => sort::op_sort(self),
            WordId::Unique => algo_ops::op_unique(self),
            WordId::Contains => algo_ops::op_contains(self),
            WordId::IndexOf => algo_ops::op_index_of(self),
            // The positional control directives of SPEC §6.4. The execution
            // loop interprets these against the source stream — `VENT` decides
            // whether the *following source unit* is evaluated, `EAT`/`KEEP`
            // set the consumption mode — so they are never dispatched by name
            // and have no primitive. Reaching one here means a caller bypassed
            // the loop, which is exactly the unknown-word answer the old
            // `executor_key: None` path gave.
            WordId::LazyNextUnitFallback
            | WordId::SetConsumptionConsume
            | WordId::SetConsumptionKeep => {
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
