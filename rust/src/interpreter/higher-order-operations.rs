use crate::elastic::{
    can_hedge_hof_kernel, validate_hedged_winner, ElasticMode, HedgedCandidateResult, HedgedPath,
};
use crate::error::{AjisaiError, Result};
use crate::interpreter::quantized_block::{quantize_code_block, QuantizedBlock};
use crate::interpreter::value_extraction_helpers::{
    extract_integer_from_value, extract_word_name_from_value, is_vector_value,
};
use crate::interpreter::{ConsumptionMode, Interpreter, OperationTargetMode};
use crate::types::{DisplayHint, Token, Value, ValueData};

pub(crate) enum ExecutableCode {
    WordName(String),
    CodeBlock(Vec<Token>),
    QuantizedBlock(std::sync::Arc<QuantizedBlock>),
}

pub(crate) fn extract_executable_code(
    interp: &mut Interpreter,
    val: &Value,
) -> Result<ExecutableCode> {
    if let Some(tokens) = val.as_code_block() {
        if let Some(qb) = quantize_code_block(tokens, interp) {
            return Ok(ExecutableCode::QuantizedBlock(std::sync::Arc::new(qb)));
        }
        return Ok(ExecutableCode::CodeBlock(tokens.clone()));
    }

    if matches!(&val.data, ValueData::Vector(_)) {
        return extract_word_name_from_value(val).map(ExecutableCode::WordName);
    }

    Err(AjisaiError::from(
        "EXTRACT_EXECUTABLE_CODE: expected code block (: ... ;) or word name, got other value",
    ))
}

fn is_truthy_boolean(val: &Value) -> bool {
    if let Some(f) = val.as_scalar() {
        return !f.is_zero();
    }
    false
}

pub(crate) fn extract_predicate_boolean(condition_result: Value) -> Result<bool> {
    if let Some(f) = condition_result.as_scalar() {
        return Ok(!f.is_zero());
    }

    if is_vector_value(&condition_result) {
        if condition_result.len() == 1 {
            return Ok(is_truthy_boolean(condition_result.get_child(0).unwrap()));
        }
        return Err(AjisaiError::create_structure_error(
            "boolean result from FILTER code",
            "other format",
        ));
    }

    Err(AjisaiError::create_structure_error(
        "boolean vector result from FILTER code",
        "other format",
    ))
}

pub(crate) fn execute_executable_code(
    interp: &mut Interpreter,
    exec: &ExecutableCode,
) -> Result<()> {
    match exec {
        ExecutableCode::CodeBlock(tokens) => {
            interp.bump_execution_epoch();
            interp.execute_section_core(tokens, 0)?;
            Ok(())
        }
        ExecutableCode::WordName(word_name) => interp.execute_word_core(word_name),
        ExecutableCode::QuantizedBlock(qb) => execute_quantized_block_stack_top(interp, qb),
    }
}

fn execute_quantized_block_stack_top(interp: &mut Interpreter, qb: &QuantizedBlock) -> Result<()> {
    interp.runtime_metrics.quantized_block_use_count += 1;
    #[cfg(feature = "trace-quant")]
    eprintln!("[trace-quant] execute quantized block");
    crate::interpreter::compiled_plan::execute_compiled_plan(interp, &qb.compiled_plan)
}

pub(crate) fn execute_quantized_map_kernel(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    elem: Value,
) -> Result<Value> {
    let saved = interp.stack.clone();
    interp.stack.clear();
    interp.stack.push(elem);
    let res = execute_quantized_block_stack_top(interp, qb).and_then(|_| {
        interp.stack.pop().ok_or(AjisaiError::from(
            "MAP: expected return value, got empty stack",
        ))
    });
    interp.stack = saved;
    res
}

fn execute_plain_map_kernel(
    interp: &mut Interpreter,
    exec: &ExecutableCode,
    elem: Value,
) -> Result<Value> {
    let saved = interp.stack.clone();
    interp.stack.clear();
    interp.stack.push(elem);
    let res = execute_executable_code(interp, exec).and_then(|_| {
        interp.stack.pop().ok_or(AjisaiError::from(
            "MAP: expected return value, got empty stack",
        ))
    });
    interp.stack = saved;
    res
}

