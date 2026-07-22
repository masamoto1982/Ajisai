//! Pure (Value, interpretation hint) -> protocol mapping.
//!
//! This is the single source of truth for the machine-facing value wire
//! format. It is shared by two serializers that must stay byte-compatible:
//! the WASM boundary (`wasm_interpreter_bindings::wasm_value_conversion`,
//! which renders a `ProtocolNode` into a `JsValue` for the GUI) and the
//! native CLI (`cli::report`, which renders it into JSON for agents). It
//! carries no platform glue, so the entire decision surface is unit / MC/DC
//! / property tested natively (AQ-REQ-003, see `value_protocol_tests.rs`).

use crate::types::fraction::Fraction;
use crate::types::{Interpretation, Value, ValueData};

/// Pure, side-effect-free description of the protocol object consumers
/// receive for a stack value: its `type`, `value`, and `displayHint`,
/// plus the value to derive the `semantics` block from.
/// Regression target: a promoted dense boolean tensor must serialize its
/// leaves as booleans, not numbers.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProtocolNode {
    pub(crate) type_str: &'static str,
    pub(crate) value: ProtocolValue,
    pub(crate) display_hint: Interpretation,
    /// Source value for the `semantics` block, or `None` for the interior
    /// nodes of a multi-dimensional tensor, which carry no `semantics`.
    pub(crate) semantics: Option<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ProtocolValue {
    Null,
    Bool(bool),
    Text(String),
    Number {
        numerator: String,
        denominator: String,
    },
    Children(Vec<ProtocolNode>),
    Handle(u64),
}

pub(crate) fn interpretation_protocol_str(hint: Interpretation) -> &'static str {
    match hint {
        Interpretation::Unassigned => "unassigned",
        Interpretation::RawNumber => "rawNumber",
        Interpretation::Interval => "interval",
        Interpretation::Text => "text",
        Interpretation::TruthValue => "truthValue",
        Interpretation::Timestamp => "timestamp",
        Interpretation::Nil => "nil",
        Interpretation::ContinuedFraction => "continuedFraction",
    }
}

fn number_protocol_value(f: &Fraction) -> ProtocolValue {
    ProtocolValue::Number {
        numerator: f.numerator().to_string(),
        denominator: f.denominator().to_string(),
    }
}

fn scalar_codepoint_string(f: &Fraction) -> String {
    f.to_i64()
        .and_then(|n| char::from_u32(n as u32))
        .map(|c| c.to_string())
        .unwrap_or_default()
}

/// Map a scalar fraction to its (type, value) under an interpretation role.
/// `datetime` keeps the numerator/denominator value shape, matching the
/// historical wire format.
fn scalar_to_protocol(f: &Fraction, effective: Interpretation) -> (&'static str, ProtocolValue) {
    match effective {
        Interpretation::TruthValue => ("boolean", ProtocolValue::Bool(!f.is_zero())),
        Interpretation::Timestamp => ("datetime", number_protocol_value(f)),
        Interpretation::Text => ("string", ProtocolValue::Text(scalar_codepoint_string(f))),
        _ => ("number", number_protocol_value(f)),
    }
}

/// Flatten a dense tensor into protocol leaves. Mirrors the Vector path:
/// only the `TruthValue` role propagates to leaves (booleans), all other
/// roles render numbers. Interior nodes of rank >= 2 carry no `semantics`.
fn tensor_to_protocol(
    data: &[Fraction],
    shape: &[usize],
    leaf_hint: Interpretation,
) -> Vec<ProtocolNode> {
    let leaves_are_bool = leaf_hint == Interpretation::TruthValue;
    if shape.is_empty() || shape.len() == 1 {
        data.iter()
            .map(|f| {
                let (type_str, value, hint) = if leaves_are_bool {
                    (
                        "boolean",
                        ProtocolValue::Bool(!f.is_zero()),
                        Interpretation::TruthValue,
                    )
                } else {
                    (
                        "number",
                        number_protocol_value(f),
                        Interpretation::RawNumber,
                    )
                };
                ProtocolNode {
                    type_str,
                    value,
                    display_hint: hint,
                    semantics: Some(Value::from_fraction(f.clone())),
                }
            })
            .collect()
    } else {
        let outer = shape[0];
        let rest = &shape[1..];
        let stride: usize = rest.iter().product();
        let interior_hint = if leaves_are_bool {
            Interpretation::TruthValue
        } else {
            Interpretation::Unassigned
        };
        (0..outer)
            .map(|i| ProtocolNode {
                type_str: "vector",
                value: ProtocolValue::Children(tensor_to_protocol(
                    &data[i * stride..(i + 1) * stride],
                    rest,
                    leaf_hint,
                )),
                display_hint: interior_hint,
                semantics: None,
            })
            .collect()
    }
}

