use super::common::{execute_executable_code, extract_executable_code, ExecutableCode};
use super::hedged::execute_hedged_map_kernel;
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{extract_integer_from_value, is_vector_value};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::{DisplayHint, Token, Value};

pub fn op_map(interp: &mut Interpreter) -> Result<()> {
    let code_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let plain_tokens: Option<Vec<Token>> = code_val.as_code_block().map(|t| t.to_vec());

    let executable: ExecutableCode = match extract_executable_code(interp, &code_val) {
        Ok(exec) => exec,
        Err(e) => {
            interp.stack.push(code_val);
            return Err(e);
        }
    };

    if let ExecutableCode::WordName(ref word_name) = executable {
        if !interp.word_exists(word_name) {
            interp.stack.push(code_val);
            return Err(AjisaiError::UnknownWord(word_name.clone()));
        }
    }

    let is_keep_mode: bool = interp.consumption_mode == ConsumptionMode::Keep;

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let target_val: Value = if is_keep_mode {
                interp.stack.last().cloned().ok_or_else(|| {
                    interp.stack.push(code_val.clone());
                    AjisaiError::StackUnderflow
                })?
            } else {
                interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
            };

            if target_val.is_nil() {
                interp.stack.push(Value::nil());
                return Ok(());
            }

            if !is_vector_value(&target_val) {
                if !is_keep_mode {
                    interp.stack.push(target_val);
                }
                interp.stack.push(code_val);
                return Err(AjisaiError::create_structure_error(
                    "vector",
                    "other format",
                ));
            }

            let n_elements: usize = target_val.len();
            if n_elements == 0 {
                interp.stack.push(Value::nil());
                return Ok(());
            }

            let mut results: Vec<Value> = Vec::with_capacity(n_elements);
            let mut saved_stack: Vec<Value> = Vec::new();
            std::mem::swap(&mut interp.stack, &mut saved_stack);

            let saved_target: OperationTargetMode = interp.operation_target_mode;
            let saved_no_change_check: bool = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut error: Option<AjisaiError> = None;
            for i in 0..n_elements {
                let elem: Value = target_val.get_child(i).unwrap().clone();
                match &executable {
                    ExecutableCode::QuantizedBlock(qb) => match execute_hedged_map_kernel(
                        interp,
                        "MAP",
                        qb,
                        plain_tokens.as_deref(),
                        elem.clone(),
                    ) {
                        Ok(result_val) => {
                            results.push(result_val);
                            continue;
                        }
                        Err(e) => {
                            error = Some(e);
                            break;
                        }
                    },
                    _ => {
                        interp.stack.clear();
                        interp.stack.push(elem);
                        match execute_executable_code(interp, &executable) {
                            Ok(_) => match interp.stack.pop() {
                                Some(result_val) => {
                                    let result_hint: DisplayHint =
                                        interp.semantic_registry.pop_hint();
                                    if is_vector_value(&result_val)
                                        && result_val.len() == 1
                                        && result_hint != DisplayHint::String
                                    {
                                        results.push(result_val.get_child(0).unwrap().clone());
                                    } else {
                                        results.push(result_val);
                                    }
                                }
                                None => {
                                    error = Some(AjisaiError::from(
                                        "MAP: expected return value, got empty stack",
                                    ));
                                    break;
                                }
                            },
                            Err(e) => {
                                error = Some(e);
                                break;
                            }
                        }
                    }
                }
            }

            interp.operation_target_mode = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = saved_stack;

            if let Some(e) = error {
                if !is_keep_mode {
                    interp.stack.push(target_val);
                }
                interp.stack.push(code_val);
                return Err(e);
            }

            interp.stack.push(Value::from_vector(results));
        }
        OperationTargetMode::Stack => {
            let count_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count: usize = match extract_integer_from_value(&count_val) {
                Ok(v) => v as usize,
                Err(e) => {
                    interp.stack.push(count_val);
                    interp.stack.push(code_val);
                    return Err(e);
                }
            };

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                interp.stack.push(code_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let mut saved_stack: Vec<Value> = Vec::new();
            std::mem::swap(&mut interp.stack, &mut saved_stack);

            let saved_target: OperationTargetMode = interp.operation_target_mode;
            let saved_no_change_check: bool = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut results: Vec<Value> = Vec::with_capacity(targets.len());
            for item in &targets {
                interp.stack.clear();
                interp.stack.push(item.clone());
                match execute_executable_code(interp, &executable) {
                    Ok(_) => match interp.stack.pop() {
                        Some(result) => results.push(result),
                        None => {
                            interp.operation_target_mode = saved_target;
                            interp.disable_no_change_check = saved_no_change_check;
                            interp.stack = saved_stack;
                            interp.stack.extend(targets);
                            interp.stack.push(count_val);
                            interp.stack.push(code_val);
                            return Err(AjisaiError::from(
                                "MAP: expected return value, got empty stack",
                            ));
                        }
                    },
                    Err(e) => {
                        interp.operation_target_mode = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = saved_stack;
                        interp.stack.extend(targets);
                        interp.stack.push(count_val);
                        interp.stack.push(code_val);
                        return Err(e);
                    }
                }
            }

            interp.operation_target_mode = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = saved_stack;
            interp.stack.extend(results);
        }
    }
    Ok(())
}
