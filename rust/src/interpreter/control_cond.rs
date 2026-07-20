use std::sync::Arc;

use crate::error::{AjisaiError, Result};
use crate::interpreter::epoch::EpochSnapshot;
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::{Interpretation, SemanticStack, Token, Value, ValueData};

use super::compiled_plan::{execute_compiled_plan, CompiledPlan};

/// One precomputed COND clause: a guard and the body it selects. Token streams
/// are `Arc`-shared so the compiled dispatch (`CompiledOp::CondDispatch`) can
/// reuse the same split every iteration instead of re-collecting and re-cloning
/// the clause blocks off the stack and re-scanning each for `|`.
///
/// `guard_plan` / `body_plan` are compiled sub-plans, attached once at lowering
/// time (when `compiled_clause_enabled`). When present they are run via
/// `execute_compiled_plan` instead of re-interpreting the guard/body token
/// stream on every iteration — the step that finally moves the loop body off
/// the interpreter. They are `None` on the dynamic path and whenever the clause
/// does not fully compile.
#[derive(Debug, Clone)]
pub struct CondClause {
    pub guard: Arc<[Token]>,
    pub body: Arc<[Token]>,
    pub guard_plan: Option<Arc<CompiledPlan>>,
    pub body_plan: Option<Arc<CompiledPlan>>,
}

/// Dynamic entry point: collect the clause blocks the preceding code pushed,
/// split them, then dispatch. This is the path the plain interpreter and any
/// non-lowered `COND` take.
pub(crate) fn op_cond(interp: &mut Interpreter) -> Result<()> {
    // Tail position of the enclosing word, if any (set by the compiled-plan
    // tail op). Guards must run as non-tail (they may call the same word in a
    // non-tail position), so clear it here and hand it only to the winning
    // clause body, where a tail self-call becomes an internal backward jump.
    let tail_context: bool = std::mem::replace(&mut interp.in_tail_context, false);
    let blocks = collect_top_code_blocks(interp);
    let clauses = split_clause_blocks(blocks)?;
    run_cond_core(interp, &clauses, tail_context)
}

/// Compiled entry point: the clauses were split once at compile time. Collect
/// the blocks the kept `PushCodeBlock` ops pushed so stack discipline is
/// preserved; when their count matches the precomputed set they are exactly
/// those blocks, so dispatch on the precomputed clauses (no clone, no re-split).
/// Otherwise (an unexpected extra block reached the stack) fall back to the
/// dynamic split of the actual blocks — keeping behavior identical to `op_cond`.
pub(crate) fn op_cond_dispatch(interp: &mut Interpreter, precomputed: &[CondClause]) -> Result<()> {
    let tail_context: bool = std::mem::replace(&mut interp.in_tail_context, false);
    let blocks = collect_top_code_blocks(interp);
    if blocks.len() == precomputed.len() && !blocks.is_empty() {
        interp.runtime_metrics.cond_dispatch_fast_count += 1;
        run_cond_core(interp, precomputed, tail_context)
    } else {
        let clauses = split_clause_blocks(blocks)?;
        run_cond_core(interp, &clauses, tail_context)
    }
}

/// Pop the target value and dispatch over `clauses`, running the first clause
/// whose guard fires (or the `IDLE` else-clause). Shared by both entry points
/// so dynamic and compiled COND are behaviorally identical.
fn run_cond_core(
    interp: &mut Interpreter,
    clauses: &[CondClause],
    tail_context: bool,
) -> Result<()> {
    let target_value: Value = match interp.consumption_mode {
        ConsumptionMode::Consume => {
            let val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let _ = interp.semantic_registry.pop_hint();
            val
        }
        ConsumptionMode::Keep => interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?,
    };

    let mut else_clause: Option<&CondClause> = None;

    // Hedged guard prefetch is part of the opt-in elastic engine; when it
    // handles the dispatch it returns from here so the greedy loop below is
    // exactly the code that runs in the default build.
    #[cfg(feature = "elastic-engine")]
    {
        if is_hedged_cond_mode(interp) {
            interp.push_hedged_trace("cond:prefetch-start");
            if let Some(clause) =
                evaluate_guard_hedged_prefetch(interp, clauses, &target_value, &mut else_clause)?
            {
                interp.push_hedged_trace("cond:winner-prefetched-guard");
                return run_clause_body(interp, clause, &target_value, tail_context);
            }
            if let Some(clause) = else_clause {
                return run_clause_body(interp, clause, &target_value, tail_context);
            }
            return Err(AjisaiError::CondExhausted);
        }
    }

    for clause in clauses {
        if is_idle_guard(&clause.guard) {
            else_clause = Some(clause);
            continue;
        }

        if evaluate_guard_greedy(
            interp,
            &clause.guard,
            clause.guard_plan.as_deref(),
            &target_value,
        )? {
            return run_clause_body(interp, clause, &target_value, tail_context);
        }
    }

    if let Some(clause) = else_clause {
        return run_clause_body(interp, clause, &target_value, tail_context);
    }

    Err(AjisaiError::CondExhausted)
}

