use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::AsyncAction;
use crate::interpreter::Interpreter;
use crate::interpreter::ConsumptionMode;
use crate::interpreter::OperationTargetMode;
use crate::types::{Token, Value};

const LOOP_MAX_ITERATIONS: usize = 10_000;

pub(crate) fn execute_branch_from_tokens(
    interp: &mut Interpreter,
    tokens: &[Token],
    start_index: usize,
) -> Result<usize> {
    let keep_flow: bool = interp.consumption_mode == ConsumptionMode::Keep;
    let saved_no_change_check: bool = interp.disable_no_change_check;
    interp.disable_no_change_check = true;

    let mut pairs: Vec<(Vec<Token>, Vec<Token>)> = Vec::new();
    let mut default_block: Option<Vec<Token>> = None;
    let mut i: usize = start_index;

    loop {
        if !matches!(tokens.get(i), Some(Token::BranchGuard)) {
            break;
        }

        let (first_block, after_first): (Vec<Token>, usize) = extract_block_after(tokens, i + 1, "$")?;

        if matches!(tokens.get(after_first), Some(Token::BlockStart)) {
            let (action_block, after_action): (Vec<Token>, usize) =
                extract_block_after(tokens, after_first, "$")?;
            pairs.push((first_block, action_block));
            i = after_action;
            continue;
        }

        default_block = Some(first_block);
        i = after_first;
        break;
    }

    let default: Vec<Token> = default_block.ok_or_else(|| {
        AjisaiError::from("$: expected final default block, got condition/action pairs only")
    })?;

    let result: Result<()> = execute_branch_with_pairs(interp, &pairs, &default, keep_flow);
    interp.disable_no_change_check = saved_no_change_check;
    result?;

    Ok(i)
}

pub(crate) fn execute_loop_from_tokens(
    interp: &mut Interpreter,
    tokens: &[Token],
    start_index: usize,
) -> Result<usize> {
    let keep_flow: bool = interp.consumption_mode == ConsumptionMode::Keep;
    let saved_no_change_check: bool = interp.disable_no_change_check;
    interp.disable_no_change_check = true;

    let (condition_block, after_condition): (Vec<Token>, usize) =
        extract_block_after(tokens, start_index + 1, "&")?;
    let (action_block, after_action): (Vec<Token>, usize) =
        extract_block_after(tokens, after_condition, "&")?;

    let result: Result<()> = execute_loop_with_blocks(interp, &condition_block, &action_block, keep_flow);
    interp.disable_no_change_check = saved_no_change_check;
    result?;

    Ok(after_action)
}

fn extract_block_after(tokens: &[Token], start_index: usize, word: &str) -> Result<(Vec<Token>, usize)> {
    if !matches!(tokens.get(start_index), Some(Token::BlockStart)) {
        return Err(AjisaiError::from(format!(
            "{}: expected code block after guard",
            word
        )));
    }

    let mut depth: i32 = 1;
    let mut i: usize = start_index + 1;
    let mut body: Vec<Token> = Vec::new();

    while i < tokens.len() && depth > 0 {
        match &tokens[i] {
            Token::BlockStart => {
                depth += 1;
                body.push(tokens[i].clone());
            }
            Token::BlockEnd => {
                depth -= 1;
                if depth > 0 {
                    body.push(tokens[i].clone());
                }
            }
            token => body.push(token.clone()),
        }
        i += 1;
    }

    if depth != 0 {
        return Err(AjisaiError::from(format!("{}: unclosed code block", word)));
    }

    Ok((body, i))
}

