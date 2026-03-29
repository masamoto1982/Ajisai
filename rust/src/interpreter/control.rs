use std::sync::Arc;

use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{
    extract_integer_from_value, is_string_value, value_as_string,
};
use crate::interpreter::Interpreter;
use crate::interpreter::OperationTargetMode;
use crate::interpreter::AsyncAction;
use crate::types::{Token, Value, WordDefinition};

pub(crate) fn execute_times(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::from(
            "TIMES: expected code and count, got insufficient stack depth",
        ));
    }

    let count_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let code_val: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let count: i64 = extract_integer_from_value(&count_val)?;

    let saved_no_change_check: bool = interp.disable_no_change_check;
    interp.disable_no_change_check = true;

    let execution_result: Result<()> = if let Some(tokens) = code_val.as_code_block() {
        let tokens: Vec<Token> = tokens.clone();
        execute_code_block_n_times(interp, &tokens, count)
    } else if is_string_value(&code_val) {
        let word_name: String = value_as_string(&code_val).ok_or_else(|| {
            AjisaiError::create_structure_error("code block (: ... ;) or word name", "other format")
        })?;
        let upper_word_name: String = word_name.to_uppercase();

        let Some(def): Option<Arc<WordDefinition>> = interp.resolve_word(&upper_word_name) else {
            interp.disable_no_change_check = saved_no_change_check;
            return Err(AjisaiError::UnknownWord(upper_word_name));
        };

        if def.is_builtin {
            interp.disable_no_change_check = saved_no_change_check;
            return Err(AjisaiError::from(
                "TIMES: expected custom word, got builtin word",
            ));
        }

        execute_word_n_times(interp, &upper_word_name, count)
    } else {
        interp.disable_no_change_check = saved_no_change_check;
        interp.stack.push(code_val);
        interp.stack.push(count_val);
        return Err(AjisaiError::from(
            "TIMES: expected code block (: ... ;) or word name, got other value",
        ));
    };

    interp.disable_no_change_check = saved_no_change_check;
    execution_result
}

fn execute_code_block_n_times(
    interp: &mut Interpreter,
    tokens: &[Token],
    count: i64,
) -> Result<()> {
    for _ in 0..count {
        let (_, _): (usize, Option<AsyncAction>) = interp.execute_section_core(tokens, 0)?;
    }
    Ok(())
}

fn execute_word_n_times(
    interp: &mut Interpreter,
    word_name: &str,
    count: i64,
) -> Result<()> {
    for _ in 0..count {
        interp.execute_word_core(word_name)?;
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
