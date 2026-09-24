//! Rendering a value as Ajisai source that rebuilds it.
//!
//! Everything in this file must satisfy `tests/round_trip_laws.rs`. The one
//! exception is an irrational scalar, whose continued-fraction display is
//! truncated at a budget (LANG.VALUES.EXACT) and so cannot rebuild it.

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
/// A Symbol does not round-trip and is not claimed to: it renders as its bare
/// name, which *calls* a Word rather than pushing the name.
pub(super) fn render_value(data: &ValueData, depth: usize) -> Rendered {
    match data {
        ValueData::Nil => Rendered::plain("NIL".to_string()),
        // A String renders quoted at every depth, from its domain alone.
        ValueData::Text(s) => Rendered::plain(format!("'{}'", s)),
        // UNKNOWN is a NIL (LANG.VALUES.TRUTH), so it takes the `Nil` arm
        // above. A Boolean renders as TRUE/FALSE however it was produced.
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
                // A String child renders quoted (`'AB'`), so strings stay
                // recognizable inside a collection.
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