/// Run a clause's body, preferring its compiled sub-plan when present.
fn run_clause_body(
    interp: &mut Interpreter,
    clause: &CondClause,
    value: &Value,
    tail_context: bool,
) -> Result<()> {
    execute_cond_body(
        interp,
        &clause.body,
        clause.body_plan.as_deref(),
        value,
        tail_context,
    )
}

/// Pop the consecutive code blocks on top of the stack, moving their token
/// vectors out (no clone). Returns them bottom-to-top, matching source order.
fn collect_top_code_blocks(interp: &mut Interpreter) -> Vec<Vec<Token>> {
    let mut blocks: Vec<Vec<Token>> = Vec::new();
    while interp
        .stack
        .last()
        .is_some_and(|v| matches!(v.data, ValueData::CodeBlock(_)))
    {
        let value = interp.stack.pop().expect("checked by last()");
        let _ = interp.semantic_registry.pop_hint();
        if let ValueData::CodeBlock(tokens) = value.data {
            blocks.push(tokens);
        }
    }
    blocks.reverse();
    blocks
}

/// Split collected clause blocks into guards and bodies, validating clause
/// style. Pure over the blocks, so the compiler can precompute the result.
pub(crate) fn split_clause_blocks(blocks: Vec<Vec<Token>>) -> Result<Vec<CondClause>> {
    if blocks.is_empty() {
        return Err(AjisaiError::from(
            "COND: expected guard/body clauses, got 0 code blocks",
        ));
    }

    let has_sep_flags: Vec<bool> = blocks
        .iter()
        .map(|block| block.iter().any(|t| matches!(t, Token::CondClauseSep)))
        .collect();
    let all_with_sep: bool = has_sep_flags.iter().all(|f| *f);
    let none_with_sep: bool = has_sep_flags.iter().all(|f| !*f);

    if !all_with_sep && !none_with_sep {
        return Err(AjisaiError::from(
            "COND: mixed clause styles are not allowed; use either {guard}{body} pairs or {guard | body} clauses consistently",
        ));
    }

    let mut clauses: Vec<CondClause> = Vec::new();
    if all_with_sep {
        for block in &blocks {
            let (guard_tokens, body_tokens) = split_cond_clause_block(block)?;
            clauses.push(CondClause {
                guard: Arc::from(guard_tokens),
                body: Arc::from(body_tokens),
                guard_plan: None,
                body_plan: None,
            });
        }
        return Ok(clauses);
    }

    if !blocks.len().is_multiple_of(2) {
        return Err(AjisaiError::from(format!(
            "COND: expected even number of code blocks (guard/body pairs), got {}",
            blocks.len()
        )));
    }

    let mut blocks = blocks.into_iter();
    while let (Some(guard_tokens), Some(body_tokens)) = (blocks.next(), blocks.next()) {
        clauses.push(CondClause {
            guard: Arc::from(guard_tokens),
            body: Arc::from(body_tokens),
            guard_plan: None,
            body_plan: None,
        });
    }

    Ok(clauses)
}

fn split_cond_clause_block(tokens: &[Token]) -> Result<(Vec<Token>, Vec<Token>)> {
    let separator_indexes: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter_map(|(i, token)| matches!(token, Token::CondClauseSep).then_some(i))
        .collect();

    if separator_indexes.len() != 1 {
        return Err(AjisaiError::from(
            "COND: a | clause must contain exactly one '|' separator",
        ));
    }

    let separator_index: usize = separator_indexes[0];
    if separator_index == 0 || separator_index + 1 >= tokens.len() {
        return Err(AjisaiError::from(
            "COND: both guard and body are required around '|'",
        ));
    }

    let guard_tokens = tokens[..separator_index].to_vec();
    let body_tokens = tokens[(separator_index + 1)..].to_vec();
    Ok((guard_tokens, body_tokens))
}

