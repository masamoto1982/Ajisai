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
    // `KEEP` modifies the `EXEC` call, never the first Word inside the block
    // — the same boundary a User Word call draws (`execute_word_core`). The
    // block runs consuming, and what the call reached, the block included, is
    // put back beneath its results. Left alone, the modifier used to leak in:
    // `[ 3 ] [ 1 + ] KEEP EXEC` kept `+`'s literal and answered
    // `[ 3/1 ] 1/1 [ 4/1 ]`.
    let keep_call = interp.consumption_mode == crate::interpreter::ConsumptionMode::Keep;
    interp.consumption_mode = crate::interpreter::ConsumptionMode::Consume;
    let kept_operands: Option<Vec<(Value, crate::types::Interpretation)>> = keep_call.then(|| {
        interp
            .stack
            .iter_slots()
            .map(|(value, role)| (value.clone(), role))
            .collect()
    });
    let enclosing_watch = interp.stack.begin_depth_watch();
    let result = exec_block(interp);
    let operand_floor = interp.stack.end_depth_watch(enclosing_watch);
    if let (Some(operands), true) = (kept_operands, result.is_ok()) {
        interp.restore_kept_operands(operands, operand_floor);
    }
    result
}

fn exec_block(interp: &mut Interpreter) -> Result<()> {
    let target: Value = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let Some(elements) = target.as_vector_view() else {
        interp.stack.push(target);
        return Err(AjisaiError::declared(
            "notExecutable",
            "EXEC: expected a Vector ([ ... ]) as the code operand, got another value",
        ));
    };
    let tokens = match crate::interpreter::value_as_code::value_elements_to_tokens(&elements) {
        Ok(t) => t,
        Err(e) => {
            interp.stack.push(target);
            return Err(e);
        }
    };
    crate::tokenizer::validate_code_tokens(&tokens).map_err(AjisaiError::MalformedSource)?;
    interp.check_source_numeric_literals(&tokens)?;
    // The block `EXEC` runs is its own token stream and is never the enclosing
    // word's tail position — see `Interpreter::execute_nested_block`.
    interp.execute_nested_block(&tokens)
}
