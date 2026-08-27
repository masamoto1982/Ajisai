use std::sync::Arc;

use crate::error::{AjisaiError, Result};
use crate::interpreter::epoch::EpochSnapshot;
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::{Interpretation, Stack, Token, Value};

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

/// Dynamic entry point: `COND` takes its clauses as a single fixed-position
/// operand — a Vector whose elements are each a `[ guard | body ]` (or
/// `[ guard ] [ body ]`-paired) clause block — the same convention
/// `MAP`/`FILTER`/`FOLD` already use for their one code operand
/// (`extract_executable_code`). This is the path the plain interpreter and
/// any non-lowered `COND` take.
///
/// Earlier revisions tried to rediscover a *run* of adjacent bracketed
/// literals immediately before `COND` lexically, the same way the pre-
/// unification runtime used to pop values while they tested as the (now-gone)
/// `CodeBlock` domain. Both approaches fail for the same reason: nothing
/// distinguishes "a clause block" from an ordinary Vector value written or
/// stored right next to it — not a runtime domain (Vector and CodeBlock are
/// one domain now) and not even source spelling (a `DEF`'d body's clause
/// blocks round-trip through `value_as_code.rs` and lose their original `{`
/// vs `[` once they exist as a `Value`). A single fixed-position operand
/// needs neither: `COND` always has exactly two operands, so there is
/// nothing to scan for.
pub(crate) fn op_cond(interp: &mut Interpreter) -> Result<()> {
    // Tail position of the enclosing word, if any (set by the compiled-plan
    // tail op). Guards must run as non-tail (they may call the same word in a
    // non-tail position), so clear it here and hand it only to the winning
    // clause body, where a tail self-call becomes an internal backward jump.
    let tail_context: bool = std::mem::replace(&mut interp.in_tail_context, false);

    let clauses_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let clauses = match extract_clause_blocks(&clauses_val).and_then(split_clause_blocks) {
        Ok(c) => c,
        Err(e) => {
            interp.stack.push(clauses_val);
            return Err(e);
        }
    };

    run_cond_core(interp, &clauses, tail_context)
}

/// Bridge the clauses operand into the token streams `split_clause_blocks`
/// expects: one entry per clause element, each converted back to tokens via
/// the same `value_as_code.rs` bridge `EXEC`/`PROBE`/`DEF` use.
fn extract_clause_blocks(clauses_val: &Value) -> Result<Vec<Vec<Token>>> {
    let elements = clauses_val.as_vector_view().ok_or_else(|| {
        AjisaiError::from(
            "COND: expected a Vector of [ guard | body ] clauses as the second operand",
        )
    })?;
    elements
        .iter()
        .map(|clause| {
            let inner = clause.as_vector_view().ok_or_else(|| {
                AjisaiError::from("COND: each clause must itself be a [ guard | body ] block")
            })?;
            crate::interpreter::value_as_code::value_elements_to_tokens(&inner)
        })
        .collect()
}

