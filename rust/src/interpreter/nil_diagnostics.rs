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
//!     the logical Unknown (U) — which shares NIL storage but is a truth value,
//!     not an operational absence — is never reported as absent and its internal
//!     `LogicallyUnknown` reason is never leaked (SPEC §2.3 / §7.5 firewall).
//!
//! Applied to a value that is not an operational NIL, `NIL?` yields `FALSE` and
//! the other four yield a reasonless NIL — the "well-formed but cannot produce a
//! value" case of the Bubble Rule (SPEC §11.2), never an error.

use crate::error::{AjisaiError, Result};
use crate::interpreter::debug_diagnosis::DebugDiagnosis;
use crate::interpreter::Interpreter;
use crate::semantic::AbsenceMetadata;
use crate::types::{Interpretation, Value, ValueData};
use std::sync::Arc;

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

/// `NIL-ORIGIN` — the origin as a lowerCamelCase protocol-string Text. Origin is
/// a required field, so an operational NIL always yields Text; a non-operational
/// value yields NIL.
pub fn op_nil_origin(interp: &mut Interpreter) -> Result<()> {
    require_non_empty(interp)?;
    let protocol = peek_operational_absence(interp).map(|absence| absence.origin.as_protocol_str());
    push_protocol_string_or_nil(interp, protocol);
    Ok(())
}

/// `NIL-RECOVERABLE?` — the recoverability as a lowerCamelCase protocol-string
/// Text. Recoverability is a required, four-valued field, so it is returned as
/// Text (consistent with SPEC §4.5.0) rather than a two-valued boolean; a
/// non-operational value yields NIL.
pub fn op_nil_recoverable(interp: &mut Interpreter) -> Result<()> {
    require_non_empty(interp)?;
    let protocol =
        peek_operational_absence(interp).map(|absence| absence.recoverability.as_protocol_str());
    push_protocol_string_or_nil(interp, protocol);
    Ok(())
}

/// `NIL-DIAGNOSIS` — the three-layer debug diagnosis as a Record, or NIL when
/// there is no diagnosis or the value is not an operational NIL.
pub fn op_nil_diagnosis(interp: &mut Interpreter) -> Result<()> {
    require_non_empty(interp)?;
    // Build the owned Record while the metadata borrow is live, then release the
    // borrow before the mutable push.
    let record_value = peek_operational_absence(interp)
        .and_then(|absence| absence.diagnosis.as_ref())
        .map(diagnosis_record);
    match record_value {
        Some(value) => push_result(interp, value, Interpretation::Unassigned),
        None => push_result(interp, Value::nil(), Interpretation::Nil),
    }
    Ok(())
}

fn text(s: &str) -> Value {
    Value::from_string(s)
}

fn plain_vector(children: Vec<Value>) -> Value {
    Value {
        data: ValueData::Vector(Arc::new(children)),
        hint: Interpretation::Unassigned,
        absence: None,
    }
}

/// Build a Record value from ordered key/value fields, following the two-element
/// `[ key value ]` pair layout used elsewhere in the runtime.
fn record(fields: Vec<(&str, Value)>) -> Value {
    let mut pairs = Vec::with_capacity(fields.len());
    let mut keys = Vec::with_capacity(fields.len());
    for (key, value) in fields {
        keys.push(key);
        pairs.push(plain_vector(vec![text(key), value]));
    }
    Value {
        data: ValueData::Record {
            pairs: Arc::new(pairs),
            shape: crate::types::record_shape::record_shape_from_ordered_keys(keys),
        },
        hint: Interpretation::Unassigned,
        absence: None,
    }
}

/// Map a [`DebugDiagnosis`] onto a Record whose keys and string values are the
/// machine-readable protocol strings of SPEC §4.5.0 (never Rust `Debug` names).
/// `summary` and `evidence` are carried verbatim as the non-canonical
/// human-readable layer; `agreedPrefix` is included only when present.
fn diagnosis_record(diagnosis: &DebugDiagnosis) -> Value {
    let mut fields: Vec<(&str, Value)> = vec![
        ("when", text(diagnosis.when.as_protocol_str())),
        ("where", locus_record(&diagnosis.where_)),
        ("why", text(diagnosis.why.as_protocol_str())),
        ("summary", text(&diagnosis.summary)),
        (
            "evidence",
            plain_vector(diagnosis.evidence.iter().map(|e| text(e)).collect()),
        ),
        (
            "nextChecks",
            plain_vector(
                diagnosis
                    .next_checks
                    .iter()
                    .map(|check| {
                        record(vec![
                            ("label", text(&check.label)),
                            ("detail", text(&check.detail)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ];
    if let Some(prefix) = diagnosis.agreed_prefix {
        fields.push(("agreedPrefix", Value::from_int(prefix as i64)));
    }
    record(fields)
}

fn locus_record(locus: &crate::interpreter::debug_diagnosis::ErrorLocus) -> Value {
    let mut fields: Vec<(&str, Value)> = vec![("kind", text(locus.kind.as_protocol_str()))];
    if let Some(word) = &locus.word {
        fields.push(("word", text(word)));
    }
    if let Some(module) = &locus.module {
        fields.push(("module", text(module)));
    }
    if let Some(dictionary) = &locus.dictionary {
        fields.push(("dictionary", text(dictionary)));
    }
    record(fields)
}
