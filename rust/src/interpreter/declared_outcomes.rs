//! `ABSENT` and `FAIL`: the trichotomy, stated by the program (LANG.FAILURE.TRICHOTOMY).
//!
//! A Core Word's contract says which of the three outcomes each of its inputs
//! meets; until these two Words a user Word could say neither of the failing
//! two — it answered a bare literal NIL, or let some inner Word raise for it.
//! `ABSENT` is a reasoned absence whose reason the program states, recovered
//! like any other; `FAIL` is an ERROR the program raises, propagating like any
//! other. Neither evaluates anything and neither can catch anything.

use super::ordering_ops::{restore, take_operand};
use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::types::Value;

/// `ABSENT ( [ 'reason' ] -> [ NIL ] )`.
pub fn op_absent(interp: &mut Interpreter) -> Result<()> {
    let operand = take_operand(interp)?;
    let Some(text) = operand.as_text() else {
        let got = operand.domain_name();
        restore(interp, operand);
        return Err(AjisaiError::declared(
            "nonText",
            format!("expected a String reason, got {got}"),
        ));
    };
    let absence = Value::nil_user_declared(text);
    interp.stack.push(absence);
    Ok(())
}

/// `FAIL ( [ 'message' ] -> [ ] )`: raises `declaredFailure`. The operand is
/// put back first, as every Word's operands are on an ERROR.
pub fn op_fail(interp: &mut Interpreter) -> Result<()> {
    let operand = take_operand(interp)?;
    let Some(text) = operand.as_text() else {
        let got = operand.domain_name();
        restore(interp, operand);
        return Err(AjisaiError::declared(
            "nonText",
            format!("expected a String message, got {got}"),
        ));
    };
    let message = text.to_string();
    restore(interp, operand);
    Err(AjisaiError::declared("declaredFailure", message))
}
