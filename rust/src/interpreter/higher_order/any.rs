use super::common::{
    execute_executable_code, extract_executable_code, extract_predicate_boolean, ExecutableCode,
};
use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::is_vector_value;
use crate::interpreter::Interpreter;
use crate::types::Stack;
use crate::types::Value;

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

    // VTU Phase III bulk fast path: ANY over a 1-D dense Tensor with
    // a fast unary predicate. Disabled in hedged modes.
     let mut saved_stack: Stack = Stack::new();
    std::mem::swap(&mut interp.stack, &mut saved_stack);
    let saved_no_change_check = interp.disable_no_change_check;
    interp.disable_no_change_check = true;

    let mut result = false;
    let mut error: Option<AjisaiError> = None;
    for i in 0..target_val.len() {
        let elem = target_val
            .child(i)
            .expect("ANY: child index in 0..len must be valid");
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
