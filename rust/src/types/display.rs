use super::exact::ExactReal;
use super::fraction::Fraction;
use super::{DenseTensor, Interpretation, Stack, Value, ValueData};
use num_bigint::BigInt;
use std::fmt;

/// Render every stack slot as its observable `(value, role)` string (SPEC §12).
///
/// This is the single stack rendering shared by all observation surfaces — the
/// CLI stack display, the REPL, the in-process conformance runner, and the JSON
/// report — so that an interpretation role such as a timestamp can never
/// render one way on one surface and another way on another. It renders each
/// slot with the *slot's* role rather than the value's construction-time hint,
/// which is what makes it differ from `Value`'s own `Display`.
pub fn render_stack(stack: &Stack) -> Vec<String> {
    stack
        .iter_slots()
        .map(|(value, role)| format_with_hint(value, role))
        .collect()
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format_with_hint(self, self.hint))
    }
}

pub fn format_with_hint(value: &Value, hint: Interpretation) -> String {
    // An operational NIL (a value carrying absence metadata) always renders
    // as `NIL`, regardless of the effective hint. A positional hint can carry
    // a word's declared output role (e.g. CHR is declared to yield TEXT),
    // which must not mask an absence into a bogus `''`/`FALSE`/datetime
    // rendering — the canonical `Display` (which uses the value's own `Nil`
    // hint) already shows `NIL` here, so this keeps hint-driven callers
    // consistent with it. The empty string `''` is itself a NIL with reason
    // `EmptySequence` (see `Value::from_string`), so it likewise renders as
    // `NIL`, matching its canonical form (SPEC §4.5; §12.2).
    if matches!(value.data, ValueData::Nil) && value.absence_metadata().is_some() {
        return "NIL".to_string();
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
        Interpretation::TruthValue => format_as_boolean(value),
        Interpretation::Timestamp => format_as_datetime(&value.data),
        Interpretation::ContinuedFraction => format_as_continued_fraction(value),
    }
}

/// Display budget for lazy continued fractions (SPEC §4.2.3:
/// "implementation-defined display budget").
const CF_DISPLAY_BUDGET: usize = 32;

/// Render a numeric scalar value as the canonical flat continued-fraction
/// form (SPEC §4.2.3): `[ a0; a1, a2 ]`, matching the classical
/// `[a0; a1, a2, …]` notation directly — `[` `]` is the sole bracket in
/// Ajisai, so the CF display uses it as-is rather than standing in for it.
/// Lazy irrationals truncate at CF_DISPLAY_BUDGET terms with a trailing `…`
/// marker.
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
                // Reaching this arm means the value did not collapse to a
                // rational, so its expansion does not terminate and what comes
                // back is always a prefix — however short. Reading truncation
                // off the length was right only while the budget was a term
                // count; now that it is a work budget, a value too expensive to
                // expand returns fewer quotients and would otherwise have been
                // rendered as if complete.
                (er.partial_quotients_bounded(CF_DISPLAY_BUDGET), true)
            }
        },
        // Non-scalar values fall back to the structural rendering.
        _ => return format_value_recursive(&value.data, 0),
    };
    render_cf_flat(&terms, truncated)
}