/// Execute a predicate quantized block with a single element.
/// Saves/restores the outer stack, pushes `elem`, runs the block, and
/// interprets the top-of-stack result as a boolean.
pub(crate) fn execute_quantized_predicate_kernel(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    elem: Value,
) -> Result<bool> {
    let saved = interp.stack.clone();
    interp.stack.clear();
    interp.stack.push(elem);
    let res = execute_quantized_block_stack_top(interp, qb).and_then(|_| {
        interp
            .stack
            .pop()
            .ok_or_else(|| {
                AjisaiError::from(
                    "predicate: expected boolean value from quantized block, got empty stack",
                )
            })
            .and_then(extract_predicate_boolean)
    });
    interp.stack = saved;
    res
}

fn execute_plain_predicate_kernel(
    interp: &mut Interpreter,
    exec: &ExecutableCode,
    elem: Value,
) -> Result<bool> {
    let saved = interp.stack.clone();
    interp.stack.clear();
    interp.stack.push(elem);
    let res = execute_executable_code(interp, exec).and_then(|_| {
        interp
            .stack
            .pop()
            .ok_or_else(|| AjisaiError::from("predicate: expected boolean value, got empty stack"))
            .and_then(extract_predicate_boolean)
    });
    interp.stack = saved;
    res
}

/// Execute a fold quantized block: pushes `(acc, elem)`, runs the block,
/// and returns the new accumulator from top-of-stack.
pub(crate) fn execute_quantized_fold_kernel(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    acc: Value,
    elem: Value,
) -> Result<Value> {
    let saved = interp.stack.clone();
    interp.stack.clear();
    interp.stack.push(acc);
    interp.stack.push(elem);
    let res = execute_quantized_block_stack_top(interp, qb).and_then(|_| {
        interp.stack.pop().ok_or_else(|| {
            AjisaiError::from("FOLD: expected return value from quantized block, got empty stack")
        })
    });
    interp.stack = saved;
    res
}

fn execute_plain_fold_kernel(
    interp: &mut Interpreter,
    exec: &ExecutableCode,
    acc: Value,
    elem: Value,
) -> Result<Value> {
    let saved = interp.stack.clone();
    interp.stack.clear();
    interp.stack.push(acc);
    interp.stack.push(elem);
    let res = execute_executable_code(interp, exec).and_then(|_| {
        interp
            .stack
            .pop()
            .ok_or_else(|| AjisaiError::from("FOLD: expected return value, got empty stack"))
    });
    interp.stack = saved;
    res
}

fn hedged_mode(mode: ElasticMode) -> bool {
    matches!(mode, ElasticMode::HedgedSafe | ElasticMode::HedgedTrace)
}

fn trace_hedged(interp: &Interpreter, msg: &str) {
    if interp.elastic_mode() == ElasticMode::HedgedTrace {
        eprintln!("[hedged] {}", msg);
    }
}

