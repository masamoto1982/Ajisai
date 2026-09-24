//! Pure Value -> protocol mapping.
//!
//! This is the single source of truth for the machine-facing value wire
//! format. It is shared by two serializers that must stay byte-compatible:
//! the WASM boundary (`wasm_interpreter_bindings::wasm_value_conversion`,
//! which renders a `ProtocolNode` into a `JsValue` for the GUI) and the
//! native CLI (`cli::report`, which renders it into JSON for agents). It
//! carries no platform glue, so the entire decision surface is unit / MC/DC
//! / property tested natively (AQ-REQ-003, see `value_protocol_tests.rs`).

use crate::types::fraction::Fraction;
use crate::types::{DenseTensor, Value, ValueData};
use num_bigint::BigInt;
use num_traits::{One, Zero};

/// Pure, side-effect-free description of the protocol object consumers
/// receive for a stack value: its `type` and `value`, plus the value to
/// derive the `semantics` block from. Every field is a function of the value
/// alone (LANG.VALUES.DENOTATION).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProtocolNode {
    pub(crate) type_str: &'static str,
    pub(crate) value: ProtocolValue,
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
    /// A Record's two aligned sequences (wire type `record`).
    Record {
        keys: Vec<ProtocolNode>,
        values: Vec<ProtocolNode>,
    },
}

/// One canonical term of an algebraic value's multiquadratic normal form.
/// Strings keep arbitrary-precision integers lossless at every host boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProtocolExactTerm {
    pub(crate) numerator: String,
    pub(crate) denominator: String,
    pub(crate) radicand: String,
}

/// Return the exact wire representation when `value` is an algebraic scalar.
///
/// Both the WASM and native CLI serializers call this helper. Keeping the
/// extraction here prevents one host from exposing the normal form while
/// another silently returns only its rational approximation.
pub(crate) fn exact_terms(value: &Value) -> Option<Vec<ProtocolExactTerm>> {
    Some(
        algebraic_normal_form(value)?
            .into_iter()
            .map(|(coefficient, radicand)| ProtocolExactTerm {
                numerator: coefficient.numerator().to_string(),
                denominator: coefficient.denominator().to_string(),
                radicand: radicand.to_string(),
            })
            .collect(),
    )
}

/// The multiquadratic normal form Σ c_m √m behind an algebraic scalar, or
/// `None` for every other value.
///
/// Both `exact_terms` and `exact_display` derive from this one extraction, so
/// the wire never carries a short rendering of terms it did not also send, or
/// terms without the rendering — the two fields are one fact in two shapes.
fn algebraic_normal_form(value: &Value) -> Option<Vec<(Fraction, BigInt)>> {
    let ValueData::ExactScalar(crate::types::exact::ExactReal::Algebraic(algebraic)) = &value.data
    else {
        return None;
    };
    Some(algebraic.normal_form_terms())
}

/// The exact value written short: `sqrt(2)`, `3/2*sqrt(5)`, `1/1 + sqrt(2)`.
///
/// A consumer reading an algebraic result top-down meets two renderings of the
/// number before it meets the number, and neither is it. `stackDisplay` is the
/// LANG.VALUES.EXACT continued fraction *truncated at a display budget* — √2 runs to
/// `[ 1; 2, 2, … ]`, ~101 characters, ending in the truncation marker `…` —
/// and the node's own `value` is a
/// rational approximation, flagged `approximate`. Each is correct as what it
/// is and misleading as what it resembles: the first looks complete and is
/// not, the second looks exact and is not. Reading either as the value is the
/// mistake this field removes.
///
/// It is a **display**, and deliberately not called canonical: `exactTerms` is
/// the value, this is one way of writing it, and the pairing is what says so.
/// Read it; do not parse it. Term order is the normal form's own ascending
/// radicand order, so the string is deterministic and identical across hosts.
///
/// It renders the stored normal form faithfully, which means it inherits that
/// form's one surprise: `8 SQRT` stores as `{1/1, 8}` and `2 SQRT 2 SQRT +` as
/// `{2/1, 2}`, so two values Ajisai's `=` decides are equal — and they are —
/// can be written `sqrt(8)` and `2/1*sqrt(2)`. Reducing the display would only
/// move the discrepancy, by making the string disagree with the `exactTerms`
/// beside it. Comparison is what decides equality here; string equality is not.
pub(crate) fn exact_display(value: &Value) -> Option<String> {
    let terms = algebraic_normal_form(value)?;
    // An algebraic irrational always has at least one term (a term-free normal
    // form would have demoted to Tier 0). Writing the zero rather than an
    // empty string keeps the field readable if that invariant ever moves.
    if terms.is_empty() {
        return Some("0/1".to_string());
    }
    let mut rendered = String::new();
    for (index, (coefficient, radicand)) in terms.iter().enumerate() {
        let negative = !coefficient.is_positive() && !coefficient.is_zero();
        if index == 0 {
            if negative {
                rendered.push('-');
            }
        } else {
            rendered.push_str(if negative { " - " } else { " + " });
        }
        rendered.push_str(&term_display(coefficient, radicand));
    }
    Some(rendered)
}

