use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::Value;

/// `EXEC` — evaluate a Vector's elements as instructions.
///
/// Every Vector is executable now (CodeBlock/Vector unification,
/// docs/dev/type-unification-work-order-2026-08.md): `[ 1 2 ADD ]` and
/// `{ 1 2 ADD }` are the same value, and `EXEC` runs either. The elements are
/// bridged back to tokens (`value_as_code.rs`) and run through the existing
/// token-based execution loop unchanged.
///
/// `as_vector_view` (not `as_vector`) matters here: a fully-numeric-
/// rectangular literal like `[ 1 2 ]` or `{ 1 2 }` silently promotes to
/// `ValueData::Tensor` (a storage optimization that predates this
/// unification), and `as_vector` alone excludes it.
///
/// `EXEC` used to render *any* value back to Ajisai source text and re-tokenize
/// it. That made a value's printed form decide its meaning, and the rendering
/// is not a right inverse of the reader: String is encoded as a Vector of
/// codepoints, which renders as those numbers, so `[ 1 2 ADD ] EXEC` pushed
/// `1 2 [ 65 68 68 ]` — the word `ADD` came back as its own codepoints instead
/// of being applied. Bridging the elements directly needs no such round-trip.
pub(crate) fn op_exec(interp: &mut Interpreter) -> Result<()> {
    let target: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let Some(elements) = target.as_vector_view() else {
        interp.stack.push(target);
        return Err(AjisaiError::create_structure_error(
            "code block",
            "non-code value",
        ));
    };
    let tokens = match crate::interpreter::value_as_code::value_elements_to_tokens(&elements) {
        Ok(t) => t,
        Err(e) => {
            interp.stack.push(target);
            return Err(e);
        }
    };
    crate::tokenizer::validate_code_tokens(&tokens).map_err(AjisaiError::from)?;
    interp.check_source_numeric_literals(&tokens)?;
    // The block `EXEC` runs is its own token stream and is never the enclosing
    // word's tail position — see `Interpreter::execute_nested_block`.
    interp.execute_nested_block(&tokens)
}