fn execute_branch_with_pairs(
    interp: &mut Interpreter,
    pairs: &[(Vec<Token>, Vec<Token>)],
    default_block: &[Token],
    keep_flow: bool,
) -> Result<()> {
    let saved_stack: Vec<Value> = interp.stack.clone();

    for (condition, action) in pairs {
        interp.stack = saved_stack.clone();
        let (_, _): (usize, Option<AsyncAction>) = interp.execute_section_core(condition, 0)?;

        if interp.check_condition_on_stack()? {
            interp.stack = saved_stack.clone();
            let (_, _): (usize, Option<AsyncAction>) = interp.execute_section_core(action, 0)?;

            if keep_flow {
                let result_stack: Vec<Value> = std::mem::take(&mut interp.stack);
                interp.stack = saved_stack;
                interp.stack.extend(result_stack);
            }
            return Ok(());
        }
    }

    interp.stack = saved_stack.clone();
    let (_, _): (usize, Option<AsyncAction>) = interp.execute_section_core(default_block, 0)?;

    if keep_flow {
        let result_stack: Vec<Value> = std::mem::take(&mut interp.stack);
        interp.stack = saved_stack;
        interp.stack.extend(result_stack);
    }

    Ok(())
}

fn execute_loop_with_blocks(
    interp: &mut Interpreter,
    condition_block: &[Token],
    action_block: &[Token],
    keep_flow: bool,
) -> Result<()> {
    let original_stack: Vec<Value> = interp.stack.clone();
    let mut current_flow: Vec<Value> = interp.stack.clone();
    let mut iterations: usize = 0;

    loop {
        if iterations >= LOOP_MAX_ITERATIONS {
            return Err(AjisaiError::from(
                "&: loop exceeded 10,000 iterations (safety limit)",
            ));
        }
        iterations += 1;

        interp.stack = current_flow.clone();
        let (_, _): (usize, Option<AsyncAction>) = interp.execute_section_core(condition_block, 0)?;

        if !interp.check_condition_on_stack()? {
            interp.stack = current_flow;
            break;
        }

        interp.stack = current_flow.clone();
        let (_, _): (usize, Option<AsyncAction>) = interp.execute_section_core(action_block, 0)?;
        current_flow = interp.stack.clone();
    }

    if keep_flow {
        let result_stack: Vec<Value> = std::mem::take(&mut interp.stack);
        interp.stack = original_stack;
        interp.stack.extend(result_stack);
    }

    Ok(())
}

pub(crate) fn op_exec(interp: &mut Interpreter) -> Result<()> {
    let target_vector: Value = match interp.operation_target_mode {
        OperationTargetMode::StackTop => interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?,
        OperationTargetMode::Stack => {
            let all_elements: Vec<Value> = interp.stack.drain(..).collect();
            Value::from_vector(all_elements)
        }
    };

    interp.operation_target_mode = OperationTargetMode::StackTop;

    crate::interpreter::vector_exec::execute_vector_as_code(interp, &target_vector)
}

pub(crate) fn op_eval(interp: &mut Interpreter) -> Result<()> {
    let source_code: String = match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            value_as_string(&val)
                .ok_or_else(|| AjisaiError::from("EVAL: expected string value, got non-string"))?
        }
        OperationTargetMode::Stack => {
            let all_elements: Vec<Value> = interp.stack.drain(..).collect();
            if all_elements.is_empty() {
                return Err(AjisaiError::from(
                    "EVAL: expected at least one character on stack, got empty stack",
                ));
            }
            let temp_vec: Value = Value::from_vector(all_elements);
            value_as_string(&temp_vec)
                .ok_or_else(|| AjisaiError::from("EVAL: expected convertible stack, got non-string data"))?
        }
    };

    interp.operation_target_mode = OperationTargetMode::StackTop;

    let tokens: Vec<Token> = crate::tokenizer::tokenize(&source_code)
        .map_err(|e| AjisaiError::from(format!("EVAL: expected valid syntax, got tokenization error: {}", e)))?;

    let (_, action): (usize, Option<AsyncAction>) = interp.execute_section_core(&tokens, 0)?;

    if action.is_some() {
        return Err(AjisaiError::from("EVAL: expected synchronous code, got async operation"));
    }

    Ok(())
}