fn evaluate_guard_isolated(
    interp: &mut Interpreter,
    guard_tokens: &[Token],
    guard_plan: Option<&CompiledPlan>,
    value: &Value,
) -> Result<bool> {
    // Preserve the observable stack as typed slots.  COND isolation is one of
    // the high-risk legacy paths because it previously saved values and roles
    // as independently managed vectors.
    let saved_stack = interp
        .semantic_stack_snapshot()
        .expect("stack values and semantic roles must remain position-aligned");
    let saved_target_mode: OperationTargetMode = interp.operation_target_mode;
    let saved_consumption_mode: ConsumptionMode = interp.consumption_mode;
    let saved_epoch: EpochSnapshot = interp.current_epoch_snapshot();

    interp.stack.clear();
    interp.semantic_registry.clear();
    interp.stack.push(value.clone());
    interp
        .semantic_registry
        .push_hint(Interpretation::Unassigned);
    interp.operation_target_mode = OperationTargetMode::StackTop;
    interp.consumption_mode = ConsumptionMode::Consume;

    // Guards are never tail position; run the compiled sub-plan when available,
    // otherwise interpret the tokens. Both produce the same result value.
    let execution_result: Result<()> = if let Some(plan) = guard_plan {
        interp.runtime_metrics.cond_clause_compiled_count += 1;
        execute_compiled_plan(interp, plan)
    } else {
        interp.execute_section_core(guard_tokens, 0).map(|_| ())
    };
    let guard_result_value: Option<Value> = interp.stack.pop();

    restore_cond_eval_state(
        interp,
        saved_stack,
        saved_target_mode,
        saved_consumption_mode,
        saved_epoch,
    );

    execution_result?;

    let result_value: Value = guard_result_value.ok_or_else(|| {
        AjisaiError::from("COND: guard must return TRUE or FALSE, got empty stack")
    })?;
    // SPEC §7.4.3: a guard that reduces to the logical `Unknown` (U) — e.g.
    // an undecidable continued-fraction comparison — is not a definite
    // `true`, so its clause does not fire. Fall through to the next clause
    // exactly as for a `false` guard. U is neither an error nor a match.
    if result_value.is_unknown() {
        return Ok(false);
    }
    // A definite Boolean guard fires iff it is TRUE (SPEC §7.7). Accept a
    // bare Boolean or one wrapped in a single-element vector; fall back to the
    // legacy numeric-guard handling (0 = false, 1 = true) below otherwise.
    if let Some(b) = result_value.as_truth() {
        return Ok(b);
    }
    if result_value.len() == 1 {
        if let Some(child) = result_value.get_child(0) {
            if let Some(b) = child.as_truth() {
                return Ok(b);
            }
        }
    }
    let unwrapped: &Value = if result_value.as_scalar().is_none() {
        if result_value.len() == 1 {
            result_value.get_child(0).ok_or_else(|| {
                AjisaiError::from("COND: guard must return TRUE or FALSE, got non-scalar")
            })?
        } else {
            return Err(AjisaiError::from(
                "COND: guard must return TRUE or FALSE, got non-scalar",
            ));
        }
    } else {
        &result_value
    };
    let scalar = unwrapped.as_scalar().ok_or_else(|| {
        AjisaiError::from("COND: guard must return TRUE or FALSE, got non-scalar")
    })?;
    if scalar.is_zero() {
        return Ok(false);
    }
    if scalar.to_i64() == Some(1) {
        return Ok(true);
    }

    Err(AjisaiError::from(format!(
        "COND: guard must return TRUE or FALSE, got {}",
        result_value
    )))
}

fn evaluate_guard_greedy(
    interp: &mut Interpreter,
    guard_tokens: &[Token],
    guard_plan: Option<&CompiledPlan>,
    value: &Value,
) -> Result<bool> {
    evaluate_guard_isolated(interp, guard_tokens, guard_plan, value)
}

fn restore_cond_eval_state(
    interp: &mut Interpreter,
    saved_stack: SemanticStack,
    saved_target_mode: OperationTargetMode,
    saved_consumption_mode: ConsumptionMode,
    saved_epoch: EpochSnapshot,
) {
    interp.replace_semantic_stack(saved_stack);
    interp.operation_target_mode = saved_target_mode;
    interp.consumption_mode = saved_consumption_mode;
    interp.dictionary_epoch = saved_epoch.dictionary_epoch;
    interp.module_epoch = saved_epoch.module_epoch;
    interp.execution_epoch = saved_epoch.execution_epoch;
    interp.global_epoch = saved_epoch.global_epoch;
}

