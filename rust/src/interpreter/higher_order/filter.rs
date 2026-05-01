use super::common::{
    execute_executable_code, extract_executable_code, extract_predicate_boolean, ExecutableCode,
};
use super::hedged::execute_hedged_predicate_kernel;
use super::runners::{execute_plain_predicate_kernel, execute_quantized_predicate_kernel};
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{extract_integer_from_value, is_vector_value};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::{Token, Value};

pub fn op_filter(interp: &mut Interpreter) -> Result<()> {
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
                    ExecutableCode::QuantizedBlock(qb) => {
                        match execute_hedged_predicate_kernel(
                            interp,
                            "FILTER",
                            qb,
                            plain_tokens.as_deref(),
                            elem.clone(),
                        ) {
                            Ok(is_true) => {
                                if is_true {
                                    results.push(elem);
                                }
                            }
                            Err(e) => {
                                error = Some(e);
                                break;
                            }
                        }
                    }
                    _ => {
                        interp.stack.clear();
                        interp.stack.push(elem.clone());
                        match execute_executable_code(interp, &executable) {
                            Ok(_) => {
                                let condition_result: Value = match interp.stack.pop() {
                                    Some(r) => r,
                                    None => {
                                        error = Some(AjisaiError::from(
                                            "FILTER: expected boolean value, got empty stack",
                                        ));
                                        break;
                                    }
                                };

                                let is_true: bool = match extract_predicate_boolean(condition_result)
                                {
                                    Ok(v) => v,
                                    Err(e) => {
                                        error = Some(e);
                                        break;
                                    }
                                };

                                if is_true {
                                    results.push(elem);
                                }
                            }
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

            if results.is_empty() {
                interp.stack.push(Value::nil());
            } else {
                interp.stack.push(Value::from_vector(results));
            }
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
                let pred_res = match &executable {
                    ExecutableCode::QuantizedBlock(qb) => {
                        execute_quantized_predicate_kernel(interp, qb, item.clone())
                    }
                    _ => execute_plain_predicate_kernel(interp, &executable, item.clone()),
                };
                match pred_res {
                    Ok(is_true) => {
                        if is_true {
                            results.push(item.clone());
                        }
                    }
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
