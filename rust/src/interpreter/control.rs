use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::Interpreter;
use crate::interpreter::OperationTargetMode;
use crate::types::{Token, Value};

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

    interp.execute_section_core(&tokens, 0)?;

    Ok(())
}
