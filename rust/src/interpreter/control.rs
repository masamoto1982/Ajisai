use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::Value;

/// `EXEC` — evaluate a CodeBlock.
///
/// CodeBlock is the only executable domain (LANG.SOURCE.CODE: "executable
/// source lives in a CodeBlock, and a Vector is data and is never
/// executable"), so anything else is a nonconforming type and raises ERROR
/// (LANG.FAILURE.ERROR).
///
/// `EXEC` used to render *any* value back to Ajisai source text and re-tokenize
/// it. That made a value's printed form decide its meaning, and the rendering
/// is not a right inverse of the reader: String is encoded as a Vector of
/// codepoints, which renders as those numbers, so `[ 1 2 ADD ] EXEC` pushed
/// `1 2 [ 65 68 68 ]` — the word `ADD` came back as its own codepoints instead
/// of being applied. A CodeBlock already holds its tokens, so evaluating one
/// needs no round-trip at all.
pub(crate) fn op_exec(interp: &mut Interpreter) -> Result<()> {
    let target: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let Some(tokens) = target.as_code_block() else {
        interp.stack.push(target);
        return Err(AjisaiError::create_structure_error(
            "code block",
            "non-code value",
        ));
    };

    let tokens = tokens.clone();
    interp.execute_section_core(&tokens, 0)?;
    Ok(())
}
