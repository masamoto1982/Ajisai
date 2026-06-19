use super::continued_fraction::ExactReal;
use super::fraction::Fraction;
use super::{DenseTensor, Interpretation, Value, ValueData};
use num_bigint::BigInt;
use std::fmt;

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format_with_hint(self, self.hint))
    }
}

pub fn format_with_hint(value: &Value, hint: Interpretation) -> String {
    // The logical Unknown (U, SPEC §7.5) always renders as `UNKNOWN`,
    // regardless of the effective hint, so it is never shown as `NIL`.
    // Display-only and non-canonical (SPEC §12.2).
    if value.is_unknown() {
        return "UNKNOWN".to_string();
    }
    match hint {
        Interpretation::Nil => {
            if matches!(value.data, ValueData::Nil) {
                "NIL".to_string()
            } else {
                format_value_recursive(&value.data, 0)
            }
        }
        // Unassigned renders the value in its raw structural form. The
        // runtime never re-guesses a richer meaning (e.g. "string-like")
        // at render time; interpretation is decided once, at construction.
        Interpretation::Unassigned => format_value_recursive(&value.data, 0),
        Interpretation::RawNumber => format_value_recursive(&value.data, 0),
        Interpretation::Interval => format_as_interval(value),
        Interpretation::Text => format_as_string(&value.data),
        Interpretation::TruthValue => format_as_boolean(value),
        Interpretation::Timestamp => format_as_datetime(&value.data),
        Interpretation::ContinuedFraction => format_as_continued_fraction(value),
    }
}

/// Display budget for lazy continued fractions (SPEC §4.2.3:
/// "implementation-defined display budget").
const CF_DISPLAY_BUDGET: usize = 32;

/// Render a numeric scalar value as the canonical nested
/// continued-fraction form (SPEC §4.2.3): `( a0 ( a1 ( a2 ) ) )`.
/// Lazy irrationals truncate at CF_DISPLAY_BUDGET terms with a `...)`
/// marker on the innermost level.
pub(crate) fn format_as_continued_fraction(value: &Value) -> String {
    // Obtain the partial-quotient sequence and whether it is truncated.
    let (terms, truncated): (Vec<BigInt>, bool) = match &value.data {
        ValueData::Scalar(f) => {
            // Rational: finite canonical CF.
            match ExactReal::from_fraction(f.clone()).partial_quotients() {
                Some(qs) => (qs, false),
                None => (Vec::new(), false), // nil fraction
            }
        }
        ValueData::ExactScalar(er) => match er.partial_quotients() {
            Some(qs) => (qs, false), // collapsed to rational
            None => {
                let qs = er.partial_quotients_bounded(CF_DISPLAY_BUDGET);
                let truncated = qs.len() == CF_DISPLAY_BUDGET;
                (qs, truncated)
            }
        },
        // Non-scalar values fall back to the structural rendering.
        _ => return format_value_recursive(&value.data, 0),
    };
    render_cf_nested(&terms, truncated)
}