pub(crate) fn execute_hedged_map_kernel(
    interp: &mut Interpreter,
    op_name: &str,
    qb: &QuantizedBlock,
    exec: &ExecutableCode,
    elem: Value,
) -> Result<Value> {
    if !hedged_mode(interp.elastic_mode()) || !can_hedge_hof_kernel(op_name) {
        return execute_quantized_map_kernel(interp, qb, elem);
    }
    interp.runtime_metrics.hedged_race_started_count += 1;
    interp.push_hedged_trace(format!("hof-race:start op={}", op_name));
    let epoch = interp.current_epoch_snapshot().global_epoch;
    let quantized = execute_quantized_map_kernel(interp, qb, elem.clone());
    let plain = execute_plain_map_kernel(interp, exec, elem);

    match (quantized, plain) {
        (Ok(q), Ok(p)) => {
            if q != p {
                interp.runtime_metrics.hedged_race_validation_reject_count += 1;
                interp.runtime_metrics.hedged_race_fallback_count += 1;
                interp
                    .push_hedged_trace(format!("hof-race:fallback op={} reason=mismatch", op_name));
                trace_hedged(
                    interp,
                    "map winner rejected: quantized/plain mismatch -> plain fallback",
                );
                return Ok(p);
            }
            let candidate = HedgedCandidateResult {
                path: HedgedPath::Quantized,
                stack: vec![q.clone()],
                epoch_at_spawn: epoch,
            };
            match validate_hedged_winner(
                &candidate,
                interp.current_epoch_snapshot().global_epoch,
                1,
            ) {
                Ok(_) => {
                    interp.runtime_metrics.hedged_race_winner_quantized_count += 1;
                    interp.runtime_metrics.hedged_race_cancel_count += 1;
                    interp.push_hedged_trace(format!(
                        "hof-race:winner op={} path=quantized",
                        op_name
                    ));
                    Ok(q)
                }
                Err(_) => {
                    interp.runtime_metrics.hedged_race_validation_reject_count += 1;
                    interp.runtime_metrics.hedged_race_fallback_count += 1;
                    interp.push_hedged_trace(format!(
                        "hof-race:fallback op={} reason=validation",
                        op_name
                    ));
                    Ok(p)
                }
            }
        }
        (Err(_), Ok(p)) => {
            interp.runtime_metrics.hedged_race_winner_plain_count += 1;
            interp.runtime_metrics.hedged_race_fallback_count += 1;
            interp.push_hedged_trace(format!("hof-race:winner op={} path=plain", op_name));
            Ok(p)
        }
        (Ok(_), Err(e_plain)) => {
            interp.runtime_metrics.hedged_race_validation_reject_count += 1;
            Err(e_plain)
        }
        (Err(eq), Err(_)) => Err(eq),
    }
}

pub(crate) fn execute_hedged_predicate_kernel(
    interp: &mut Interpreter,
    op_name: &str,
    qb: &QuantizedBlock,
    exec: &ExecutableCode,
    elem: Value,
) -> Result<bool> {
    if !hedged_mode(interp.elastic_mode()) || !can_hedge_hof_kernel(op_name) {
        return execute_quantized_predicate_kernel(interp, qb, elem);
    }
    interp.runtime_metrics.hedged_race_started_count += 1;
    interp.push_hedged_trace(format!("hof-race:start op={}", op_name));
    let quantized = execute_quantized_predicate_kernel(interp, qb, elem.clone());
    let plain = execute_plain_predicate_kernel(interp, exec, elem);
    match (quantized, plain) {
        (Ok(q), Ok(p)) => {
            if q != p {
                interp.runtime_metrics.hedged_race_validation_reject_count += 1;
                interp.runtime_metrics.hedged_race_fallback_count += 1;
                interp
                    .push_hedged_trace(format!("hof-race:fallback op={} reason=mismatch", op_name));
                return Ok(p);
            }
            interp.runtime_metrics.hedged_race_winner_quantized_count += 1;
            interp.runtime_metrics.hedged_race_cancel_count += 1;
            interp.push_hedged_trace(format!("hof-race:winner op={} path=quantized", op_name));
            Ok(q)
        }
        (Err(_), Ok(p)) => {
            interp.runtime_metrics.hedged_race_winner_plain_count += 1;
            interp.runtime_metrics.hedged_race_fallback_count += 1;
            interp.push_hedged_trace(format!("hof-race:winner op={} path=plain", op_name));
            Ok(p)
        }
        (Ok(_), Err(e_plain)) => {
            interp.runtime_metrics.hedged_race_validation_reject_count += 1;
            Err(e_plain)
        }
        (Err(eq), Err(_)) => Err(eq),
    }
}

