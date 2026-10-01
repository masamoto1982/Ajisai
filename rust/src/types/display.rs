use super::exact::ExactReal;
use super::fraction::Fraction;
use super::{DenseTensor, Stack, Value, ValueData};
use num_bigint::BigInt;
use num_traits::Signed;
use std::fmt;

/// Render every stack slot as its display string (LANG.OBSERVATION.PROTOCOL).
///
/// This is the single stack rendering shared by all observation surfaces — the
/// CLI stack display, the REPL, the in-process conformance runner, and the JSON
/// report. Each slot renders from its value alone (LANG.VALUES.DENOTATION).
pub fn render_stack(stack: &Stack) -> Vec<String> {
    stack.iter().map(Value::to_string).collect()
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format_value_recursive(&self.data, 0))
    }
}

/// Render a value as Ajisai source that evaluates to it.
///
/// Everything this writes must satisfy `tests/round_trip_laws.rs`, except
/// what that file names out of scope: an irrational scalar, whose display is
/// truncated at a budget (LANG.VALUES.EXACT), a NIL carrying a reason, which
/// renders as the bare `NIL` that denotes only the literal one, and a Symbol
/// on its own.
///
/// A value with a literal renders as that literal, and a fragment with a
/// literal is one unit, so such fragments nest without a phrase to compose.
/// A Record has no literal — only `[ ]` delimits, and `{` `}` are ordinary
/// name characters — so it renders as the phrase that builds it,
/// `[ keys ] [ values ] RECORD`, and a Vector holding one renders as its
/// elements followed by `n COLLECT`, since inside `[ ]` that phrase would be
/// data. This is the same writing `value_as_code.rs` gives a definition body.
///
/// A Symbol on its own renders as its bare name, which *calls* a Word rather
/// than pushing the name, and is not claimed to round-trip; inside a
/// `COLLECT` phrase it is written as `[ NAME ] 0 GET`, which does.
fn format_value_recursive(data: &ValueData, depth: usize) -> String {
    match data {
        ValueData::Nil => "NIL".to_string(),
        // A String renders quoted at every depth, from its domain alone.
        ValueData::Text(s) => format!("'{}'", s),
        // UNKNOWN is a NIL (LANG.VALUES.TRUTH), so it takes the `Nil` arm
        // above. A Boolean renders as TRUE/FALSE however it was produced.
        ValueData::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        ValueData::Scalar(f) => format_fraction(f),
        ValueData::ExactScalar(er) => format_exact_real(er),
        // A Record renders as the phrase that builds it: its keys and its
        // values, each as a Vector, then `RECORD`. The empty Record falls out
        // as `[ ] [ ] RECORD`, with no case of its own.
        ValueData::Record(record) => {
            let keys: Vec<&Value> = record.entries().map(|(key, _)| key).collect();
            let values: Vec<&Value> = record.entries().map(|(_, value)| value).collect();
            format!(
                "{} {} RECORD",
                render_vector(&keys, depth + 1),
                render_vector(&values, depth + 1)
            )
        }
        ValueData::Vector(v) => {
            let children: Vec<&Value> = v.iter().collect();
            render_vector(&children, depth)
        }
        ValueData::Tensor { data, shape } => format_tensor_recursive(data, shape, depth),
        // A Symbol renders as its own bare name — unquoted, unlike Text.
        ValueData::Symbol(name) => name.to_string(),
    }
}

/// Whether a value holds a Record anywhere inside it, so that no bracket
/// literal denotes it and the Vector around it must be built by a phrase.
fn holds_record(data: &ValueData) -> bool {
    match data {
        ValueData::Record(_) => true,
        ValueData::Vector(children) => children.iter().any(|child| holds_record(&child.data)),
        _ => false,
    }
}

