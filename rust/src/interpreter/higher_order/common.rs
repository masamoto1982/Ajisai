use crate::error::{AjisaiError, Result};
use crate::interpreter::quantized_block::{quantize_code_block, QuantizedBlock};
use crate::interpreter::value_extraction_helpers::{extract_word_name_from_value, is_vector_value};
use crate::interpreter::Interpreter;
use crate::types::{Value, ValueData};

pub(crate) enum ExecutableCode {
    WordName(String),
    CodeBlock(Vec<crate::types::Token>),
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

pub(super) fn is_truthy_boolean(val: &Value) -> bool {
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

pub(super) fn execute_quantized_block_stack_top(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
) -> Result<()> {
    interp.runtime_metrics.quantized_block_use_count += 1;
    #[cfg(feature = "trace-quant")]
    eprintln!("[trace-quant] execute quantized block");
    crate::interpreter::compiled_plan::execute_compiled_plan(interp, &qb.compiled_plan)
}
