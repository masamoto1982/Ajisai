use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::Interpreter;
use crate::interpreter::{ConsumptionMode, OperationTargetMode};
use crate::interpreter::AsyncAction;
use crate::types::{Token, Value};

/// Maximum iterations for `..` ROUTE (loop mode) to prevent infinite loops.
const ROUTE_MAX_ITERATIONS: usize = 10_000;

/// ROUTE — unified control structure for branching and looping.
///
/// Consumes code blocks from the stack top, pairs them as condition-action,
/// and routes the flow through the matching branch.
///
/// Modifier effects:
/// - `.` ROUTE (default): single branch selection (if-else / case)
/// - `..` ROUTE: loop while any condition matches (while / loop)
/// - `,,` ROUTE: bifurcation — keep original flow + result
/// - `.. ,,` ROUTE: loop + bifurcation
pub(crate) fn execute_route(interp: &mut Interpreter) -> Result<()> {
    let is_loop: bool = interp.operation_target_mode == OperationTargetMode::Stack;
    let keep_flow: bool = interp.consumption_mode == ConsumptionMode::Keep;

    // Reset modes before execution
    interp.operation_target_mode = OperationTargetMode::StackTop;
    interp.consumption_mode = ConsumptionMode::Consume;

    // 1. Pop all consecutive code blocks from stack top
    let mut code_blocks: Vec<Vec<Token>> = Vec::new();
    while let Some(top) = interp.stack.last() {
        if top.as_code_block().is_some() {
            let val: Value = interp.stack.pop().unwrap();
            code_blocks.push(val.as_code_block().unwrap().clone());
        } else {
            break;
        }
    }

    if code_blocks.is_empty() {
        return Err(AjisaiError::from(
            "ROUTE: expected at least one code block on the stack",
        ));
    }

    // Reverse to get original push order (first pushed = first condition)
    code_blocks.reverse();

    // 2. Determine default block (odd count → last block is default)
    let has_default: bool = code_blocks.len() % 2 == 1;
    let default_block: Option<Vec<Token>> = if has_default {
        Some(code_blocks.pop().unwrap())
    } else {
        None
    };

    // 3. Build condition-action pairs
    let pairs: Vec<(Vec<Token>, Vec<Token>)> = code_blocks
        .chunks(2)
        .map(|chunk: &[Vec<Token>]| (chunk[0].clone(), chunk[1].clone()))
        .collect::<Vec<_>>();

    // 4. Save the no-change-check flag (disable during route execution)
    let saved_no_change_check: bool = interp.disable_no_change_check;
    interp.disable_no_change_check = true;

    let result: Result<()> = if is_loop {
        execute_route_loop(interp, &pairs, &default_block, keep_flow)
    } else {
        execute_route_branch(interp, &pairs, &default_block, keep_flow)
    };

    interp.disable_no_change_check = saved_no_change_check;
    result
}

/// `.` ROUTE — single branch selection.
///
/// 1. Save the current stack as flow
/// 2. For each condition: restore stack, execute condition, check top
/// 3. First true condition → restore stack, execute action, done
/// 4. All false + default → restore stack, execute default
/// 5. All false + no default → restore stack (pass-through)
fn execute_route_branch(
    interp: &mut Interpreter,
    pairs: &[(Vec<Token>, Vec<Token>)],
    default_block: &Option<Vec<Token>>,
    keep_flow: bool,
) -> Result<()> {
    let saved_stack: Vec<Value> = interp.stack.clone();

    for (condition, action) in pairs {
        // Restore stack for condition evaluation
        interp.stack = saved_stack.clone();
        let (_, _): (usize, Option<AsyncAction>) = interp.execute_section_core(condition, 0)?;

        if interp.check_condition_on_stack()? {
            // Condition matched — restore stack and execute action
            interp.stack = saved_stack.clone();
            let (_, _): (usize, Option<AsyncAction>) = interp.execute_section_core(action, 0)?;

            if keep_flow {
                // ,, ROUTE: prepend original flow before result
                let result_stack: Vec<Value> = std::mem::take(&mut interp.stack);
                interp.stack = saved_stack;
                interp.stack.extend(result_stack);
            }
            return Ok(());
        }
    }

    // No condition matched
    if let Some(default) = default_block {
        interp.stack = saved_stack.clone();
        let (_, _): (usize, Option<AsyncAction>) = interp.execute_section_core(default, 0)?;

        if keep_flow {
            let result_stack: Vec<Value> = std::mem::take(&mut interp.stack);
            interp.stack = saved_stack;
            interp.stack.extend(result_stack);
        }
    } else {
        // Pass-through: restore original stack
        interp.stack = saved_stack;
    }

    Ok(())
}

/// `..` ROUTE — loop while any condition matches.
///
/// 1. Save the current stack as flow
/// 2. Loop:
///    a. For each condition: reset to current flow, execute condition, check
///    b. If true: reset to current flow, execute action, result becomes new flow
///    c. Go to step 2
/// 3. All false + default → execute default on current flow, done
/// 4. All false + no default → current flow is result
/// 5. Safety valve: max 10,000 iterations
fn execute_route_loop(
    interp: &mut Interpreter,
    pairs: &[(Vec<Token>, Vec<Token>)],
    default_block: &Option<Vec<Token>>,
    keep_flow: bool,
) -> Result<()> {
    let original_stack: Vec<Value> = interp.stack.clone();
    let mut current_flow: Vec<Value> = interp.stack.clone();
    let mut iterations: usize = 0;

    loop {
        if iterations >= ROUTE_MAX_ITERATIONS {
            return Err(AjisaiError::from(
                "ROUTE: loop exceeded 10,000 iterations (safety limit)",
            ));
        }
        iterations += 1;

        let mut matched: bool = false;

        for (condition, action) in pairs {
            // Restore to current flow for condition evaluation
            interp.stack = current_flow.clone();
            let (_, _): (usize, Option<AsyncAction>) = interp.execute_section_core(condition, 0)?;

            if interp.check_condition_on_stack()? {
                // Condition matched — execute action on current flow
                interp.stack = current_flow.clone();
                let (_, _): (usize, Option<AsyncAction>) = interp.execute_section_core(action, 0)?;
                current_flow = interp.stack.clone();
                matched = true;
                break;
            }
        }

        if !matched {
            // No condition matched — execute default if present, then exit loop
            if let Some(default) = default_block {
                interp.stack = current_flow;
                let (_, _): (usize, Option<AsyncAction>) = interp.execute_section_core(default, 0)?;
            } else {
                interp.stack = current_flow;
            }
            break;
        }
    }

    if keep_flow {
        // ,, .. ROUTE: prepend original flow before result
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
