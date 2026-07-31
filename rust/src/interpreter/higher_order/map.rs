use super::common::{execute_executable_code, extract_executable_code, ExecutableCode};
use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::value_extraction_helpers::is_vector_value;
use crate::interpreter::{ConsumptionMode, Interpreter};
use crate::types::Stack;
use crate::types::{Interpretation, Value};

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

    let target_val: Value = if is_keep_mode {
        interp.stack.last().cloned().ok_or_else(|| {
            interp.stack.push(code_val.clone());
            AjisaiError::StackUnderflow
        })?
    } else {
        interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?
    };

    if target_val.is_nil() {
        interp
            .stack
            .push(Value::nil_inheriting_absence_from(&target_val));
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
        interp
            .stack
            .push(Value::nil_with_reason(NilReason::EmptySequence));
        return Ok(());
    }

    let mut results: Vec<Value> = Vec::with_capacity(n_elements);
    let mut saved_stack: Stack = Stack::new();
    std::mem::swap(&mut interp.stack, &mut saved_stack);
    let saved_no_change_check: bool = interp.disable_no_change_check;
    interp.disable_no_change_check = true;

    let mut error: Option<AjisaiError> = None;
    for i in 0..n_elements {
        let elem: Value = target_val
            .child(i)
            .expect("MAP: child index in 0..len must be valid");
        interp.stack.clear();
        interp.stack.push(elem);
        match execute_executable_code(interp, &executable) {
            Ok(_) => match interp.stack.pop_slot() {
                Some((result_val, result_hint)) => {
                    let is_string_result = result_hint == Interpretation::Text
                        || result_val.hint == Interpretation::Text;
                    if is_vector_value(&result_val) && result_val.len() == 1 && !is_string_result {
                        results.push(result_val.child(0).expect("len==1 implies child(0) exists"));
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
    interp.disable_no_change_check = saved_no_change_check;
    interp.stack = saved_stack;

    if let Some(e) = error {
        if !is_keep_mode {
            interp.stack.push(target_val);
        }
        interp.stack.push(code_val);
        return Err(e);
    }

    interp.stack.push(Value::from_vector_promoted(results));

    Ok(())
}