fn vector_codepoint_text(children: &[Value]) -> String {
    children
        .iter()
        .filter_map(|child| match &child.data {
            ValueData::Scalar(codepoint) => {
                codepoint.to_i64().and_then(|n| char::from_u32(n as u32))
            }
            _ => None,
        })
        .collect()
}

/// The complete, pure (Value, external hint) -> protocol mapping. This is
/// the single source of truth for the value wire format (WASM and CLI) and
/// the unit of native verification for the serialization boundary.
pub(crate) fn value_to_protocol(
    value: &Value,
    external_hint_opt: Option<Interpretation>,
) -> ProtocolNode {
    let effective = external_hint_opt.unwrap_or(value.hint);
    // The ContinuedFraction role serializes numeric scalars as the canonical
    // nested-form string (SPEC §12.2), not a lossy rational approximation.
    if effective == Interpretation::ContinuedFraction
        && matches!(value.data, ValueData::Scalar(_) | ValueData::ExactScalar(_))
    {
        return ProtocolNode {
            type_str: "string",
            value: ProtocolValue::Text(crate::types::display::format_as_continued_fraction(value)),
            display_hint: effective,
            semantics: None,
        };
    }
    // The logical Unknown (U, SPEC §7.5) is observed through the
    // `truthValue` axis as `unknown`, never as a NIL. Detected via the
    // canonical `is_unknown()` predicate (SPEC §2.3 firewall: the internal
    // NIL representation is not observable).
    if value.is_unknown() {
        return ProtocolNode {
            type_str: "truthValue",
            value: ProtocolValue::Text("unknown".to_string()),
            display_hint: Interpretation::TruthValue,
            semantics: Some(value.clone()),
        };
    }
    let (type_str, protocol_value) = match &value.data {
        ValueData::Nil => ("nil", ProtocolValue::Null),
        // U is handled by the `is_unknown()` early return above, so this arm
        // is unreachable; it deliberately reports `truthValue`, never `nil`,
        // to uphold the firewall (SPEC §2.3) even if that guard ever moves.
        ValueData::Unknown(_) => ("truthValue", ProtocolValue::Text("unknown".to_string())),
        ValueData::Boolean(b) => ("boolean", ProtocolValue::Bool(*b)),
        ValueData::ExactScalar(er) => {
            // Serialize ExactScalar as best rational approximation with large
            // denominator. The resulting node carries `semantics:
            // Some(value.clone())` (the original exact real) plus an
            // `approximate: true` marker in its semantics block (see the
            // serializers), so the approximation is observable and the
            // consumer can reference the exact source (SPEC §2.3).
            use num_bigint::BigInt;
            let approx = er
                .best_rational_approximation(&BigInt::from(1_000_000_000u64))
                .unwrap_or_else(crate::types::fraction::Fraction::nil);
            scalar_to_protocol(&approx, effective)
        }
        ValueData::Scalar(f) => scalar_to_protocol(f, effective),
        ValueData::Vector(children) => {
            if effective == Interpretation::Text {
                (
                    "string",
                    ProtocolValue::Text(vector_codepoint_text(children)),
                )
            } else {
                let child_hint = if effective == Interpretation::TruthValue {
                    Some(Interpretation::TruthValue)
                } else {
                    None
                };
                let kids = children
                    .iter()
                    .map(|c| value_to_protocol(c, child_hint))
                    .collect();
                ("vector", ProtocolValue::Children(kids))
            }
        }
        ValueData::Tensor { data, shape } => {
            if effective == Interpretation::Text && shape.len() <= 1 {
                let text: String = data
                    .iter()
                    .filter_map(|f| f.to_i64().and_then(|n| char::from_u32(n as u32)))
                    .collect();
                ("string", ProtocolValue::Text(text))
            } else {
                let kids = tensor_to_protocol(&data.to_fractions(), shape, effective);
                ("vector", ProtocolValue::Children(kids))
            }
        }
        ValueData::Record { pairs, .. } => {
            let kids = pairs.iter().map(|p| value_to_protocol(p, None)).collect();
            ("vector", ProtocolValue::Children(kids))
        }
        ValueData::CodeBlock(_) => ("nil", ProtocolValue::Null),
        ValueData::ProcessHandle(id) => ("process_handle", ProtocolValue::Handle(*id)),
        ValueData::SupervisorHandle(id) => ("supervisor_handle", ProtocolValue::Handle(*id)),
    };
    ProtocolNode {
        type_str,
        value: protocol_value,
        display_hint: effective,
        semantics: Some(value.clone()),
    }
}
