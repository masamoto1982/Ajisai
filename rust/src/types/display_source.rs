//! Rendering a value as Ajisai source that rebuilds it.
//!
//! This is the structural half of the display: the part whose output is
//! source text. Its counterpart in `display.rs` is the role-dependent half —
//! a datetime, an interval, a continued fraction — which renders for reading
//! only and makes no round-trip claim.
//!
//! Splitting them apart is what keeps the round-trip rule statable in one
//! place: everything in this file must satisfy `tests/round_trip_laws.rs`,
//! and nothing in `display.rs` need do so.

use super::display::{format_exact_real, format_fraction, format_tensor_recursive};
use super::ValueData;

/// A rendered value: its source text.
pub(super) struct Rendered {
    pub(super) source: String,
}

impl Rendered {
    fn plain(source: String) -> Self {
        Rendered { source }
    }
}

/// Every value in the language has a literal, so a collection renders as one
/// whatever it holds: the delimiters and the elements, spaced.
fn render_delimited(open: &str, close: &str, elements: &[Rendered]) -> String {
    if elements.is_empty() {
        return format!("{open} {close}");
    }
    let joined = elements
        .iter()
        .map(|e| e.source.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    format!("{open} {joined} {close}")
}

fn render_vector_source(elements: &[Rendered]) -> String {
    render_delimited("[", "]", elements)
}

/// Render a value as Ajisai source that evaluates to it.
///
/// Every fragment is one literal, so every fragment nests inside another
/// without a phrase to compose (Ajisai offers no stack shuffling to compose
/// one with). `tests/round_trip_laws.rs` holds this to that promise by
/// executing what it writes.
///
/// Two domains do not round-trip and are not claimed to: a Symbol renders as
/// its bare name, which *calls* a Word rather than pushing the name, and a
/// role-dependent rendering (datetime, interval, continued fraction) comes
/// from `format_with_hint` above, not from here.
pub(super) fn render_value(data: &ValueData, depth: usize) -> Rendered {
    match data {
        ValueData::Nil => Rendered::plain("NIL".to_string()),
        // A String renders quoted at every depth, from its domain alone. This
        // is what replaces the old `Interpretation::Text` dispatch: the
        // Stack surface used to consult a role to decide whether a vector of
        // numbers was "really" text, and now there is nothing to decide.
        ValueData::Text(s) => Rendered::plain(format!("'{}'", s)),
        // The logical Unknown (U — `Nil` carrying the `TruthValue` hint)
        // has no dedicated variant, so it takes the `Nil` arm above and
        // renders as `NIL`, same as an operational NIL.
        // A definite boolean renders uniformly as TRUE/FALSE in every role
        // (LANG.VALUES.TRUTH), so the three-valued axis is observable
        // consistently whether the boolean came from a literal, a
        // comparison, or a logic word. Display-only and non-canonical.
        ValueData::Boolean(b) => Rendered::plain(if *b { "TRUE" } else { "FALSE" }.to_string()),
        ValueData::Scalar(f) => Rendered::plain(format_fraction(f)),
        ValueData::ExactScalar(er) => Rendered::plain(format_exact_real(er)),
        // A Record renders as its own literal (LANG.RECORDS.STRUCTURE):
        // each key beside the value under it, in the order the Record holds
        // them. The empty Record falls out as `{ }`, with no case of its own.
        ValueData::Record(record) => {
            let mut elements: Vec<Rendered> = Vec::with_capacity(record.len() * 2);
            for (key, value) in record.entries() {
                elements.push(render_value(&key.data, depth + 1));
                elements.push(render_value(&value.data, depth + 1));
            }
            Rendered {
                source: render_delimited("{", "}", &elements),
            }
        }
        ValueData::Vector(v) => {
            let elements: Vec<Rendered> = v
                .iter()
                // A nested element keeps its own role: a Text-role child
                // renders as a quoted string (`'AB'`), so strings stay
                // recognizable as strings inside a collection (SPEC
                // LANG.OBSERVATION.PROTOCOL). This falls out of `render_value`
                // dispatching on the String domain, with no role to consult.
                .map(|child| render_value(&child.data, depth + 1))
                .collect();
            Rendered {
                source: render_vector_source(&elements),
            }
        }
        ValueData::Tensor { data, shape } => {
            Rendered::plain(format_tensor_recursive(data, shape, depth))
        }
        // A Symbol renders as its own bare name — unquoted, unlike Text. Every
        // Vector-domain value renders with `[ ]`, the Record domain with
        // `{ }`: one pair per domain, no spelling carried over from how the
        // value was written.
        ValueData::Symbol(name) => Rendered::plain(name.to_string()),
    }
}