/// Build the nested right-associative CF string from partial quotients.
/// finite [a0,a1,a2] -> "( a0 ( a1 ( a2 ) ) )"
/// truncated [a0,a1,a2] -> "( a0 ( a1 ( a2 ...) ) )"
fn render_cf_nested(terms: &[BigInt], truncated: bool) -> String {
    if terms.is_empty() {
        return "( )".to_string();
    }
    let mut s = String::new();
    for t in terms {
        s.push_str("( ");
        s.push_str(&t.to_string());
        s.push(' ');
    }
    if truncated {
        // The `...)` marker provides the innermost closing paren.
        s.push_str("...)");
    } else {
        s.push(')');
    }
    // Close the remaining opened levels.
    for _ in 0..terms.len() - 1 {
        s.push_str(" )");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::render_cf_nested;
    use num_bigint::BigInt;

    fn bi(n: i64) -> BigInt {
        BigInt::from(n)
    }

    #[test]
    fn render_cf_nested_exact_forms() {
        assert_eq!(render_cf_nested(&[bi(1)], false), "( 1 )");
        assert_eq!(render_cf_nested(&[bi(1), bi(2)], false), "( 1 ( 2 ) )");
        assert_eq!(
            render_cf_nested(&[bi(1), bi(2), bi(2)], false),
            "( 1 ( 2 ( 2 ) ) )"
        );
        assert_eq!(
            render_cf_nested(&[bi(1), bi(2), bi(2)], true),
            "( 1 ( 2 ( 2 ...) ) )"
        );
        assert_eq!(render_cf_nested(&[], false), "( )");
    }

    #[test]
    fn irrational_renders_as_nested_cf_not_approximation() {
        use super::format_exact_real;
        use crate::types::continued_fraction::ExactReal;
        use crate::types::fraction::Fraction;
        use num_bigint::BigInt;

        // √2 = [1; 2, 2, 2, …]. Default display must be the canonical nested
        // CF form (SPEC §4.2.3), never `sqrt(...)` or a `~`-approximation.
        let sqrt2 = ExactReal::from_sqrt_rational(Fraction::new(BigInt::from(2), BigInt::from(1)))
            .expect("√2 is a valid algebraic sqrt");
        let s = format_exact_real(&sqrt2);
        assert!(
            s.starts_with("( 1 ( 2 ( 2 "),
            "expected nested CF, got {s:?}"
        );
        assert!(
            s.contains("...)"),
            "lazy CF must carry the `...)` truncation marker, got {s:?}"
        );
        assert!(
            !s.contains("sqrt"),
            "must not use sqrt() display, got {s:?}"
        );
        assert!(
            !s.contains('~'),
            "must not use ~approximation display, got {s:?}"
        );
        let opens = s.matches('(').count();
        let closes = s.matches(')').count();
        assert_eq!(opens, closes, "unbalanced parens in {s:?}");

        // A perfect square collapses to the exact rational form.
        let sqrt4 = ExactReal::from_sqrt_rational(Fraction::new(BigInt::from(4), BigInt::from(1)))
            .expect("√4 is a valid sqrt");
        assert_eq!(format_exact_real(&sqrt4), "2/1");
    }

    #[test]
    fn render_cf_nested_balanced_parens() {
        for terms in [
            vec![bi(1)],
            vec![bi(1), bi(2)],
            vec![bi(2), bi(2), bi(2), bi(2)],
        ] {
            for truncated in [false, true] {
                let s = render_cf_nested(&terms, truncated);
                let opens = s.matches('(').count();
                let closes = s.matches(')').count();
                assert_eq!(opens, closes, "unbalanced parens in {s:?}");
            }
        }
    }
}

fn format_as_interval(value: &Value) -> String {
    match &value.data {
        ValueData::Vector(v) if v.len() == 2 => {
            let lo = match &v[0].data {
                ValueData::Scalar(f) => format_fraction(f),
                _ => format_value_recursive(&v[0].data, 0),
            };
            let hi = match &v[1].data {
                ValueData::Scalar(f) => format_fraction(f),
                _ => format_value_recursive(&v[1].data, 0),
            };
            format!("[{}, {}]", lo, hi)
        }
        ValueData::Tensor { data, shape } if shape.as_slice() == [2] && data.len() == 2 => {
            format!(
                "[{}, {}]",
                format_fraction(&data.fraction_or_nil(0)),
                format_fraction(&data.fraction_or_nil(1))
            )
        }
        _ => format_value_recursive(&value.data, 0),
    }
}

fn format_value_recursive(data: &ValueData, depth: usize) -> String {
    match data {
        ValueData::Nil => "NIL".to_string(),
        // A definite boolean renders uniformly as TRUE/FALSE in every role
        // (SPEC §12.2), so the three-valued axis is observable consistently
        // whether the boolean came from a literal, a comparison, or a logic
        // word. Display-only and non-canonical.
        ValueData::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        ValueData::Scalar(f) => format_fraction(f),
        ValueData::ExactScalar(er) => format_exact_real(er),
        ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
            if v.is_empty() {
                return "[ ]".to_string();
            }

            let open = '[';
            let close = ']';

            let inner: Vec<String> = v
                .iter()
                .map(|child| format_value_recursive(&child.data, depth + 1))
                .collect();

            format!("{} {} {}", open, inner.join(" "), close)
        }
        ValueData::Tensor { data, shape } => format_tensor_recursive(data, shape, depth),
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
        ValueData::ProcessHandle(id) => format!("<process:{}>", id),
        ValueData::SupervisorHandle(id) => format!("<supervisor:{}>", id),
    }
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

fn format_code_block(tokens: &[super::Token]) -> String {
    use super::Token;
    let token_strs: Vec<String> = tokens
        .iter()
        .map(|t| match t {
            Token::Number(n) => n.to_string(),
            Token::String(s) => format!("'{}'", s),
            Token::Symbol(s) => s.to_string(),
            Token::VectorStart => "[".to_string(),
            Token::VectorEnd => "]".to_string(),
            Token::BlockStart => "{".to_string(),
            Token::BlockEnd => "}".to_string(),
            Token::Pipeline => "~".to_string(),
            Token::NilCoalesce => "^".to_string(),
            Token::CondClauseSep => "|".to_string(),
            Token::LineBreak => "\n".to_string(),
        })
        .collect();
    token_strs.join(" ")
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

/// Display an `ExactReal`. Rational variants use the canonical
/// `numerator/denominator` form. Irrational variants (`AlgebraicSqrt`,
/// `Gosper`) render in the canonical nested continued-fraction form of
/// SPEC §4.2.3 — `( a0 ( a1 ( a2 ... ) ) )` — truncated at the display
/// budget with a `...)` marker for lazy CFs. This keeps the default
/// numeric surface exact and AI-readable: arithmetic on irrationals is
/// computed exactly on the CF representation (Gosper, SPEC §7.3), so the
/// display must not collapse it to an approximate rational.
fn format_exact_real(er: &ExactReal) -> String {
    match er {
        ExactReal::Rational(f) => format_fraction(f),
        _ => match er.partial_quotients() {
            // Collapsed to a finite (rational) CF: render the exact nested form.
            Some(qs) => render_cf_nested(&qs, false),
            // Lazy irrational: emit partial quotients up to the display budget.
            None => {
                let qs = er.partial_quotients_bounded(CF_DISPLAY_BUDGET);
                if qs.is_empty() {
                    // The emitter could not determine even a0 within the
                    // display budget (a rare Gosper transform — e.g. a product
                    // of equal surds that is exactly rational but whose CF the
                    // streaming algorithm does not resolve in budget). Render
                    // the undetermined-CF marker rather than an empty `( )` or
                    // an approximate `~` rational.
                    "( ...)".to_string()
                } else {
                    let truncated = qs.len() == CF_DISPLAY_BUDGET;
                    render_cf_nested(&qs, truncated)
                }
            }
        },
    }
}

/// Render a value for an **output** boundary (`PRINT`, SPEC §7.9).
///
/// The stack projection shows a Text-role value wrapped in `'...'` so the
/// reader can see that it is a string and not a bare numeric vector. Those
/// quotes are a display affordance of the Stack surface only: at an output
/// boundary the surrounding quotes are dropped and the raw character content
/// is emitted (`'TEST'` on the stack is printed as `TEST`). Quote characters
/// that are part of the content survive unchanged (`'T'ES'T'` prints as
/// `T'ES'T`). Non-text values render exactly as they do on the stack.
pub fn format_for_output(value: &Value) -> String {
    if value.is_unknown() {
        return "UNKNOWN".to_string();
    }
    if value.hint == Interpretation::Text {
        return format_text_content(&value.data);
    }
    format_with_hint(value, value.hint)
}

fn format_as_string(data: &ValueData) -> String {
    match data {
        // These variants are not character data; they carry no surrounding
        // quotes in the stack projection either, so reuse their bare form.
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
        ValueData::ProcessHandle(id) => format!("<process:{}>", id),
        ValueData::SupervisorHandle(id) => format!("<supervisor:{}>", id),
        _ => format!("'{}'", format_text_content(data)),
    }
}

/// Decode the raw character content of a Text-role value, without the
/// surrounding display quotes. This is the body shared by `format_as_string`
/// (which wraps it in `'...'` for the stack projection) and
/// `format_for_output` (which emits it bare for `PRINT`).
fn format_text_content(data: &ValueData) -> String {
    match data {
        ValueData::Nil => String::new(),
        ValueData::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        ValueData::ExactScalar(er) => format_exact_real(er),
        ValueData::Scalar(f) => {
            if let Some(n) = f.to_i64() {
                if n >= 0 && n <= 0x10FFFF {
                    if let Some(c) = char::from_u32(n as u32) {
                        return c.to_string();
                    }
                }
            }
            format_fraction(f)
        }
        ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => v
            .iter()
            .filter_map(|child| {
                if let ValueData::Scalar(f) = &child.data {
                    f.to_i64().and_then(|n| {
                        if n >= 0 && n <= 0x10FFFF {
                            char::from_u32(n as u32)
                        } else {
                            None
                        }
                    })
                } else {
                    None
                }
            })
            .collect(),
        ValueData::Tensor { data, .. } => data
            .iter()
            .filter_map(|f| {
                f.to_i64().and_then(|n| {
                    if n >= 0 && n <= 0x10FFFF {
                        char::from_u32(n as u32)
                    } else {
                        None
                    }
                })
            })
            .collect(),
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
        ValueData::ProcessHandle(id) => format!("<process:{}>", id),
        ValueData::SupervisorHandle(id) => format!("<supervisor:{}>", id),
    }
}

/// Boolean label for a single element of a truth-valued vector/tensor.
/// The logical Unknown (U, SPEC §7.5) renders as `UNKNOWN`; an
/// operational NIL stays `NIL`.
fn boolean_element_label(child: &Value) -> &'static str {
    if child.is_unknown() {
        return "UNKNOWN";
    }
    match &child.data {
        ValueData::Nil => "NIL",
        ValueData::Boolean(b) => {
            if *b {
                "TRUE"
            } else {
                "FALSE"
            }
        }
        ValueData::Scalar(f) => {
            if f.is_nil() {
                "NIL"
            } else if f.is_zero() {
                "FALSE"
            } else {
                "TRUE"
            }
        }
        ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
            if v.is_empty() {
                "FALSE"
            } else {
                "TRUE"
            }
        }
        ValueData::Tensor { data, .. } => {
            if data.is_empty() {
                "FALSE"
            } else {
                "TRUE"
            }
        }
        ValueData::ExactScalar(_) => "TRUE",
        ValueData::CodeBlock(_) => "TRUE",
        ValueData::ProcessHandle(_) | ValueData::SupervisorHandle(_) => "TRUE",
    }
}

