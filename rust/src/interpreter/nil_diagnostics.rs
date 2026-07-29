//! Diagnostic absence accessors (SPEC §4.5.0 / §7.15).
//!
//! The five words `NIL?`, `NIL-REASON`, `NIL-ORIGIN`, `NIL-RECOVERABLE?`, and
//! `NIL-DIAGNOSIS` let a program read the diagnostic metadata that a Bubble/NIL
//! carries (SPEC §11.2, `NilReason`) instead of collapsing every absence with a
//! single `VENT` fallback.
//!
//! Two invariants hold for all five (see the module-level notes in
//! `builtin_word_definitions.rs`):
//!
//!   * **Observation, not consumption.** Each word retains the inspected value
//!     on the stack and pushes its result above it, mirroring the LENGTH/GET
//!     inspection-word precedent of SPEC §7.1.1. A diagnosis is an observation.
//!   * **Operational NIL only.** They key off [`Value::is_operational_nil`], so
//!     the logical Unknown (U) — a distinct `ValueData::Unknown` variant, a
//!     truth value rather than an operational absence — is never reported as
//!     absent and never leaks a NIL reason (SPEC §2.3 / §7.5 firewall, now a
//!     type invariant).
//!
//! Applied to a value that is not an operational NIL, `NIL?` yields `FALSE` and
//! the other four yield a reasonless NIL — the "well-formed but cannot produce a
//! value" case of the Bubble Rule (SPEC §11.2), never an error.

use crate::error::{AjisaiError, Result};
use crate::interpreter::Interpreter;
use crate::semantic::AbsenceMetadata;
use crate::types::{Interpretation, Value};

/// Borrow the operational-NIL metadata of the top-of-stack value without
/// consuming it. Returns `None` when the stack is empty *(malformed use)*, or
/// when the top is not an operational NIL (a non-NIL value, or the logical U).
fn peek_operational_absence(interp: &Interpreter) -> Option<&AbsenceMetadata> {
    let top = interp.stack.last()?;
    if !top.is_operational_nil() {
        return None;
    }
    // Every operational NIL has metadata; `absence_metadata` is `Some` for a
    // reasoned bubble and the literal-NIL constructor. Fall back defensively.
    top.absence_metadata()
}

fn require_non_empty(interp: &Interpreter) -> Result<()> {
    if interp.stack.is_empty() {
        return Err(AjisaiError::StackUnderflow);
    }
    Ok(())
}

/// Push a result above the retained inspection target and register its semantic
/// interpretation so the value renders correctly (Text with quotes, a truth
/// value, a NIL, a Record). The target below keeps its own hint untouched.
fn push_result(interp: &mut Interpreter, value: Value, hint: Interpretation) {
    interp.stack.push_with_role(value, hint);
}

/// A protocol-string Text result, or NIL when the accessor found no value.
/// Carries the matching interpretation hint so a Text result renders as text
/// and a NIL result renders as NIL.
fn push_protocol_string_or_nil(interp: &mut Interpreter, value: Option<&'static str>) {
    match value {
        Some(protocol) => push_result(interp, Value::from_string(protocol), Interpretation::Text),
        None => push_result(interp, Value::nil(), Interpretation::Nil),
    }
}

/// `NIL?` — retain the value and push `TRUE` when it is an operational NIL,
/// `FALSE` otherwise. It checks absence only and never branches on the reason
/// (SPEC §4.5.0).
pub fn op_nil_check(interp: &mut Interpreter) -> Result<()> {
    let is_absent = match interp.stack.last() {
        Some(value) => value.is_operational_nil(),
        None => return Err(AjisaiError::StackUnderflow),
    };
    push_result(
        interp,
        Value::from_bool(is_absent),
        Interpretation::TruthValue,
    );
    Ok(())
}

/// `NIL-REASON` — the direct reason as a lowerCamelCase protocol-string Text,
/// or NIL when the value carries no reason or is not an operational NIL.
pub fn op_nil_reason(interp: &mut Interpreter) -> Result<()> {
    require_non_empty(interp)?;
    let protocol = peek_operational_absence(interp)
        .and_then(|absence| absence.reason.as_ref())
        .map(|reason| reason.as_protocol_str());
    push_protocol_string_or_nil(interp, protocol);
    Ok(())
}
