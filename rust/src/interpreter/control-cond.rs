use crate::error::{AjisaiError, Result};
use crate::interpreter::epoch::EpochSnapshot;
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::{DisplayHint, Token, Value};

pub(crate) fn op_cond(interp: &mut Interpreter) -> Result<()> {
    let pairs: Vec<(Vec<Token>, Vec<Token>)> = collect_cond_pairs_from_stack(interp)?;
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

    let mut else_body: Option<Vec<Token>> = None;
    if is_hedged_cond_mode(interp) {
        interp.push_hedged_trace("cond:prefetch-start");
        if let Some(body) =
            evaluate_guard_hedged_prefetch(interp, &pairs, &target_value, &mut else_body)?
        {
            interp.push_hedged_trace("cond:winner-prefetched-guard");
            return execute_cond_body(interp, body, &target_value);
        }
    } else {
        for (guard_tokens, body_tokens) in &pairs {
            if is_idle_guard(guard_tokens) {
                else_body = Some(body_tokens.clone());
                continue;
            }

            if evaluate_guard_greedy(interp, guard_tokens, &target_value)? {
                return execute_cond_body(interp, body_tokens, &target_value);
            }
        }
    }

    if let Some(body_tokens) = else_body {
        return execute_cond_body(interp, &body_tokens, &target_value);
    }

    Err(AjisaiError::CondExhausted)
}

fn collect_cond_pairs_from_stack(
    interp: &mut Interpreter,
) -> Result<Vec<(Vec<Token>, Vec<Token>)>> {
    let mut collected_blocks: Vec<Vec<Token>> = Vec::new();

    while let Some(last_value) = interp.stack.last() {
        if let Some(tokens) = last_value.as_code_block() {
            collected_blocks.push(tokens.clone());
            interp.stack.pop();
            let _ = interp.semantic_registry.pop_hint();
            continue;
        }
        break;
    }

    collected_blocks.reverse();

    if collected_blocks.is_empty() {
        return Err(AjisaiError::from(
            "COND: expected guard/body clauses, got 0 code blocks",
        ));
    }

    let has_sep_flags: Vec<bool> = collected_blocks
        .iter()
        .map(|block| {
            block
                .iter()
                .any(|token| matches!(token, Token::CondClauseSep))
        })
        .collect();
    let all_with_sep: bool = has_sep_flags.iter().all(|has_sep| *has_sep);
    let none_with_sep: bool = has_sep_flags.iter().all(|has_sep| !*has_sep);

    if !all_with_sep && !none_with_sep {
        return Err(AjisaiError::from(
            "COND: mixed clause styles are not allowed; use either {guard}{body} pairs or {guard $ body} clauses consistently",
        ));
    }

    let mut pairs: Vec<(Vec<Token>, Vec<Token>)> = Vec::new();
    if all_with_sep {
        for block in &collected_blocks {
            let (guard_tokens, body_tokens) = split_cond_clause_block(block)?;
            pairs.push((guard_tokens, body_tokens));
        }
        return Ok(pairs);
    }

    if collected_blocks.len() % 2 != 0 {
        return Err(AjisaiError::from(format!(
            "COND: expected even number of code blocks (guard/body pairs), got {}",
            collected_blocks.len()
        )));
    }

    let mut i: usize = 0;
    while i < collected_blocks.len() {
        let guard_tokens: Vec<Token> = collected_blocks[i].clone();
        let body_tokens: Vec<Token> = collected_blocks[i + 1].clone();
        pairs.push((guard_tokens, body_tokens));
        i += 2;
    }

    Ok(pairs)
}

fn split_cond_clause_block(tokens: &[Token]) -> Result<(Vec<Token>, Vec<Token>)> {
    let separator_indexes: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter_map(|(i, token)| matches!(token, Token::CondClauseSep).then_some(i))
        .collect();

    if separator_indexes.len() != 1 {
        return Err(AjisaiError::from(
            "COND: a $ clause must contain exactly one '$' separator",
        ));
    }

    let separator_index: usize = separator_indexes[0];
    if separator_index == 0 || separator_index + 1 >= tokens.len() {
        return Err(AjisaiError::from(
            "COND: both guard and body are required around '$'",
        ));
    }

    let guard_tokens = tokens[..separator_index].to_vec();
    let body_tokens = tokens[(separator_index + 1)..].to_vec();
    Ok((guard_tokens, body_tokens))
}