#[cfg(feature = "elastic-engine")]
fn is_pure_cond_guard(guard_tokens: &[Token]) -> bool {
    let mut symbols: Vec<String> = Vec::new();
    for token in guard_tokens {
        match token {
            Token::Symbol(s) => symbols.push(s.to_string()),
            Token::Number(_) | Token::String(_) => {}
            Token::LineBreak => {}
            _ => return false,
        }
    }
    crate::elastic::can_hedge_cond_guard(&symbols)
}

#[cfg(feature = "elastic-engine")]
fn is_hedged_cond_mode(interp: &Interpreter) -> bool {
    matches!(
        interp.elastic_mode(),
        crate::elastic::ElasticMode::HedgedSafe | crate::elastic::ElasticMode::HedgedTrace
    )
}

#[cfg(feature = "elastic-engine")]
fn evaluate_guard_hedged_prefetch<'a>(
    interp: &mut Interpreter,
    clauses: &'a [CondClause],
    target_value: &Value,
    else_clause: &mut Option<&'a CondClause>,
) -> Result<Option<&'a CondClause>> {
    let mut prefetched: Vec<Option<Result<bool>>> = vec![None; clauses.len()];
    let mut has_impure_guard = false;

    for (idx, clause) in clauses.iter().enumerate() {
        if is_idle_guard(&clause.guard) {
            continue;
        }
        if is_pure_cond_guard(&clause.guard) {
            interp.runtime_metrics.cond_guard_prefetch_count += 1;
            prefetched[idx] = Some(evaluate_guard_isolated(
                interp,
                &clause.guard,
                clause.guard_plan.as_deref(),
                target_value,
            ));
        } else {
            has_impure_guard = true;
        }
    }
    if has_impure_guard {
        interp.push_hedged_trace("cond:partial-prefetch-impure-guard-present");
    }

    for (idx, clause) in clauses.iter().enumerate() {
        if is_idle_guard(&clause.guard) {
            *else_clause = Some(clause);
            continue;
        }

        let is_true = if let Some(result) = prefetched[idx].clone() {
            result?
        } else {
            evaluate_guard_greedy(
                interp,
                &clause.guard,
                clause.guard_plan.as_deref(),
                target_value,
            )?
        };
        if is_true {
            return Ok(Some(clause));
        }
    }
    Ok(None)
}

fn execute_cond_body(
    interp: &mut Interpreter,
    body_tokens: &[Token],
    body_plan: Option<&CompiledPlan>,
    value: &Value,
    tail_context: bool,
) -> Result<()> {
    let saved_stack = interp
        .semantic_stack_snapshot()
        .expect("stack values and semantic roles must remain position-aligned");
    let saved_target_mode: OperationTargetMode = interp.operation_target_mode;
    let saved_consumption_mode: ConsumptionMode = interp.consumption_mode;

    interp.stack.clear();
    interp.stack.push(value.clone());
    interp.semantic_registry.stack_hints = vec![Interpretation::Unassigned];
    interp.operation_target_mode = OperationTargetMode::StackTop;
    interp.consumption_mode = ConsumptionMode::Consume;

    // This clause body runs in the word's tail position iff the COND itself
    // did. A tail self-call at the end of the body then defers to the
    // trampoline instead of recursing; its residual single value (the next
    // iteration's argument) flows out as this body's result below. The
    // deferral happens in `execute_section_core` (interpreted) or, when the
    // body is compiled, in `execute_compiled_line`'s tail-op handling — both
    // keyed on `in_tail_context` and `tail_self_word`.
    interp.in_tail_context = tail_context;
    let execution_result: Result<()> = if let Some(plan) = body_plan {
        interp.runtime_metrics.cond_clause_compiled_count += 1;
        execute_compiled_plan(interp, plan)
    } else {
        interp.execute_section_core(body_tokens, 0).map(|_| ())
    };
    interp.in_tail_context = false;
    let body_result_hint: Interpretation = interp.semantic_registry.pop_hint();
    let body_result_value: Option<Value> = interp.stack.pop();

    interp.replace_semantic_stack(saved_stack);
    interp.operation_target_mode = saved_target_mode;
    interp.consumption_mode = saved_consumption_mode;

    execution_result?;
    let result_value: Value =
        body_result_value.ok_or_else(|| AjisaiError::from("COND: body must return a value"))?;
    interp.stack.push(result_value);
    interp.semantic_registry.push_hint(body_result_hint);
    Ok(())
}

fn is_idle_guard(guard_tokens: &[Token]) -> bool {
    if guard_tokens.len() != 1 {
        return false;
    }
    matches!(&guard_tokens[0], Token::Symbol(symbol) if symbol.as_ref().eq_ignore_ascii_case("IDLE"))
}