pub(crate) fn execute_hedged_fold_kernel(
    interp: &mut Interpreter,
    op_name: &str,
    qb: &QuantizedBlock,
    exec: &ExecutableCode,
    acc: Value,
    elem: Value,
) -> Result<Value> {
    if !hedged_mode(interp.elastic_mode()) || !can_hedge_hof_kernel(op_name) {
        return execute_quantized_fold_kernel(interp, qb, acc, elem);
    }
    interp.runtime_metrics.hedged_race_started_count += 1;
    interp.push_hedged_trace(format!("hof-race:start op={}", op_name));
    let quantized = execute_quantized_fold_kernel(interp, qb, acc.clone(), elem.clone());
    let plain = execute_plain_fold_kernel(interp, exec, acc, elem);
    match (quantized, plain) {
        (Ok(q), Ok(p)) => {
            if q != p {
                interp.runtime_metrics.hedged_race_validation_reject_count += 1;
                interp.runtime_metrics.hedged_race_fallback_count += 1;
                interp
                    .push_hedged_trace(format!("hof-race:fallback op={} reason=mismatch", op_name));
                return Ok(p);
            }
            interp.runtime_metrics.hedged_race_winner_quantized_count += 1;
            interp.runtime_metrics.hedged_race_cancel_count += 1;
            interp.push_hedged_trace(format!("hof-race:winner op={} path=quantized", op_name));
            Ok(q)
        }
        (Err(_), Ok(p)) => {
            interp.runtime_metrics.hedged_race_winner_plain_count += 1;
            interp.runtime_metrics.hedged_race_fallback_count += 1;
            interp.push_hedged_trace(format!("hof-race:winner op={} path=plain", op_name));
            Ok(p)
        }
        (Ok(_), Err(e_plain)) => {
            interp.runtime_metrics.hedged_race_validation_reject_count += 1;
            Err(e_plain)
        }
        (Err(eq), Err(_)) => Err(eq),
    }
}