/// One `c√m` term, sign already emitted by the caller.
///
/// The coefficient keeps Ajisai's own `numerator/denominator` rendering rather
/// than collapsing `2/1` to `2`: every other number the language displays is
/// written that way, and a display that quietly switches conventions for
/// algebraic values is a second thing to learn.
fn term_display(coefficient: &Fraction, radicand: &BigInt) -> String {
    let numerator = coefficient.numerator();
    let magnitude = format!(
        "{}/{}",
        if numerator < BigInt::zero() {
            -numerator
        } else {
            numerator
        },
        coefficient.denominator(),
    );
    // The monomial `1` keys the rational part of the normal form: there is no
    // radical to write, only the coefficient.
    if radicand.is_one() {
        return magnitude;
    }
    if magnitude == "1/1" {
        return format!("sqrt({radicand})");
    }
    format!("{magnitude}*sqrt({radicand})")
}

fn number_protocol_value(f: &Fraction) -> ProtocolValue {
    ProtocolValue::Number {
        numerator: f.numerator().to_string(),
        denominator: f.denominator().to_string(),
    }
}

/// Flatten a dense tensor into protocol leaves. A dense tensor holds numbers
/// only (Booleans never densify), so every present lane is a `number` and an
/// absent lane is a `nil` carrying its reason. Interior nodes of rank >= 2
/// carry no `semantics`.
fn tensor_to_protocol(data: &DenseTensor, offset: usize, shape: &[usize]) -> Vec<ProtocolNode> {
    if shape.is_empty() || shape.len() == 1 {
        let len = shape.first().copied().unwrap_or_else(|| data.len());
        (offset..offset + len)
            .map(|lane| {
                // `from_dense_lane` turns the denominator-0 absence sentinel a
                // lane stores into `ValueData::Nil` *carrying the reason
                // stored beside it*, so an absent lane is reported as `nil`
                // rather than as the unreadable number `0/0`, and says why it
                // is absent (LANG.VALUES.NIL).
                let leaf = Value::from_dense_lane(data, lane);
                let (type_str, value) = match &leaf.data {
                    ValueData::Scalar(f) => ("number", number_protocol_value(f)),
                    _ => ("nil", ProtocolValue::Null),
                };
                ProtocolNode {
                    type_str,
                    value,
                    semantics: Some(leaf),
                }
            })
            .collect()
    } else {
        let outer = shape[0];
        let rest = &shape[1..];
        let stride: usize = rest.iter().product();
        (0..outer)
            .map(|i| ProtocolNode {
                type_str: "vector",
                value: ProtocolValue::Children(tensor_to_protocol(data, offset + i * stride, rest)),
                semantics: None,
            })
            .collect()
    }
}

/// The complete, pure Value -> protocol mapping. This is the single source of
/// truth for the value wire format (WASM and CLI) and the unit of native
/// verification for the serialization boundary. `type` is the value's domain
/// (LANG.VALUES.DISJOINT); nothing about how the value was produced reaches it.
pub(crate) fn value_to_protocol(value: &Value) -> ProtocolNode {
    let (type_str, protocol_value) = match &value.data {
        ValueData::Nil => ("nil", ProtocolValue::Null),
        ValueData::Boolean(b) => ("boolean", ProtocolValue::Bool(*b)),
        ValueData::ExactScalar(er) => {
            // Serialize ExactScalar as best rational approximation with large
            // denominator. The resulting node carries `semantics:
            // Some(value.clone())` (the original exact real) plus an
            // `approximate: true` marker in its semantics block (see the
            // serializers), so the approximation is observable and the
            // consumer can reference the exact source (LANG.OBSERVATION.FIREWALL).
            let approx = er
                .best_rational_approximation(&BigInt::from(1_000_000_000u64))
                .unwrap_or_else(Fraction::nil);
            ("number", number_protocol_value(&approx))
        }
        ValueData::Scalar(f) => ("number", number_protocol_value(f)),
        ValueData::Text(s) => ("string", ProtocolValue::Text(s.to_string())),
        ValueData::Vector(children) => (
            "vector",
            ProtocolValue::Children(children.iter().map(value_to_protocol).collect()),
        ),
        ValueData::Tensor { data, shape } => (
            "vector",
            ProtocolValue::Children(tensor_to_protocol(data, 0, shape)),
        ),
        // A Symbol is a value of its own domain (LANG.VALUES.DISJOINT); it
        // crosses this boundary as its own bare name, the only thing there
        // is to show.
        ValueData::Symbol(name) => ("symbol", ProtocolValue::Text(name.to_string())),
        // A Record crosses the boundary as its two observable sequences,
        // each an array of nodes, so a host reads keys and values without
        // guessing a key's type (LANG.RECORDS.STRUCTURE).
        ValueData::Record(record) => (
            "record",
            ProtocolValue::Record {
                keys: record.keys().iter().map(value_to_protocol).collect(),
                values: record.values().iter().map(value_to_protocol).collect(),
            },
        ),
    };
    ProtocolNode {
        type_str,
        value: protocol_value,
        semantics: Some(value.clone()),
    }
}