fn evaluate_guard_isolated(
    interp: &mut Interpreter,
    guard_tokens: &[Token],
    value: &Value,
) -> Result<bool> {
    let saved_stack: Vec<Value> = std::mem::take(&mut interp.stack);
    let saved_hints: Vec<DisplayHint> = interp.semantic_registry.stack_hints.clone();
    let saved_target_mode: OperationTargetMode = interp.operation_target_mode;
    let saved_consumption_mode: ConsumptionMode = interp.consumption_mode;
    let saved_safe_mode: bool = interp.safe_mode;
    let saved_epoch: EpochSnapshot = interp.current_epoch_snapshot();

    interp.stack.push(value.clone());
    interp.semantic_registry.push_hint(DisplayHint::Auto);
    interp.operation_target_mode = OperationTargetMode::StackTop;
    interp.consumption_mode = ConsumptionMode::Consume;
    interp.safe_mode = false;

    let execution_result: Result<usize> = interp.execute_section_core(guard_tokens, 0);
    let guard_result_value: Option<Value> = interp.stack.pop();

    restore_cond_eval_state(
        interp,
        saved_stack,
        saved_hints,
        saved_target_mode,
        saved_consumption_mode,
        saved_safe_mode,
        saved_epoch,
    );

    execution_result?;

    let result_value: Value = guard_result_value.ok_or_else(|| {
        AjisaiError::from("COND: guard must return TRUE or FALSE, got empty stack")
    })?;
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
    value: &Value,
) -> Result<bool> {
    evaluate_guard_isolated(interp, guard_tokens, value)
}

fn restore_cond_eval_state(
    interp: &mut Interpreter,
    saved_stack: Vec<Value>,
    saved_hints: Vec<DisplayHint>,
    saved_target_mode: OperationTargetMode,
    saved_consumption_mode: ConsumptionMode,
    saved_safe_mode: bool,
    saved_epoch: EpochSnapshot,
) {
    interp.stack = saved_stack;
    interp.semantic_registry.stack_hints = saved_hints;
    interp.operation_target_mode = saved_target_mode;
    interp.consumption_mode = saved_consumption_mode;
    interp.safe_mode = saved_safe_mode;
    interp.dictionary_epoch = saved_epoch.dictionary_epoch;
    interp.module_epoch = saved_epoch.module_epoch;
    interp.execution_epoch = saved_epoch.execution_epoch;
    interp.global_epoch = saved_epoch.global_epoch;
}

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

fn is_hedged_cond_mode(interp: &Interpreter) -> bool {
    matches!(
        interp.elastic_mode(),
        crate::elastic::ElasticMode::HedgedSafe | crate::elastic::ElasticMode::HedgedTrace
    )
}

fn evaluate_guard_hedged_prefetch<'a>(
    interp: &mut Interpreter,
    pairs: &'a [(Vec<Token>, Vec<Token>)],
    target_value: &Value,
    else_body: &mut Option<Vec<Token>>,
) -> Result<Option<&'a [Token]>> {
    let mut prefetched: Vec<Option<Result<bool>>> = vec![None; pairs.len()];
    let mut has_impure_guard = false;

    for (idx, (guard_tokens, _)) in pairs.iter().enumerate() {
        if is_idle_guard(guard_tokens) {
            continue;
        }
        if is_pure_cond_guard(guard_tokens) {
            interp.runtime_metrics.cond_guard_prefetch_count += 1;
            prefetched[idx] = Some(evaluate_guard_isolated(interp, guard_tokens, target_value));
        } else {
            has_impure_guard = true;
        }
    }
    if has_impure_guard {
        interp.push_hedged_trace("cond:partial-prefetch-impure-guard-present");
    }

    for (idx, (guard_tokens, body_tokens)) in pairs.iter().enumerate() {
        if is_idle_guard(guard_tokens) {
            *else_body = Some(body_tokens.clone());
            continue;
        }

        let is_true = if let Some(result) = prefetched[idx].clone() {
            result?
        } else {
            evaluate_guard_greedy(interp, guard_tokens, target_value)?
        };
        if is_true {
            return Ok(Some(body_tokens.as_slice()));
        }
    }
    Ok(None)
}

fn execute_cond_body(interp: &mut Interpreter, body_tokens: &[Token], value: &Value) -> Result<()> {
    let saved_stack: Vec<Value> = std::mem::take(&mut interp.stack);
    let saved_hints: Vec<DisplayHint> = interp.semantic_registry.stack_hints.clone();
    let saved_target_mode: OperationTargetMode = interp.operation_target_mode;
    let saved_consumption_mode: ConsumptionMode = interp.consumption_mode;
    let saved_safe_mode: bool = interp.safe_mode;

    interp.stack.push(value.clone());
    interp.semantic_registry.stack_hints = vec![DisplayHint::Auto];
    interp.operation_target_mode = OperationTargetMode::StackTop;
    interp.consumption_mode = ConsumptionMode::Consume;
    interp.safe_mode = false;

    let execution_result: Result<usize> = interp.execute_section_core(body_tokens, 0);
    let body_result_hint: DisplayHint = interp.semantic_registry.pop_hint();
    let body_result_value: Option<Value> = interp.stack.pop();

    interp.stack = saved_stack;
    interp.semantic_registry.stack_hints = saved_hints;
    interp.operation_target_mode = saved_target_mode;
    interp.consumption_mode = saved_consumption_mode;
    interp.safe_mode = saved_safe_mode;

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