/// A Vector renders as its literal, `[ … ]`, when every element has a
/// literal; one that holds a Record renders as the phrase that builds it, its
/// elements followed by `n COLLECT`, because inside `[ ]` the Record's own
/// phrase would be read as data. A Symbol inside that phrase is written
/// `[ NAME ] 0 GET`, which reads the name out of a literal rather than
/// calling it.
fn render_vector(children: &[&Value], depth: usize) -> String {
    if children.iter().any(|child| holds_record(&child.data)) {
        let elements: Vec<String> = children
            .iter()
            .map(|child| match &child.data {
                ValueData::Symbol(name) => format!("[ {name} ] 0 GET"),
                _ => format_value_recursive(&child.data, depth + 1),
            })
            .collect();
        return format!("{} {} COLLECT", elements.join(" "), children.len());
    }
    let elements: Vec<String> = children
        .iter()
        // A String child renders quoted (`'AB'`), so strings stay
        // recognizable inside a collection.
        .map(|child| format_value_recursive(&child.data, depth + 1))
        .collect();
    render_delimited("[", "]", &elements)
}

/// A collection renders as its delimiters and its elements, spaced.
fn render_delimited(open: &str, close: &str, elements: &[String]) -> String {
    if elements.is_empty() {
        return format!("{open} {close}");
    }
    format!("{open} {} {close}", elements.join(" "))
}

fn format_tensor_recursive(data: &DenseTensor, shape: &[usize], _depth: usize) -> String {
    if shape.is_empty() {
        return "[ ]".to_string();
    }
    if shape.len() == 1 {
        if data.is_empty() {
            return "[ ]".to_string();
        }
        let inner: Vec<String> = data.iter().map(|f| format_fraction(&f)).collect();
        return format!("[ {} ]", inner.join(" "));
    }
    let outer = shape[0];
    let rest = &shape[1..];
    let stride: usize = rest.iter().product();
    if outer == 0 || stride == 0 {
        return "[ ]".to_string();
    }
    let flat = data.to_fractions();
    let inner: Vec<String> = (0..outer)
        .map(|i| {
            format_tensor_slice_recursive(&flat[i * stride..(i + 1) * stride], rest, _depth + 1)
        })
        .collect();
    format!("[ {} ]", inner.join(" "))
}

fn format_tensor_slice_recursive(data: &[Fraction], shape: &[usize], _depth: usize) -> String {
    if shape.is_empty() {
        return "[ ]".to_string();
    }
    if shape.len() == 1 {
        if data.is_empty() {
            return "[ ]".to_string();
        }
        let inner: Vec<String> = data.iter().map(format_fraction).collect();
        return format!("[ {} ]", inner.join(" "));
    }
    let outer = shape[0];
    let rest = &shape[1..];
    let stride: usize = rest.iter().product();
    if outer == 0 || stride == 0 {
        return "[ ]".to_string();
    }
    let inner: Vec<String> = (0..outer)
        .map(|i| {
            format_tensor_slice_recursive(&data[i * stride..(i + 1) * stride], rest, _depth + 1)
        })
        .collect();
    format!("[ {} ]", inner.join(" "))
}

/// Canonical numeric rendering: every number is shown as a reduced
/// `numerator/denominator`, integers included (`3` -> `3/1`). There is no
/// decimal surface form and no per-value style — the display is uniform
/// and matches the exact-real internal model.
fn format_fraction(f: &Fraction) -> String {
    if f.is_nil() {
        return "NIL".to_string();
    }
    format!("{}/{}", f.numerator(), f.denominator())
}

/// Display an `ExactReal`. A rational writes as `numerator/denominator`;
/// an algebraic irrational writes its normal form as one token —
/// `sqrt(2)`, `1/2*sqrt(2)`, `1/1+sqrt(2)`, `sqrt(2)-sqrt(3)`, rendering the
/// same terms the host protocol's `exactTerms` carries. It is a display, not
/// source: no literal denotes an irrational, and a Vector literal would read
/// `2 SQRT` as a number and a Symbol. Written without spaces so that inside a
/// Vector it still reads as one element. Nothing is truncated or
/// approximated: the normal form *is* the value, and its rendering is finite.
fn format_exact_real(er: &ExactReal) -> String {
    match er {
        ExactReal::Rational(f) => format_fraction(f),
        ExactReal::Algebraic(a) => render_algebraic_terms(&a.normal_form_terms()),
    }
}

