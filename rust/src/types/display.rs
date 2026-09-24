use super::exact::ExactReal;
use super::fraction::Fraction;
use super::{DenseTensor, Stack, Value, ValueData};
use num_bigint::BigInt;
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

/// Display budget for lazy continued fractions (LANG.VALUES.EXACT:
/// "implementation-defined display budget").
const CF_DISPLAY_BUDGET: usize = 32;

/// Build the flat CF string from partial quotients, in the classical
/// `[a0; a1, a2, …]` convention (LANG.VALUES.EXACT):
/// finite   [a0]         -> "[ a0 ]"          (no tail, no `;`)
/// finite   [a0,a1,a2]   -> "[ a0; a1, a2 ]"
/// truncated [a0,a1,a2]  -> "[ a0; a1, a2, … ]"
/// truncated [a0]        -> "[ a0; … ]"
/// truncated []          -> "[ … ]"
///
/// The `;` marks the one real distinction the notation carries: `a0` is
/// any integer, while the tail terms are each a positive integer — the
/// partial quotients of a value that is itself always ≥ 1 (the "complete
/// quotient" one level down). The truncation marker is the Unicode
/// ellipsis `…` rather than ASCII `...`: Ajisai numbers never render with
/// a `.` (fractions always print `n/d`), so a literal `.` next to a digit
/// would be the one place a display string could look like a malformed
/// decimal; `…` is a different code point entirely, so no such reading is
/// possible even by accident.
fn render_cf_flat(terms: &[BigInt], truncated: bool) -> String {
    if terms.is_empty() {
        return if truncated {
            "[ … ]".to_string()
        } else {
            "[ ]".to_string()
        };
    }
    let mut s = String::from("[ ");
    s.push_str(&terms[0].to_string());
    if terms.len() > 1 || truncated {
        s.push_str("; ");
        let tail: Vec<String> = terms[1..].iter().map(BigInt::to_string).collect();
        s.push_str(&tail.join(", "));
        if truncated {
            if terms.len() > 1 {
                s.push_str(", …");
            } else {
                s.push('…');
            }
        }
    }
    s.push_str(" ]");
    s
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

/// Display an `ExactReal`. Rational variants use the canonical
/// `numerator/denominator` form. Irrational variants (`AlgebraicSqrt`,
/// `Gosper`) render in the canonical flat continued-fraction form of
/// LANG.VALUES.EXACT — `[ a0; a1, a2 ]` — truncated at the display budget with
/// a trailing `…` for lazy CFs. This keeps the default numeric surface
/// exact and AI-readable: arithmetic on irrationals is computed exactly
/// on the CF representation (Gosper, LANG.VALUES.EXACT), so the display must not
/// collapse it to an approximate rational.
pub(super) fn format_exact_real(er: &ExactReal) -> String {
    match er {
        ExactReal::Rational(f) => format_fraction(f),
        _ => match er.partial_quotients() {
            // Collapsed to a finite (rational) CF: render the exact flat form.
            Some(qs) => render_cf_flat(&qs, false),
            // Lazy irrational: emit partial quotients up to the display budget.
            None => {
                let qs = er.partial_quotients_bounded(CF_DISPLAY_BUDGET);
                if qs.is_empty() {
                    // Not even `a0` was affordable: either a rare Gosper
                    // transform the streaming algorithm does not resolve, or a
                    // value carrying so many algebraic terms that one
                    // floor-and-reciprocate step exceeds the whole expansion
                    // budget. Render the undetermined-CF marker rather than an
                    // empty `[ ]` or an approximate `~` rational — `exactTerms`
                    // beside it still carries the value exactly.
                    "[ … ]".to_string()
                } else {
                    // Always a prefix: this arm is only reached for a value
                    // whose expansion does not terminate.
                    render_cf_flat(&qs, true)
                }
            }
        },
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
    use super::render_cf_flat;
    use num_bigint::BigInt;

    fn bi(n: i64) -> BigInt {
        BigInt::from(n)
    }

    #[test]
    fn render_cf_flat_exact_forms() {
        assert_eq!(render_cf_flat(&[bi(1)], false), "[ 1 ]");
        assert_eq!(render_cf_flat(&[bi(1), bi(2)], false), "[ 1; 2 ]");
        assert_eq!(render_cf_flat(&[bi(1), bi(2), bi(2)], false), "[ 1; 2, 2 ]");
        assert_eq!(
            render_cf_flat(&[bi(1), bi(2), bi(2)], true),
            "[ 1; 2, 2, … ]"
        );
        assert_eq!(render_cf_flat(&[bi(1)], true), "[ 1; … ]");
        assert_eq!(render_cf_flat(&[], false), "[ ]");
        assert_eq!(render_cf_flat(&[], true), "[ … ]");
    }

    #[test]
    fn irrational_renders_as_flat_cf_not_approximation() {
        use super::format_exact_real;
        use crate::types::exact::ExactReal;
        use crate::types::fraction::Fraction;
        use num_bigint::BigInt;

        // √2 = [1; 2, 2, 2, …]. Default display must be the canonical flat
        // CF form (LANG.VALUES.EXACT), never `sqrt(...)` or a `~`-approximation.
        let sqrt2 = ExactReal::from_sqrt_rational(Fraction::new(BigInt::from(2), BigInt::from(1)))
            .expect("√2 is a valid algebraic sqrt");
        let s = format_exact_real(&sqrt2);
        assert!(s.starts_with("[ 1; 2, 2, "), "expected flat CF, got {s:?}");
        assert!(
            s.ends_with(", … ]"),
            "lazy CF must carry the trailing `…` truncation marker, got {s:?}"
        );
        assert!(
            !s.contains("sqrt"),
            "must not use sqrt() display, got {s:?}"
        );
        assert!(
            !s.contains('~'),
            "must not use ~approximation display, got {s:?}"
        );
        assert!(
            !s.contains('.'),
            "CF display must never contain a literal '.', got {s:?}"
        );
        let opens = s.matches('[').count();
        let closes = s.matches(']').count();
        assert_eq!(opens, closes, "unbalanced brackets in {s:?}");

        // A perfect square collapses to the exact rational form.
        let sqrt4 = ExactReal::from_sqrt_rational(Fraction::new(BigInt::from(4), BigInt::from(1)))
            .expect("√4 is a valid sqrt");
        assert_eq!(format_exact_real(&sqrt4), "2/1");
    }

    #[test]
    fn render_cf_flat_balanced_brackets() {
        for terms in [
            vec![bi(1)],
            vec![bi(1), bi(2)],
            vec![bi(2), bi(2), bi(2), bi(2)],
        ] {
            for truncated in [false, true] {
                let s = render_cf_flat(&terms, truncated);
                let opens = s.matches('[').count();
                let closes = s.matches(']').count();
                assert_eq!(opens, closes, "unbalanced brackets in {s:?}");
            }
        }
    }
}
