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

/// A rendered value: its source text, and whether a Record occurs anywhere
/// inside it. The flag is carried rather than recomputed because it decides
/// how the *enclosing* Vector renders.
pub(super) struct Rendered {
    pub(super) source: String,
    has_record: bool,
}

impl Rendered {
    fn plain(source: String) -> Self {
        Rendered {
            source,
            has_record: false,
        }
    }
}

/// A Vector of ordinary values is the literal `[ 1 2 ]`. One holding a Record
/// cannot be: a bracket literal does not evaluate what is written inside it,
/// so `[ [ 'a' ] [ 1 ] RECORD ]` is a three-element Vector — two Vectors and
/// the name `RECORD`. That is the one shape whose literal reads back as a
/// *different* value rather than as none, so it renders as the `COLLECT`
/// phrase that does build it.
fn render_vector_source(elements: &[Rendered]) -> String {
    if elements.is_empty() {
        return "[ ]".to_string();
    }
    let joined = elements
        .iter()
        .map(|e| e.source.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    if elements.iter().any(|e| e.has_record) {
        format!("{} {} COLLECT", joined, elements.len())
    } else {
        format!("[ {} ]", joined)
    }
}

/// Render a value as Ajisai source that evaluates to it.
///
/// Every fragment leaves exactly one value on the stack, which is what lets
/// fragments nest: `<keys> <values> RECORD` composes two Vector fragments
/// with no stack shuffling (Ajisai offers none). `tests/round_trip_laws.rs`
/// holds this to that promise by executing what it writes.
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
        // A Record renders as the call that builds it. It has no literal,
        // `RECORD` being the only way one comes to exist
        // (LANG.RECORDS.STRUCTURE), so showing that call is the display
        // telling the truth — and it is source, which `{ key: value … }` was
        // not. The empty Record falls out as `[ ] [ ] RECORD`, two Vectors of
        // equal length, with no case of its own.
        //
        // The cost is legibility: the brace form put each key beside its
        // value and this makes a reader count positions. That was accepted
        // deliberately for the round trip, and it falls on a human, not an
        // agent — LANG.OBSERVATION.PROTOCOL hands a Record over as aligned
        // key and value sequences and LANG.OBSERVATION.FIREWALL forbids
        // reading semantics out of display text at all.
        ValueData::Record(record) => {
            let keys: Vec<Rendered> = record
                .entries()
                .map(|(key, _)| render_value(&key.data, depth + 1))
                .collect();
            let values: Vec<Rendered> = record
                .entries()
                .map(|(_, value)| render_value(&value.data, depth + 1))
                .collect();
            Rendered {
                source: format!(
                    "{} {} RECORD",
                    render_vector_source(&keys),
                    render_vector_source(&values)
                ),
                has_record: true,
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
            let has_record = elements.iter().any(|e| e.has_record);
            Rendered {
                source: render_vector_source(&elements),
                has_record,
            }
        }
        ValueData::Tensor { data, shape } => {
            Rendered::plain(format_tensor_recursive(data, shape, depth))
        }
        // A Symbol renders as its own bare name — unquoted, unlike Text — and
        // every Vector-domain value renders with `[ ]` uniformly now (the old
        // `{ }`-spelled, lexeme-preserving CodeBlock rendering is gone with
        // the CodeBlock domain).
        ValueData::Symbol(name) => Rendered::plain(name.to_string()),
    }
}
