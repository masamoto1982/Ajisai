use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_word_name_from_value;
use crate::interpreter::Interpreter;
use crate::types::Value;

pub(crate) enum ExecutableCode {
    WordName(String),
    CodeBlock(Vec<crate::types::Token>),
}

pub(crate) fn extract_executable_code(
    _interp: &mut Interpreter,
    val: &Value,
) -> Result<ExecutableCode> {
    // Every Vector is a code operand candidate now (CodeBlock/Vector
    // unification) — bridged back to tokens (`value_as_code.rs`).
    // `as_vector_view` (Tensor-aware) — see control.rs's EXEC for why.
    // Which of the higher-order word's two operands *is* the code one is a
    // separate question this function does not answer: callers decide by
    // stack position before reaching it (the top-of-stack operand), same as
    // before this unification.
    if let Some(elements) = val.as_vector_view() {
        let tokens = crate::interpreter::value_as_code::value_elements_to_tokens(&elements)?;
        return Ok(ExecutableCode::CodeBlock(tokens));
    }

    // A Word name is a String (`[ 1 2 3 ] 'DBL' MAP`). This tested for a
    // Vector because a String *was* one; the String domain is the test now.
    if val.is_text() {
        return extract_word_name_from_value(val).map(ExecutableCode::WordName);
    }

    Err(AjisaiError::from(
        "EXTRACT_EXECUTABLE_CODE: expected a Vector ([ ... ]) or word name, got other value",
    ))
}

/// The predicate result of a higher-order block, which LANG.VALUES.TRUTH
/// restricts to the two-valued Boolean domain.
///
/// The domains are disjoint (LANG.VALUES.DISJOINT), so nothing else is a truth
/// value: a scalar is not a Boolean even when it is non-zero, a singleton
/// Vector is not its element, and NIL is absence rather than falsity. Each of
/// those used to be accepted here, which gave `FILTER` a truthiness rule no
/// other Word shared — `[ 1 2 3 ] [ 1 ] FILTER` silently kept every element
/// instead of raising the nonconforming-type ERROR its contract registers.
/// A caller that wants a numeric condition writes the comparison it means,
/// e.g. `0 NEQ`.
pub(crate) fn extract_predicate_boolean(condition_result: Value) -> Result<bool> {
    if let Some(b) = condition_result.as_truth() {
        return Ok(b);
    }

    Err(AjisaiError::create_structure_error(
        "boolean result from predicate block",
        "non-boolean value",
    ))
}

pub(crate) fn execute_executable_code(
    interp: &mut Interpreter,
    exec: &ExecutableCode,
) -> Result<()> {
    match exec {
        ExecutableCode::CodeBlock(tokens) => {
            interp.bump_execution_epoch();
            interp.execute_nested_block(tokens)
        }
        ExecutableCode::WordName(word_name) => interp.execute_word_core(word_name),
    }
}
