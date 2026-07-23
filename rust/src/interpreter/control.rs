use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::Interpreter;
use crate::interpreter::OperationTargetMode;
use crate::types::{Token, Value, ValueData};

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

/// `OR-ELSE`: the value-based, block-taking NIL fallback (SPEC §6.4).
///
/// This is the explicit, value-based counterpart to the `^` / `VENT` control
/// directive. Where `^` inspects the token stream and skips *the next source
/// unit* unevaluated — making its meaning depend on lexical structure
/// (`1 ^ 2 3 ADD` skips only the `2`) — `OR-ELSE` operates purely on stack
/// values. The fallback is an ordinary `{ ... }` CodeBlock pushed before it, so
/// grouping is explicit and the behaviour is invariant under whitespace,
/// re-grouping, and refactoring.
///
/// ```text
/// [ candidate {fallback} ] -> [ candidate ]           (candidate is not NIL)
/// [ NIL       {fallback} ] -> [ fallback-result... ]  (candidate is NIL)
/// ```
///
/// The fallback block runs only when the candidate is a genuine NIL. The
/// logical UNKNOWN (U, SPEC §7.5) is *not* NIL, so — exactly like `^` — it
/// passes through unchanged and the fallback is left unrun.
pub(crate) fn op_or_else(interp: &mut Interpreter) -> Result<()> {
    // The fallback block sits on top (pushed last), the candidate below it.
    let block: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let fallback_tokens: Vec<Token> = match block.data {
        ValueData::CodeBlock(tokens) => tokens,
        _ => {
            return Err(AjisaiError::from(
                "OR-ELSE: expected a code block { ... } on top of the stack",
            ))
        }
    };

    let (candidate, role) = interp.stack.pop_slot().ok_or(AjisaiError::StackUnderflow)?;

    if !candidate.is_nil() {
        // Not NIL (numbers, text, vectors, U, …): keep the candidate with its
        // stack-position role and discard the fallback block unrun.
        interp.stack.push_with_role(candidate, role);
        return Ok(());
    }

    // NIL: discard it and evaluate the fallback block in the current context;
    // whatever it leaves on the stack replaces the NIL.
    interp.execute_section_core(&fallback_tokens, 0)?;
    Ok(())
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
            value_as_string(&temp_vec).ok_or_else(|| {
                AjisaiError::from("EVAL: expected convertible stack, got non-string data")
            })?
        }
    };

    interp.operation_target_mode = OperationTargetMode::StackTop;

    let tokens: Vec<Token> = crate::tokenizer::tokenize(&source_code).map_err(|e| {
        AjisaiError::from(format!(
            "EVAL: expected valid syntax, got tokenization error: {}",
            e
        ))
    })?;

    interp.execute_section_core(&tokens, 0)?;

    Ok(())
}
