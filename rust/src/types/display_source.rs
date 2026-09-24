//! Rendering a value as Ajisai source that rebuilds it.
//!
//! Everything in this file must satisfy `tests/round_trip_laws.rs`, except
//! what that file names out of scope: an irrational scalar, whose display is
//! truncated at a budget (LANG.VALUES.EXACT), a Symbol, and a Record, whose
//! `{ key value … }` display has no literal behind it.

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

/// A collection renders as its delimiters and its elements, spaced.
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
/// Every fragment is one unit, so fragments nest without a phrase to compose.
/// `tests/round_trip_laws.rs` holds the domains that have a literal to
/// rebuilding themselves by executing what this writes.
///
/// A Symbol does not round-trip and is not claimed to: it renders as its bare
/// name, which *calls* a Word rather than pushing the name. Nor does a
/// Record: it renders key beside value in `{ }`, which is a display only.
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
        // A Record renders each key beside the value under it, in the order
        // the Record holds them. The empty Record falls out as `{ }`, with no
        // case of its own.
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
        // A Symbol renders as its own bare name — unquoted, unlike Text.
        ValueData::Symbol(name) => Rendered::plain(name.to_string()),
    }
}
