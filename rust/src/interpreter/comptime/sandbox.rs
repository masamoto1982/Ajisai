use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::{Token, Value};

pub(crate) const PRECOMPUTE_STEP_LIMIT: usize = 20_000;

pub(crate) fn run_precompute_block(
    interp: &Interpreter,
    block_body_tokens: &[Token],
) -> Result<Vec<Value>> {
    let mut sandbox = Interpreter::new();
    sandbox.core_vocabulary = interp.core_vocabulary.clone();
    sandbox.user_words = interp.user_words.clone();
    sandbox.user_dictionaries = interp.user_dictionaries.clone();
    sandbox.module_vocabulary = interp.module_vocabulary.clone();
    sandbox.active_user_dictionary = interp.active_user_dictionary.clone();
    sandbox.max_execution_steps = PRECOMPUTE_STEP_LIMIT;
    sandbox
        .execute_section_core(block_body_tokens, 0)
        .map_err(|e| match e {
            AjisaiError::ExecutionLimitExceeded { .. } => {
                AjisaiError::from("PRECOMPUTE failed: execution exceeded step limit")
            }
            other => AjisaiError::from(format!("PRECOMPUTE failed: {}", other)),
        })?;

    Ok(sandbox.stack.into_values())
}
