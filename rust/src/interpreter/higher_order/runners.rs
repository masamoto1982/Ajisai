use super::common::{
    execute_executable_code, execute_quantized_block_stack_top, extract_predicate_boolean,
    ExecutableCode,
};
use super::fast_kernels::{
    try_execute_fast_quantized_fold_kernel, try_execute_fast_quantized_map_kernel,
    try_execute_fast_quantized_predicate_kernel,
};
use crate::error::{AjisaiError, Result};
use crate::interpreter::quantized_block::QuantizedBlock;
use crate::interpreter::Interpreter;
use crate::types::Value;

pub(crate) fn execute_quantized_map_kernel(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    elem: Value,
) -> Result<Value> {
    if let Some(fast) = try_execute_fast_quantized_map_kernel(interp, qb, elem.clone()) {
        return fast;
    }

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

pub(super) fn execute_plain_map_kernel(
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

pub(crate) fn execute_quantized_predicate_kernel(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    elem: Value,
) -> Result<bool> {
    if let Some(fast) = try_execute_fast_quantized_predicate_kernel(interp, qb, elem.clone()) {
        return fast;
    }

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

pub(super) fn execute_plain_predicate_kernel(
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

pub(crate) fn execute_quantized_fold_kernel(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    acc: Value,
    elem: Value,
) -> Result<Value> {
    if let Some(fast) =
        try_execute_fast_quantized_fold_kernel(interp, qb, acc.clone(), elem.clone())
    {
        return fast;
    }

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

pub(super) fn execute_plain_fold_kernel(
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