pub fn op_map(interp: &mut Interpreter) -> Result<()> {
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
                        &executable,
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

pub fn op_filter(interp: &mut Interpreter) -> Result<()> {
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
                            &executable,
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

                                let is_true: bool = if is_vector_value(&condition_result) {
                                    if condition_result.len() == 1 {
                                        is_truthy_boolean(condition_result.get_child(0).unwrap())
                                    } else {
                                        error = Some(AjisaiError::create_structure_error(
                                            "boolean result from FILTER code",
                                            "other format",
                                        ));
                                        break;
                                    }
                                } else {
                                    error = Some(AjisaiError::create_structure_error(
                                        "boolean vector result from FILTER code",
                                        "other format",
                                    ));
                                    break;
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
                interp.stack.clear();
                interp.stack.push(item.clone());
                match execute_executable_code(interp, &executable) {
                    Ok(_) => {
                        let condition_result: Value = match interp.stack.pop() {
                            Some(result) => result,
                            None => {
                                interp.operation_target_mode = saved_target;
                                interp.disable_no_change_check = saved_no_change_check;
                                interp.stack = saved_stack;
                                interp.stack.extend(targets);
                                interp.stack.push(count_val);
                                interp.stack.push(code_val);
                                return Err(AjisaiError::from(
                                    "FILTER: expected boolean value, got empty stack",
                                ));
                            }
                        };

                        if is_vector_value(&condition_result)
                            && condition_result.len() == 1
                            && is_truthy_boolean(condition_result.get_child(0).unwrap())
                        {
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

pub fn op_any(interp: &mut Interpreter) -> Result<()> {
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
            let target_val: Value = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(code_val.clone());
                AjisaiError::StackUnderflow
            })?;

            if target_val.is_nil() {
                interp.stack.push(Value::from_bool(false));
                return Ok(());
            }
            if !is_vector_value(&target_val) {
                interp.stack.push(target_val);
                interp.stack.push(code_val);
                return Err(AjisaiError::create_structure_error(
                    "vector",
                    "other format",
                ));
            }

            let mut saved_stack: Vec<Value> = Vec::new();
            std::mem::swap(&mut interp.stack, &mut saved_stack);
            let saved_target = interp.operation_target_mode;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut result = false;
            let mut error: Option<AjisaiError> = None;
            for i in 0..target_val.len() {
                let elem = target_val.get_child(i).unwrap().clone();
                match &executable {
                    ExecutableCode::QuantizedBlock(qb) => {
                        match execute_hedged_predicate_kernel(interp, "ANY", qb, &executable, elem)
                        {
                            Ok(is_true) => {
                                if is_true {
                                    result = true;
                                    break;
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
                        interp.stack.push(elem);
                        match execute_executable_code(interp, &executable) {
                            Ok(_) => {
                                let condition_result = match interp.stack.pop() {
                                    Some(v) => v,
                                    None => {
                                        error = Some(AjisaiError::from(
                                            "ANY: expected boolean value, got empty stack",
                                        ));
                                        break;
                                    }
                                };
                                match extract_predicate_boolean(condition_result) {
                                    Ok(is_true) => {
                                        if is_true {
                                            result = true;
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        error = Some(e);
                                        break;
                                    }
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
                interp.stack.push(target_val);
                interp.stack.push(code_val);
                return Err(e);
            }

            interp.stack.push(Value::from_bool(result));
            Ok(())
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
            let saved_target = interp.operation_target_mode;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut result = false;
            for item in &targets {
                interp.stack.clear();
                interp.stack.push(item.clone());
                match execute_executable_code(interp, &executable) {
                    Ok(_) => {
                        let condition_result = match interp.stack.pop() {
                            Some(v) => v,
                            None => {
                                interp.operation_target_mode = saved_target;
                                interp.disable_no_change_check = saved_no_change_check;
                                interp.stack = saved_stack;
                                interp.stack.extend(targets);
                                interp.stack.push(count_val);
                                interp.stack.push(code_val);
                                return Err(AjisaiError::from(
                                    "FILTER: expected boolean value, got empty stack",
                                ));
                            }
                        };
                        let is_true = match extract_predicate_boolean(condition_result) {
                            Ok(v) => v,
                            Err(e) => {
                                interp.operation_target_mode = saved_target;
                                interp.disable_no_change_check = saved_no_change_check;
                                interp.stack = saved_stack;
                                interp.stack.extend(targets);
                                interp.stack.push(count_val);
                                interp.stack.push(code_val);
                                return Err(e);
                            }
                        };
                        if is_true {
                            result = true;
                            break;
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
            interp.stack.push(Value::from_bool(result));
            Ok(())
        }
    }
}

pub fn op_all(interp: &mut Interpreter) -> Result<()> {
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
            let target_val = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(code_val.clone());
                AjisaiError::StackUnderflow
            })?;

            if target_val.is_nil() {
                interp.stack.push(Value::from_bool(true));
                return Ok(());
            }
            if !is_vector_value(&target_val) {
                interp.stack.push(target_val);
                interp.stack.push(code_val);
                return Err(AjisaiError::create_structure_error(
                    "vector",
                    "other format",
                ));
            }

            let mut saved_stack: Vec<Value> = Vec::new();
            std::mem::swap(&mut interp.stack, &mut saved_stack);
            let saved_target = interp.operation_target_mode;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut result = true;
            let mut error: Option<AjisaiError> = None;
            for i in 0..target_val.len() {
                let elem = target_val.get_child(i).unwrap().clone();
                match &executable {
                    ExecutableCode::QuantizedBlock(qb) => {
                        match execute_hedged_predicate_kernel(interp, "ALL", qb, &executable, elem)
                        {
                            Ok(is_true) => {
                                if !is_true {
                                    result = false;
                                    break;
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
                        interp.stack.push(elem);
                        match execute_executable_code(interp, &executable) {
                            Ok(_) => {
                                let condition_result = match interp.stack.pop() {
                                    Some(v) => v,
                                    None => {
                                        error = Some(AjisaiError::from(
                                            "ALL: expected boolean value, got empty stack",
                                        ));
                                        break;
                                    }
                                };
                                match extract_predicate_boolean(condition_result) {
                                    Ok(is_true) => {
                                        if !is_true {
                                            result = false;
                                            break;
                                        }
                                    }
                                    Err(e) => {
                                        error = Some(e);
                                        break;
                                    }
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
                interp.stack.push(target_val);
                interp.stack.push(code_val);
                return Err(e);
            }

            interp.stack.push(Value::from_bool(result));
            Ok(())
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
            let saved_target = interp.operation_target_mode;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut result = true;
            for item in &targets {
                interp.stack.clear();
                interp.stack.push(item.clone());
                match execute_executable_code(interp, &executable) {
                    Ok(_) => {
                        let condition_result = match interp.stack.pop() {
                            Some(v) => v,
                            None => {
                                interp.operation_target_mode = saved_target;
                                interp.disable_no_change_check = saved_no_change_check;
                                interp.stack = saved_stack;
                                interp.stack.extend(targets);
                                interp.stack.push(count_val);
                                interp.stack.push(code_val);
                                return Err(AjisaiError::from(
                                    "FILTER: expected boolean value, got empty stack",
                                ));
                            }
                        };
                        let is_true = match extract_predicate_boolean(condition_result) {
                            Ok(v) => v,
                            Err(e) => {
                                interp.operation_target_mode = saved_target;
                                interp.disable_no_change_check = saved_no_change_check;
                                interp.stack = saved_stack;
                                interp.stack.extend(targets);
                                interp.stack.push(count_val);
                                interp.stack.push(code_val);
                                return Err(e);
                            }
                        };
                        if !is_true {
                            result = false;
                            break;
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
            interp.stack.push(Value::from_bool(result));
            Ok(())
        }
    }
}

pub fn op_count(interp: &mut Interpreter) -> Result<()> {
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
            let target_val = interp.stack.pop().ok_or_else(|| {
                interp.stack.push(code_val.clone());
                AjisaiError::StackUnderflow
            })?;

            if target_val.is_nil() {
                interp.stack.push(Value::from_int(0));
                return Ok(());
            }
            if !is_vector_value(&target_val) {
                interp.stack.push(target_val);
                interp.stack.push(code_val);
                return Err(AjisaiError::create_structure_error(
                    "vector",
                    "other format",
                ));
            }

            let mut saved_stack: Vec<Value> = Vec::new();
            std::mem::swap(&mut interp.stack, &mut saved_stack);
            let saved_target = interp.operation_target_mode;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut count: i64 = 0;
            let mut error: Option<AjisaiError> = None;
            for i in 0..target_val.len() {
                let elem = target_val.get_child(i).unwrap().clone();
                match &executable {
                    ExecutableCode::QuantizedBlock(qb) => {
                        match execute_hedged_predicate_kernel(
                            interp,
                            "COUNT",
                            qb,
                            &executable,
                            elem,
                        ) {
                            Ok(is_true) => {
                                if is_true {
                                    count += 1;
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
                        interp.stack.push(elem);
                        match execute_executable_code(interp, &executable) {
                            Ok(_) => {
                                let condition_result = match interp.stack.pop() {
                                    Some(v) => v,
                                    None => {
                                        error = Some(AjisaiError::from(
                                            "COUNT: expected boolean value, got empty stack",
                                        ));
                                        break;
                                    }
                                };
                                match extract_predicate_boolean(condition_result) {
                                    Ok(is_true) => {
                                        if is_true {
                                            count += 1;
                                        }
                                    }
                                    Err(e) => {
                                        error = Some(e);
                                        break;
                                    }
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
                interp.stack.push(target_val);
                interp.stack.push(code_val);
                return Err(e);
            }

            interp.stack.push(Value::from_int(count));
            Ok(())
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
            let saved_target = interp.operation_target_mode;
            let saved_no_change_check = interp.disable_no_change_check;
            interp.operation_target_mode = OperationTargetMode::StackTop;
            interp.disable_no_change_check = true;

            let mut matched_count: i64 = 0;
            for item in &targets {
                interp.stack.clear();
                interp.stack.push(item.clone());
                match execute_executable_code(interp, &executable) {
                    Ok(_) => {
                        let condition_result = match interp.stack.pop() {
                            Some(v) => v,
                            None => {
                                interp.operation_target_mode = saved_target;
                                interp.disable_no_change_check = saved_no_change_check;
                                interp.stack = saved_stack;
                                interp.stack.extend(targets);
                                interp.stack.push(count_val);
                                interp.stack.push(code_val);
                                return Err(AjisaiError::from(
                                    "FILTER: expected boolean value, got empty stack",
                                ));
                            }
                        };
                        let is_true = match extract_predicate_boolean(condition_result) {
                            Ok(v) => v,
                            Err(e) => {
                                interp.operation_target_mode = saved_target;
                                interp.disable_no_change_check = saved_no_change_check;
                                interp.stack = saved_stack;
                                interp.stack.extend(targets);
                                interp.stack.push(count_val);
                                interp.stack.push(code_val);
                                return Err(e);
                            }
                        };
                        if is_true {
                            matched_count += 1;
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
            interp.stack.push(Value::from_int(matched_count));
            Ok(())
        }
    }
}