/// Compiled entry point: the clauses were split once at compile time
/// (`lower_cond_dispatch`, `compiled_plan.rs`) from the single clauses-
/// wrapper literal lexically known to precede this `COND`. The wrapper's
/// `PushCodeBlock`/`PushVectorLiteral` op still pushes its Value at runtime,
/// for stack-discipline parity with the interpreted path (and so a compiled
/// and an interpreted run of the same source see the same stack traffic);
/// since the split is already known from compile time, that one value is
/// popped unread rather than re-derived from it.
pub(crate) fn op_cond_dispatch(interp: &mut Interpreter, precomputed: &[CondClause]) -> Result<()> {
    let tail_context: bool = std::mem::replace(&mut interp.in_tail_context, false);
    interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    interp.runtime_metrics.cond_dispatch_fast_count += 1;
    run_cond_core(interp, precomputed, tail_context)
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
            val
        }
        ConsumptionMode::Keep => interp
            .stack
            .last()
            .cloned()
            .ok_or(AjisaiError::StackUnderflow)?,
    };

    // Clauses are tried in the order they were written, and `IDLE` is tried in
    // that order too: reaching an `IDLE` clause means no clause above it fired,
    // which is precisely when `IDLE` is defined to fire. It used to be pulled
    // out of the sequence and deferred to the end, so a clause written *below*
    // an `IDLE` could win — the one place where reading COND top to bottom gave
    // the wrong answer. Writing `IDLE` anywhere but last is still poor style,
    // and now it also means exactly what it looks like.
    //
    // Hedged guard prefetch is part of the opt-in elastic engine; when it
    // handles the dispatch it returns from here so the greedy loop below is
    // exactly the code that runs in the default build.
    for clause in clauses {
        if is_idle_guard(&clause.guard) {
            return run_clause_body(interp, clause, &target_value, tail_context);
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
            "COND: mixed clause styles are not allowed; use either [guard][body] pairs or [guard | body] clauses consistently",
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
    // as independently managed vectors. The `Stack` now owns values and roles
    // together, so cloning it captures the aligned `(value, role)` slots
    // directly — no snapshot type and no alignment assertion needed.
    let saved_stack = interp.stack.clone();
    let saved_consumption_mode: ConsumptionMode = interp.consumption_mode;
    let saved_epoch: EpochSnapshot = interp.current_epoch_snapshot();

    interp.stack.clear();
    interp
        .stack
        .push_with_role(value.clone(), Interpretation::Unassigned);
    interp.consumption_mode = ConsumptionMode::Consume;
    // A clause is a block written in the enclosing frame, so it reads that
    // frame's names — the isolation COND enforces is of the stack, which is
    // what makes recursion terminate cleanly, and a name is not on the stack.
    interp.open_binding_scope(false);

    // Guards are never tail position; run the compiled sub-plan when available,
    // otherwise interpret the tokens. Both produce the same result value.
    let execution_result: Result<()> = if let Some(plan) = guard_plan {
        interp.runtime_metrics.cond_clause_compiled_count += 1;
        execute_compiled_plan(interp, plan)
    } else {
        interp.execute_section_core(guard_tokens, 0).map(|_| ())
    };
    interp.close_binding_scope();
    let guard_result_value: Option<Value> = interp.stack.pop();

    restore_cond_eval_state(interp, saved_stack, saved_consumption_mode, saved_epoch);

    execution_result?;

    let result_value: Value = guard_result_value.ok_or_else(|| {
        AjisaiError::from("COND: guard must return TRUE or FALSE, got empty stack")
    })?;
    // SPEC §7.4.3: a guard that reduces to the logical `Unknown` (U) — e.g.
    // an undecidable continued-fraction comparison — is not a definite
    // `true`, so its clause does not fire. Fall through to the next clause
    // exactly as for a `false` guard. U is neither an error nor a match.
    // A definite Boolean guard fires iff it is TRUE (SPEC §7.7). Accept a
    // bare Boolean or one wrapped in a single-element vector; fall back to the
    // legacy numeric-guard handling (0 = false, 1 = true) below otherwise.
    //
    // FINDING (not fixed here): both fallbacks are the truthiness coercion
    // LANG.VALUES.TRUTH rules out, and that LANG.VALUES.DISJOINT rules out
    // twice over — a singleton Vector is not its element, and a scalar is not
    // a Boolean. `AND`/`OR`/`NOT` and `extract_predicate_boolean` had the same
    // coercion removed; this is the last place it survives.
    //
    // Element lifting made it load-bearing rather than merely reachable. The
    // comparison Words used to project a singleton operand, so `[ 7 ] [ 5 ] >`
    // answered a bare `TRUE`; it now answers `[ TRUE ]`, which reaches the
    // wrapper case below. Tightening the guard therefore has to move the
    // `[ n ]`-wrapped-scalar idiom off comparisons across the examples and
    // tests, which is its own change rather than part of the lifting one.
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
    saved_stack: Stack,
    saved_consumption_mode: ConsumptionMode,
    saved_epoch: EpochSnapshot,
) {
    interp.stack = saved_stack;
    interp.consumption_mode = saved_consumption_mode;
    interp.dictionary_epoch = saved_epoch.dictionary_epoch;
    interp.execution_epoch = saved_epoch.execution_epoch;
    interp.global_epoch = saved_epoch.global_epoch;
}

fn execute_cond_body(
    interp: &mut Interpreter,
    body_tokens: &[Token],
    body_plan: Option<&CompiledPlan>,
    value: &Value,
    tail_context: bool,
) -> Result<()> {
    let saved_stack = interp.stack.clone();
    let saved_consumption_mode: ConsumptionMode = interp.consumption_mode;

    interp.stack.clear();
    interp
        .stack
        .push_with_role(value.clone(), Interpretation::Unassigned);
    interp.consumption_mode = ConsumptionMode::Consume;

    // This clause body runs in the word's tail position iff the COND itself
    // did. A tail self-call at the end of the body then defers to the
    // trampoline instead of recursing; its residual single value (the next
    // iteration's argument) flows out as this body's result below. The
    // deferral happens in `execute_section_core` (interpreted) or, when the
    // body is compiled, in `execute_compiled_line`'s tail-op handling — both
    // keyed on `in_tail_context` and `tail_self_word`.
    interp.in_tail_context = tail_context;
    interp.open_binding_scope(false);
    let execution_result: Result<()> = if let Some(plan) = body_plan {
        interp.runtime_metrics.cond_clause_compiled_count += 1;
        execute_compiled_plan(interp, plan)
    } else {
        interp.execute_section_core(body_tokens, 0).map(|_| ())
    };
    interp.close_binding_scope();
    interp.in_tail_context = false;
    let (body_result_value, body_result_hint): (Option<Value>, Interpretation) =
        match interp.stack.pop_slot() {
            Some((value, role)) => (Some(value), role),
            None => (None, Interpretation::Unassigned),
        };

    interp.stack = saved_stack;
    interp.consumption_mode = saved_consumption_mode;

    execution_result?;
    let result_value: Value =
        body_result_value.ok_or_else(|| AjisaiError::from("COND: body must return a value"))?;
    interp.stack.push_with_role(result_value, body_result_hint);
    Ok(())
}

fn is_idle_guard(guard_tokens: &[Token]) -> bool {
    if guard_tokens.len() != 1 {
        return false;
    }
    matches!(&guard_tokens[0], Token::Symbol(symbol) if symbol.as_ref().eq_ignore_ascii_case("IDLE"))
}