/// Build the flat CF string from partial quotients, in the classical
/// `[a0; a1, a2, …]` convention (SPEC §4.2.3):
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
        // A String renders quoted at every depth, from its domain alone. This
        // is what replaces the old `Interpretation::Text` dispatch: the
        // Stack surface used to consult a role to decide whether a vector of
        // numbers was "really" text, and now there is nothing to decide.
        ValueData::Text(s) => format!("'{}'", s),
        // The logical Unknown (U — `Nil` carrying the `TruthValue` hint)
        // has no dedicated variant, so it takes the `Nil` arm above and
        // renders as `NIL`, same as an operational NIL.
        // A definite boolean renders uniformly as TRUE/FALSE in every role
        // (LANG.VALUES.TRUTH), so the three-valued axis is observable
        // consistently whether the boolean came from a literal, a
        // comparison, or a logic word. Display-only and non-canonical.
        ValueData::Boolean(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        ValueData::Scalar(f) => format_fraction(f),
        ValueData::ExactScalar(er) => format_exact_real(er),
        ValueData::Vector(v) => {
            if v.is_empty() {
                return "[ ]".to_string();
            }

            let open = '[';
            let close = ']';

            let inner: Vec<String> = v
                .iter()
                .map(|child| {
                    // A nested element keeps its own role: a Text-role child
                    // renders as a quoted string (`'AB'`), so strings stay
                    // recognizable as strings inside a collection (SPEC
                    // §12.2). This now falls out of `format_value_recursive`
                    // dispatching on the String domain, with no role to
                    // consult.
                    format_value_recursive(&child.data, depth + 1)
                })
                .collect();

            format!("{} {} {}", open, inner.join(" "), close)
        }
        ValueData::Tensor { data, shape } => format_tensor_recursive(data, shape, depth),
        // A Symbol renders as its own bare name — unquoted, unlike Text —
        // and every Vector-domain value renders with `[ ]` uniformly now
        // (the old `{ }`-spelled, lexeme-preserving CodeBlock rendering is
        // gone with the CodeBlock domain).
        ValueData::Symbol(name) => name.to_string(),
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
/// `Gosper`) render in the canonical flat continued-fraction form of
/// SPEC §4.2.3 — `[ a0; a1, a2 ]` — truncated at the display budget with
/// a trailing `…` for lazy CFs. This keeps the default numeric surface
/// exact and AI-readable: arithmetic on irrationals is computed exactly
/// on the CF representation (Gosper, SPEC §7.3), so the display must not
/// collapse it to an approximate rational.
fn format_exact_real(er: &ExactReal) -> String {
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
    if let ValueData::Text(s) = &value.data {
        return s.to_string();
    }
    format_with_hint(value, value.hint)
}

/// Boolean label for a single element of a truth-valued vector/tensor. An
/// operational NIL renders as `NIL`; the logical Unknown (U, LANG.VALUES.TRUTH) —
/// `Nil` data carrying the `TruthValue` hint, no dedicated variant — takes
/// the same arm below and renders as `NIL` too.
fn boolean_element_label(child: &Value) -> &'static str {
    match &child.data {
        ValueData::Nil => "NIL",
        // A String is not a truth value, so it has no boolean label; it can
        // only reach here inside a `TruthValue`-role vector, where rendering
        // it as `NIL` matches the other non-numeric arms.
        ValueData::Text(_) => "NIL",
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
        ValueData::Vector(v) => {
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
        ValueData::Symbol(_) => "TRUE",
    }
}

fn format_as_boolean(value: &Value) -> String {
    match &value.data {
        // The logical Unknown (U — `Nil` carrying the `TruthValue` hint,
        // no dedicated variant) takes this same arm and renders as `NIL`,
        // same as an operational NIL.
        ValueData::Nil => "NIL".to_string(),
        ValueData::Text(_) => format_value_recursive(&value.data, 0),
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
        // A TruthValue-hinted Vector renders with `[ ]`, the one spelling
        // every Vector-domain value now uses uniformly (`{ }` was a second,
        // colliding spelling for exactly this shape before the CodeBlock/
        // Vector unification — LANG.VALUES.DISJOINT never had two renderings
        // for one value, and now it does not have to). Each element still
        // renders as its truth-role label (TRUE/FALSE/NIL), independent of
        // that outer bracket choice.
        ValueData::Vector(v) => {
            if v.is_empty() {
                return "[ ]".to_string();
            }

            let inner: Vec<&str> = v.iter().map(boolean_element_label).collect();
            format!("[ {} ]", inner.join(" "))
        }
        ValueData::Tensor { data, .. } => {
            if data.is_empty() {
                return "[ ]".to_string();
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
            format!("[ {} ]", inner.join(" "))
        }
        ValueData::Symbol(name) => name.to_string(),
    }
}

fn format_as_datetime(data: &ValueData) -> String {
    match data {
        ValueData::Nil => format_value_recursive(data, 0),
        ValueData::Text(_) | ValueData::Boolean(_) | ValueData::Symbol(_) => {
            format_value_recursive(data, 0)
        }
        ValueData::ExactScalar(er) => format!("@{}", format_exact_real(er)),
        ValueData::Scalar(f) => format!("@{}", format_fraction(f)),
        ValueData::Vector(_) | ValueData::Tensor { .. } => format_value_recursive(data, 0),
    }
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
        // CF form (SPEC §4.2.3), never `sqrt(...)` or a `~`-approximation.
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
