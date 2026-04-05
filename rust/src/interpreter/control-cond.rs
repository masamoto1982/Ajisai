use crate::error::{AjisaiError, Result};
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
        ConsumptionMode::Keep => interp.stack.last().cloned().ok_or(AjisaiError::StackUnderflow)?,
    };

    let mut else_body: Option<Vec<Token>> = None;
    for (guard_tokens, body_tokens) in &pairs {
        if is_idle_guard(guard_tokens) {
            else_body = Some(body_tokens.clone());
            continue;
        }

        if evaluate_guard_with_value(interp, guard_tokens, &target_value)? {
            return execute_cond_body(interp, body_tokens, &target_value);
        }
    }

    if let Some(body_tokens) = else_body {
        return execute_cond_body(interp, &body_tokens, &target_value);
    }

    Err(AjisaiError::CondExhausted)
}

fn collect_cond_pairs_from_stack(interp: &mut Interpreter) -> Result<Vec<(Vec<Token>, Vec<Token>)>> {
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

    if collected_blocks.is_empty() || collected_blocks.len() % 2 != 0 {
        return Err(AjisaiError::from(format!(
            "COND: expected even number of code blocks (guard/body pairs), got {}",
            collected_blocks.len()
        )));
    }

    collected_blocks.reverse();
    let mut pairs: Vec<(Vec<Token>, Vec<Token>)> = Vec::new();
    let mut i: usize = 0;
    while i < collected_blocks.len() {
        let guard_tokens: Vec<Token> = collected_blocks[i].clone();
        let body_tokens: Vec<Token> = collected_blocks[i + 1].clone();
        pairs.push((guard_tokens, body_tokens));
        i += 2;
    }
    Ok(pairs)
}

fn evaluate_guard_with_value(interp: &mut Interpreter, guard_tokens: &[Token], value: &Value) -> Result<bool> {
    let saved_stack: Vec<Value> = std::mem::take(&mut interp.stack);
    let saved_hints: Vec<DisplayHint> = interp.semantic_registry.stack_hints.clone();
    let saved_target_mode: OperationTargetMode = interp.operation_target_mode;
    let saved_consumption_mode: ConsumptionMode = interp.consumption_mode;
    let saved_safe_mode: bool = interp.safe_mode;

    interp.stack.push(value.clone());
    interp.semantic_registry.push_hint(DisplayHint::Auto);
    interp.operation_target_mode = OperationTargetMode::StackTop;
    interp.consumption_mode = ConsumptionMode::Consume;
    interp.safe_mode = false;

    let execution_result: Result<usize> = interp.execute_section_core(guard_tokens, 0);
    let guard_result_value: Option<Value> = interp.stack.pop();

    interp.stack = saved_stack;
    interp.semantic_registry.stack_hints = saved_hints;
    interp.operation_target_mode = saved_target_mode;
    interp.consumption_mode = saved_consumption_mode;
    interp.safe_mode = saved_safe_mode;

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
    let scalar = unwrapped
        .as_scalar()
        .ok_or_else(|| AjisaiError::from("COND: guard must return TRUE or FALSE, got non-scalar"))?;
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
    interp.semantic_registry.push_hint(DisplayHint::Auto);
    Ok(())
}

fn is_idle_guard(guard_tokens: &[Token]) -> bool {
    if guard_tokens.len() != 1 {
        return false;
    }
    matches!(&guard_tokens[0], Token::Symbol(symbol) if symbol.as_ref().eq_ignore_ascii_case("IDLE"))
}