/// The normal form `Σ cᵢ√mᵢ` as one token, terms in the normal form's own
/// ascending radicand order (the rational term, radicand 1, first). The
/// coefficient keeps Ajisai's own `numerator/denominator` rendering rather
/// than collapsing `2/1` to `2`: every other number the language displays is
/// written that way. A unit coefficient is left unwritten.
pub(crate) fn render_algebraic_terms(terms: &[(Fraction, BigInt)]) -> String {
    // An algebraic irrational always has at least one term (a term-free normal
    // form would have demoted to a rational). Writing the zero rather than an
    // empty string keeps the display readable if that invariant ever moves.
    if terms.is_empty() {
        return "0/1".to_string();
    }
    let mut out = String::new();
    for (index, (coefficient, radicand)) in terms.iter().enumerate() {
        let negative = !coefficient.is_positive() && !coefficient.is_zero();
        if index == 0 {
            if negative {
                out.push('-');
            }
        } else {
            out.push(if negative { '-' } else { '+' });
        }
        let magnitude = Fraction::new(coefficient.numerator().abs(), coefficient.denominator());
        // The monomial `1` keys the rational part of the normal form: there is
        // no radical to write, only the coefficient.
        if radicand == &BigInt::from(1) {
            out.push_str(&format_fraction(&magnitude));
        } else if magnitude.is_integer() && magnitude.numerator() == BigInt::from(1) {
            out.push_str(&format!("sqrt({radicand})"));
        } else {
            out.push_str(&format!("{}*sqrt({radicand})", format_fraction(&magnitude)));
        }
    }
    out
}

/// What an error message says it got: a Scalar by its value, because for a
/// count or an index the wrong number is the whole fault (`got 1/2`), and
/// every other operand by its domain (`got String`).
pub fn describe_operand(value: &Value) -> String {
    match &value.data {
        ValueData::Scalar(_) | ValueData::ExactScalar(_) => value.to_string(),
        _ => value.domain_name().to_string(),
    }
}

/// Render a value for an **output** boundary (`PRINT`, LANG.EFFECTS.OUTPUT).
///
/// The stack projection shows a String wrapped in `'...'` so the
/// reader can see that it is a string and not a bare numeric vector. Those
/// quotes are a display affordance of the Stack surface only: at an output
/// boundary the surrounding quotes are dropped and the raw character content
/// is emitted (`'TEST'` on the stack is printed as `TEST`). Quote characters
/// that are part of the content survive unchanged (`'T'ES'T'` prints as
/// `T'ES'T`). Non-text values render exactly as they do on the stack.
pub fn format_for_output(value: &Value) -> String {
    if let ValueData::Text(s) = &value.data {
        return s.to_string();
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::format_exact_real;
    use crate::types::exact::ExactReal;
    use crate::types::fraction::Fraction;
    use num_bigint::BigInt;

    fn sqrt_of(n: i64, d: i64) -> ExactReal {
        ExactReal::from_sqrt_rational(Fraction::new(BigInt::from(n), BigInt::from(d)))
            .expect("a valid sqrt")
    }

    /// The display of an irrational is its normal form as one token: exact,
    /// finite, spaceless, and in the normal form's own term order.
    #[test]
    fn irrational_renders_its_normal_form_as_one_token() {
        assert_eq!(format_exact_real(&sqrt_of(2, 1)), "sqrt(2)");
        assert_eq!(format_exact_real(&sqrt_of(1, 2)), "1/2*sqrt(2)");
        let sqrt2 = sqrt_of(2, 1);
        let one = ExactReal::Rational(Fraction::new(BigInt::from(1), BigInt::from(1)));
        assert_eq!(format_exact_real(&one.add(&sqrt2)), "1/1+sqrt(2)");
        assert_eq!(format_exact_real(&one.sub(&sqrt2)), "1/1-sqrt(2)");
        assert_eq!(
            format_exact_real(&sqrt2.sub(&sqrt_of(3, 1))),
            "sqrt(2)-sqrt(3)"
        );
        assert_eq!(format_exact_real(&sqrt2.neg()), "-sqrt(2)");
        assert_eq!(format_exact_real(&sqrt2.add(&sqrt2)), "2/1*sqrt(2)");
        // One value, one display (LANG.VALUES.DENOTATION): √8 is 2√2 however
        // it was built, so it renders exactly as √2 + √2 does.
        assert_eq!(format_exact_real(&sqrt_of(8, 1)), "2/1*sqrt(2)");
        // A perfect square collapses to the exact rational form.
        assert_eq!(format_exact_real(&sqrt_of(4, 1)), "2/1");
    }
}