fn format_as_boolean(value: &Value) -> String {
    // The logical Unknown is handled by `format_with_hint`, but guard
    // here too so the function is correct in isolation.
    if value.is_unknown() {
        return "UNKNOWN".to_string();
    }
    match &value.data {
        ValueData::Nil => "NIL".to_string(),
        ValueData::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        // ExactScalar values are always non-zero positive irrationals → TRUE
        ValueData::ExactScalar(_) => "TRUE".to_string(),
        ValueData::Scalar(f) => {
            if f.is_nil() {
                "NIL".to_string()
            } else if f.is_zero() {
                "FALSE".to_string()
            } else {
                "TRUE".to_string()
            }
        }
        ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
            if v.is_empty() {
                return "FALSE".to_string();
            }

            let inner: Vec<&str> = v.iter().map(boolean_element_label).collect();
            format!("{{ {} }}", inner.join(" "))
        }
        ValueData::Tensor { data, .. } => {
            if data.is_empty() {
                return "FALSE".to_string();
            }
            let inner: Vec<&str> = data
                .iter()
                .map(|f| {
                    if f.is_nil() {
                        "NIL"
                    } else if f.is_zero() {
                        "FALSE"
                    } else {
                        "TRUE"
                    }
                })
                .collect();
            format!("{{ {} }}", inner.join(" "))
        }
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
        ValueData::ProcessHandle(id) => format!("<process:{}>", id),
        ValueData::SupervisorHandle(id) => format!("<supervisor:{}>", id),
    }
}

fn format_as_datetime(data: &ValueData) -> String {
    match data {
        ValueData::Nil => format_value_recursive(data, 0),
        ValueData::Boolean(_) => format_value_recursive(data, 0),
        ValueData::ExactScalar(er) => format!("@{}", format_exact_real(er)),
        ValueData::Scalar(f) => format!("@{}", format_fraction(f)),
        ValueData::Vector(_) | ValueData::Tensor { .. } | ValueData::Record { .. } => {
            format_value_recursive(data, 0)
        }
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
        ValueData::ProcessHandle(id) => format!("<process:{}>", id),
        ValueData::SupervisorHandle(id) => format!("<supervisor:{}>", id),
    }
}
