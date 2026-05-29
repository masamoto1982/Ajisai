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
        Interpretation::TruthValue => format_as_boolean(&value.data),
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
            Token::Pipeline => "==".to_string(),
            Token::NilCoalesce => "=>".to_string(),
            Token::CondClauseSep => "$".to_string(),
            Token::SafeMode => "~".to_string(),
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

/// Display an `ExactReal`. Rational variants use the canonical fraction
/// format. `AlgebraicSqrt` variants show `sqrt(p/q)`. Gosper transforms
/// show a best-rational-approximation with a `~` prefix to indicate the
/// value is irrational but displayed approximately.
fn format_exact_real(er: &ExactReal) -> String {
    match er {
        ExactReal::Rational(f) => format_fraction(f),
        ExactReal::AlgebraicSqrt { radicand } => {
            format!("sqrt({})", format_fraction(radicand))
        }
        ExactReal::Gosper(_) => {
            // Best-rational approximation up to denominator 10^6
            match er.best_rational_approximation(&BigInt::from(1_000_000u64)) {
                Some(approx) => format!("~{}", format_fraction(&approx)),
                None => "~?".to_string(),
            }
        }
    }
}

fn format_as_string(data: &ValueData) -> String {
    match data {
        ValueData::Nil => "''".to_string(),
        ValueData::ExactScalar(er) => format!("'{}'", format_exact_real(er)),
        ValueData::Scalar(f) => {
            if let Some(n) = f.to_i64() {
                if n >= 0 && n <= 0x10FFFF {
                    if let Some(c) = char::from_u32(n as u32) {
                        return format!("'{}'", c);
                    }
                }
            }
            format!("'{}'", format_fraction(f))
        }
        ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
            if v.is_empty() {
                return "''".to_string();
            }

            let chars: String = v
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
                .collect();

            format!("'{}'", chars)
        }
        ValueData::Tensor { data, .. } => {
            if data.is_empty() {
                return "''".to_string();
            }
            let chars: String = data
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
                .collect();
            format!("'{}'", chars)
        }
        ValueData::CodeBlock(tokens) => format_code_block(tokens),
        ValueData::ProcessHandle(id) => format!("<process:{}>", id),
        ValueData::SupervisorHandle(id) => format!("<supervisor:{}>", id),
    }
}

fn format_as_boolean(data: &ValueData) -> String {
    match data {
        ValueData::Nil => "NIL".to_string(),
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

            let inner: Vec<&str> = v
                .iter()
                .map(|child| match &child.data {
                    ValueData::Nil => "NIL",
                    ValueData::Scalar(f) => {
                        if f.is_nil() {
                            "NIL"
                        } else if f.is_zero() {
                            "FALSE"
                        } else {
                            "TRUE"
                        }
                    }
                    ValueData::Vector(_) | ValueData::Record { .. } => {
                        let cv = match &child.data {
                            ValueData::Vector(v) => v,
                            ValueData::Record { pairs, .. } => pairs,
                            _ => unreachable!(),
                        };
                        if cv.is_empty() {
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
                })
                .collect();
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
