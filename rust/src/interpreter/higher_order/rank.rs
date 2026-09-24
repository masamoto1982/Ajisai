//! `RANK`: `MAP` at a stated depth (LANG.COLLECTIONS.HIGHER).
//!
//! `MAP` walks the outermost axis and nothing below it, so a block could not
//! reach an inner axis at all — the one gap in an otherwise complete vector
//! vocabulary, and the one APL closes with a rank operator. Here the depth is
//! an operand of one Word rather than a modifier on every Word, which is the
//! whole of the difference between this and a rank *axis*.

use super::common::{execute_executable_code, extract_executable_code, ExecutableCode};
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{extract_integer_from_value, is_vector_value};
use crate::interpreter::Interpreter;
use crate::types::{Stack, Value};

/// `RANK ( [ vec ] [ n ] [ body ] -> [ result ] )`: descend `n` levels into
/// the Vector, stopping early at a leaf, and evaluate the block once on each
/// value reached, in index order, rebuilding the structure above. Depth 1 is
/// `MAP`; depth 0 evaluates the block once on the whole Vector.
pub fn op_rank(interp: &mut Interpreter) -> Result<()> {
    let code_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let executable: ExecutableCode = match extract_executable_code(interp, &code_val) {
        Ok(exec) => exec,
        Err(e) => {
            interp.stack.push(code_val);
            return Err(e);
        }
    };

    let depth_val: Value = match interp.stack.pop() {
        Some(depth_val) => depth_val,
        None => {
            interp.stack.push(code_val);
            return Err(AjisaiError::StackUnderflow);
        }
    };
    let depth: usize = match extract_integer_from_value(&depth_val) {
        Ok(depth) if depth >= 0 => depth as usize,
        // `invalidCount`: RANK's own declared condition for a depth operand
        // that is not a non-negative integer, as TAKE's count.
        Ok(_) | Err(AjisaiError::StructureError { .. }) => {
            interp.stack.push(depth_val);
            interp.stack.push(code_val);
            return Err(AjisaiError::declared(
                "invalidCount",
                "RANK: expected a non-negative integer depth",
            ));
        }
        Err(e) => {
            interp.stack.push(depth_val);
            interp.stack.push(code_val);
            return Err(e);
        }
    };

    let target_val: Value = match interp.stack.pop() {
        Some(target) => target,
        None => {
            interp.stack.push(depth_val);
            interp.stack.push(code_val);
            return Err(AjisaiError::StackUnderflow);
        }
    };

    if target_val.is_nil() {
        interp
            .stack
            .push(Value::nil_inheriting_absence_from(&target_val));
        return Ok(());
    }
    if !is_vector_value(&target_val) {
        interp.stack.push(target_val);
        interp.stack.push(depth_val);
        interp.stack.push(code_val);
        return Err(AjisaiError::declared(
            "nonVector",
            "RANK: expected a Vector, got a non-vector value",
        ));
    }

    // The block runs on an isolated frame holding the value reached and
    // nothing else, exactly as MAP's does (LANG.SOURCE.FRAME).
    let mut saved_stack: Stack = Stack::new();
    std::mem::swap(&mut interp.stack, &mut saved_stack);
    let saved_no_change_check: bool = interp.disable_no_change_check;
    interp.disable_no_change_check = true;
    let outcome = apply_at_depth(interp, &target_val, depth, &executable);
    interp.disable_no_change_check = saved_no_change_check;
    interp.stack = saved_stack;

    match outcome {
        Ok(result) => {
            interp.stack.push(result);
            Ok(())
        }
        Err(e) => {
            interp.stack.push(target_val);
            interp.stack.push(depth_val);
            interp.stack.push(code_val);
            Err(e)
        }
    }
}

/// Descend `depth` levels, stopping at a leaf, and evaluate the block on
/// whatever is reached. Termination: `depth` falls by one per level and the
/// children of a materialized Vector are finite (`remainingElements` in
/// spec/termination.json).
fn apply_at_depth(
    interp: &mut Interpreter,
    value: &Value,
    depth: usize,
    executable: &ExecutableCode,
) -> Result<Value> {
    if depth > 0 {
        if let Some(children) = value.as_vector_view() {
            let mut results: Vec<Value> = Vec::with_capacity(children.len());
            for child in children.iter() {
                results.push(apply_at_depth(interp, child, depth - 1, executable)?);
            }
            return Ok(Value::from_vector_promoted(results));
        }
    }
    interp.stack.clear();
    interp.stack.push(value.clone());
    execute_executable_code(interp, executable)?;
    match interp.stack.pop_slot() {
        // The block's one result is the value at this position, whatever its
        // shape — no unwrapping, for the reason MAP gives.
        Some((result, _hint)) => Ok(result),
        None => Err(AjisaiError::declared(
            "blockContractViolation",
            "RANK: expected return value, got empty stack",
        )),
    }
}
