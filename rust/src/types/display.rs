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

pub(crate) fn format_value_recursive(data: &ValueData, depth: usize) -> String {
    super::display_source::render_value(data, depth).source
}

pub(super) fn format_tensor_recursive(
    data: &DenseTensor,
    shape: &[usize],
    _depth: usize,
) -> String {
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
pub(super) fn format_fraction(f: &Fraction) -> String {
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
pub(super) fn format_exact_real(er: &ExactReal) -> String {
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
