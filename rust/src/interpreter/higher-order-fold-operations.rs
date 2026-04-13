use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{
    extract_integer_from_value, is_vector_value,
};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::Value;
use super::higher_order::{extract_executable_code, execute_executable_code, ExecutableCode};

pub fn op_fold(interp: &mut Interpreter) -> Result<()> {
    let code_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

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
            let init_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let target_val: Value = if is_keep_mode {
                interp.stack.last().cloned().ok_or_else(|| {
                    interp.stack.push(init_val.clone());
                    interp.stack.push(code_val.clone());
                    AjisaiError::StackUnderflow
                })?
            } else {
                interp.stack.pop().ok_or_else(|| {
                    interp.stack.push(init_val.clone());
                    interp.stack.push(code_val.clone());
                    AjisaiError::StackUnderflow
                })?
            };

            if target_val.is_nil() {
                interp.stack.push(init_val);
                return Ok(());
            }

            if !is_vector_value(&target_val) {
                if !is_keep_mode {
                    interp.stack.push(target_val);
                }
                interp.stack.push(init_val);
                interp.stack.push(code_val);
                return Err(AjisaiError::create_structure_error("vector", "other format"));
            }

            let n_elements: usize = target_val.len();
            if n_elements == 0 {
                interp.stack.push(init_val);
                return Ok(());
            }

            let mut accumulator: Value = init_val;
            let mut saved_stack: Vec<Value> = Vec::new();
            std::mem::swap(&mut interp.stack, &mut saved_stack);

            let saved_target: OperationTargetMode = interp.operation_target_mode;
            let saved_no_change_check: bool = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut error: Option<AjisaiError> = None;
            for i in 0..n_elements {
                let elem: Value = target_val.get_child(i).unwrap().clone();
                interp.stack.clear();
                interp.stack.push(accumulator.clone());
                interp.stack.push(elem);

                match execute_executable_code(interp, &executable) {
                    Ok(_) => match interp.stack.pop() {
                        Some(result) => {
                            accumulator = result;
                        }
                        None => {
                            error = Some(AjisaiError::from(
                                "FOLD: expected return value, got empty stack",
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

            interp.operation_target_mode = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = saved_stack;

            if let Some(e) = error {
                if !is_keep_mode {
                    interp.stack.push(target_val);
                }
                interp.stack.push(accumulator);
                interp.stack.push(code_val);
                return Err(e);
            }

            interp.stack.push(accumulator);
            Ok(())
        }
        OperationTargetMode::Stack => {
            let init_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count: usize = extract_integer_from_value(&count_val)? as usize;

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                interp.stack.push(init_val);
                interp.stack.push(code_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let mut saved_stack: Vec<Value> = Vec::new();
            std::mem::swap(&mut interp.stack, &mut saved_stack);

            let mut accumulator: Value = init_val;

            let saved_target: OperationTargetMode = interp.operation_target_mode;
            let saved_no_change_check: bool = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            for item in targets {
                interp.stack.clear();
                interp.stack.push(accumulator);
                interp.stack.push(item);

                match execute_executable_code(interp, &executable) {
                    Ok(_) => {
                        let result: Value = interp.stack.pop().ok_or_else(|| {
                            AjisaiError::from("FOLD: expected return value, got empty stack")
                        })?;
                        accumulator = result;
                    }
                    Err(e) => {
                        interp.operation_target_mode = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = saved_stack;
                        return Err(e);
                    }
                }
            }

            interp.operation_target_mode = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = saved_stack;
            interp.stack.push(accumulator);
            Ok(())
        }
    }
}

pub fn op_unfold(interp: &mut Interpreter) -> Result<()> {
    const MAX_ITERATIONS: usize = 10000;

    let code_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

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

    match interp.operation_target_mode {
        OperationTargetMode::StackTop => {
            let init_state: Value = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(code_val.clone());
                AjisaiError::StackUnderflow
            })?;

            let mut state: Value = init_state.clone();
            let mut results: Vec<Value> = Vec::new();

            let mut saved_stack: Vec<Value> = Vec::new();
            std::mem::swap(&mut interp.stack, &mut saved_stack);

            let saved_target: OperationTargetMode = interp.operation_target_mode;
            let saved_no_change_check: bool = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut iteration_count: usize = 0;
            loop {
                if iteration_count >= MAX_ITERATIONS {
                    interp.operation_target_mode = saved_target;
                    interp.disable_no_change_check = saved_no_change_check;
                    interp.stack = saved_stack;
                    interp.stack.push(init_state);
                    interp.stack.push(code_val);
                    return Err(AjisaiError::from(
                        "UNFOLD: expected termination, got 10000 iterations without NIL",
                    ));
                }
                iteration_count += 1;

                interp.stack.clear();
                interp.stack.push(state.clone());

                if let Err(e) = execute_executable_code(interp, &executable) {
                    interp.operation_target_mode = saved_target;
                    interp.disable_no_change_check = saved_no_change_check;
                    interp.stack = saved_stack;
                    interp.stack.push(init_state);
                    interp.stack.push(code_val);
                    return Err(e);
                }

                let result: Value = interp.stack.pop().ok_or_else(|| {
                    interp.operation_target_mode = saved_target;
                    interp.disable_no_change_check = saved_no_change_check;
                    AjisaiError::from("UNFOLD: expected return value, got empty stack")
                })?;
                let _input: Option<Value> = interp.stack.pop();

                let unwrapped: Value = result;

                if unwrapped.is_nil() {
                    break;
                }

                if is_vector_value(&unwrapped) && unwrapped.len() == 2 {
                    results.push(unwrapped.get_child(0).unwrap().clone());

                    let next_state: &Value = unwrapped.get_child(1).unwrap();
                    if next_state.is_nil() {
                        break;
                    }

                    state = Value::from_vector(vec![next_state.clone()]);
                    continue;
                }

                interp.operation_target_mode = saved_target;
                interp.disable_no_change_check = saved_no_change_check;
                interp.stack = saved_stack;
                interp.stack.push(init_state);
                interp.stack.push(code_val);
                return Err(AjisaiError::from(
                    "UNFOLD: expected [element, next_state] or NIL, got other format",
                ));
            }

            interp.operation_target_mode = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = saved_stack;
            if results.is_empty() {
                interp.stack.push(Value::nil());
            } else {
                interp.stack.push(Value::from_vector(results));
            }
            Ok(())
        }
        OperationTargetMode::Stack => {
            let init_state: Value = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(code_val.clone());
                AjisaiError::StackUnderflow
            })?;

            let mut state: Value = init_state.clone();
            let mut saved_stack: Vec<Value> = Vec::new();
            std::mem::swap(&mut interp.stack, &mut saved_stack);

            let saved_target: OperationTargetMode = interp.operation_target_mode;
            let saved_no_change_check: bool = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut results: Vec<Value> = Vec::new();
            let mut iteration_count: usize = 0;

            loop {
                if iteration_count >= MAX_ITERATIONS {
                    interp.operation_target_mode = saved_target;
                    interp.disable_no_change_check = saved_no_change_check;
                    interp.stack = saved_stack;
                    interp.stack.push(init_state);
                    interp.stack.push(code_val);
                    return Err(AjisaiError::from(
                        "UNFOLD: expected termination, got 10000 iterations without NIL",
                    ));
                }
                iteration_count += 1;

                interp.stack.clear();
                interp.stack.push(state.clone());

                match execute_executable_code(interp, &executable) {
                    Ok(_) => {
                        let result: Value = interp.stack.pop().ok_or_else(|| {
                            AjisaiError::from("UNFOLD: expected return value, got empty stack")
                        })?;
                        let _input: Option<Value> = interp.stack.pop();

                        let unwrapped: Value = result;

                        if unwrapped.is_nil() {
                            break;
                        }

                        if is_vector_value(&unwrapped) && unwrapped.len() == 2 {
                            results.push(unwrapped.get_child(0).unwrap().clone());

                            let next_state: &Value = unwrapped.get_child(1).unwrap();
                            if next_state.is_nil() {
                                break;
                            }

                            state = Value::from_vector(vec![next_state.clone()]);
                            continue;
                        }

                        interp.operation_target_mode = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = saved_stack;
                        interp.stack.push(init_state);
                        interp.stack.push(code_val);
                        return Err(AjisaiError::from(
                            "UNFOLD: expected [element, next_state] or NIL, got other format",
                        ));
                    }
                    Err(e) => {
                        interp.operation_target_mode = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = saved_stack;
                        interp.stack.push(init_state);
                        interp.stack.push(code_val);
                        return Err(e);
                    }
                }
            }

            interp.operation_target_mode = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = saved_stack;
            interp.stack.extend(results);
            Ok(())
        }
    }
}

pub fn op_scan(interp: &mut Interpreter) -> Result<()> {
    let code_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

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
            let init_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let target_val: Value = if is_keep_mode {
                interp.stack.last().cloned().ok_or_else(|| {
                    interp.stack.push(init_val.clone());
                    interp.stack.push(code_val.clone());
                    AjisaiError::StackUnderflow
                })?
            } else {
                interp.stack.pop().ok_or_else(|| {
                    interp.stack.push(init_val.clone());
                    interp.stack.push(code_val.clone());
                    AjisaiError::StackUnderflow
                })?
            };

            if target_val.is_nil() {
                interp.stack.push(Value::nil());
                return Ok(());
            }

            if !is_vector_value(&target_val) {
                if !is_keep_mode {
                    interp.stack.push(target_val);
                }
                interp.stack.push(init_val);
                interp.stack.push(code_val);
                return Err(AjisaiError::create_structure_error("vector", "other format"));
            }

            let mut accumulator: Value = init_val;
            let mut results: Vec<Value> = Vec::with_capacity(target_val.len());
            let mut saved_stack: Vec<Value> = Vec::new();
            std::mem::swap(&mut interp.stack, &mut saved_stack);

            let saved_target: OperationTargetMode = interp.operation_target_mode;
            let saved_no_change_check: bool = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut error: Option<AjisaiError> = None;
            for i in 0..target_val.len() {
                let elem: Value = target_val.get_child(i).unwrap().clone();
                interp.stack.clear();
                interp.stack.push(accumulator.clone());
                interp.stack.push(elem);

                match execute_executable_code(interp, &executable) {
                    Ok(_) => match interp.stack.pop() {
                        Some(result) => {
                            accumulator = result;
                            results.push(accumulator.clone());
                        }
                        None => {
                            error = Some(AjisaiError::from(
                                "SCAN: expected return value, got empty stack",
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

            interp.operation_target_mode = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = saved_stack;

            if let Some(e) = error {
                if !is_keep_mode {
                    interp.stack.push(target_val);
                }
                interp.stack.push(accumulator);
                interp.stack.push(code_val);
                return Err(e);
            }

            if results.is_empty() {
                interp.stack.push(Value::nil());
            } else {
                interp.stack.push(Value::from_vector(results));
            }
            Ok(())
        }
        OperationTargetMode::Stack => {
            let init_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
            let count: usize = extract_integer_from_value(&count_val)? as usize;

            if interp.stack.len() < count {
                interp.stack.push(count_val);
                interp.stack.push(init_val);
                interp.stack.push(code_val);
                return Err(AjisaiError::StackUnderflow);
            }

            let targets: Vec<Value> = interp.stack.drain(interp.stack.len() - count..).collect();
            let mut saved_stack: Vec<Value> = Vec::new();
            std::mem::swap(&mut interp.stack, &mut saved_stack);

            let mut accumulator: Value = init_val;
            let mut results: Vec<Value> = Vec::with_capacity(targets.len());

            let saved_target: OperationTargetMode = interp.operation_target_mode;
            let saved_no_change_check: bool = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            for item in targets {
                interp.stack.clear();
                interp.stack.push(accumulator);
                interp.stack.push(item);

                match execute_executable_code(interp, &executable) {
                    Ok(_) => {
                        let result: Value = interp.stack.pop().ok_or_else(|| {
                            AjisaiError::from("SCAN: expected return value, got empty stack")
                        })?;
                        accumulator = result;
                        results.push(accumulator.clone());
                    }
                    Err(e) => {
                        interp.operation_target_mode = saved_target;
                        interp.disable_no_change_check = saved_no_change_check;
                        interp.stack = saved_stack;
                        return Err(e);
                    }
                }
            }

            interp.operation_target_mode = saved_target;
            interp.disable_no_change_check = saved_no_change_check;
            interp.stack = saved_stack;
            if results.is_empty() {
                interp.stack.push(Value::nil());
            } else {
                interp.stack.push(Value::from_vector(results));
            }
            Ok(())
        }
    }
}
