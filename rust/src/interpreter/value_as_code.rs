//! Bridges a `Value::Vector`'s elements back into `Vec<Token>` so the
//! existing token-based execution engine (`execute_nested_block`, the
//! contract inference walker, a user Word's stored body) keeps running
//! unmodified after the CodeBlock/Vector unification
//! (docs/dev/type-unification-work-order-2026-08.md).
//!
//! `EXEC`, `PROBE`, `DEF`, and the higher-order words (`MAP`/`FILTER`/
//! `FOLD`/`ANY`/`ALL`) all reach a Vector value that needs to run as
//! instructions. Rather than a second execution loop keyed on `&[Value]`,
//! this converts the elements back to tokens and hands them to the existing,
//! already-correct loop (tail-call elimination, `KEEP` handling, error
//! diagnosis context) — the same design `REFLECT` used for its canonical
//! wire format before this unification removed it.
//!
//! A literal's original lexeme is not preserved: a `Value::Scalar` that came
//! from source `1.0` re-synthesizes as `1`. This is not a loss the language
//! cares about — LANG.VALUES.DENOTATION already says a value's construction
//! history is not part of the value, so `1.0` and `1` denoting the same
//! Scalar are `EQ` and were always meant to be indistinguishable once built.
//! An `ExactScalar` element (reachable only via a runtime computation such as
//! `SQRT`, never a literal) has no number-literal lexeme at all — no
//! implementation can synthesize one, since LANG.VALUES.EXACT is explicit
//! that literals are source forms for rationals only — so bridging it is a
//! hard error, a narrow and honest edge rather than a silently wrong answer.

use crate::error::{AjisaiError, Result};
use crate::types::{Token, Value, ValueData};

pub(crate) fn value_elements_to_tokens(elements: &[Value]) -> Result<Vec<Token>> {
    let mut tokens = Vec::with_capacity(elements.len());
    for element in elements {
        push_value_as_tokens(element, &mut tokens)?;
    }
    Ok(tokens)
}

fn push_value_as_tokens(value: &Value, out: &mut Vec<Token>) -> Result<()> {
    match &value.data {
        // `|` round-trips back to the real CondClauseSep token — see
        // vector_literal.rs's matching note on why a bare Symbol("|") is
        // what a clause block's literal collection produces.
        ValueData::Symbol(name) if name.as_ref() == "|" => out.push(Token::CondClauseSep),
        ValueData::Symbol(name) => out.push(Token::Symbol(name.clone())),
        ValueData::Text(s) => out.push(Token::String(s.clone())),
        ValueData::Scalar(f) => out.push(Token::Number(format!("{f}").into())),
        ValueData::Boolean(true) => out.push(Token::Symbol("TRUE".into())),
        ValueData::Boolean(false) => out.push(Token::Symbol("FALSE".into())),
        ValueData::Nil => out.push(Token::Symbol("NIL".into())),
        ValueData::Vector(children) => {
            out.push(Token::VectorStart);
            for child in children.iter() {
                push_value_as_tokens(child, out)?;
            }
            out.push(Token::VectorEnd);
        }
        ValueData::Tensor { .. } => {
            let nested = value
                .as_vector_view()
                .expect("Tensor always has a Vector view");
            out.push(Token::VectorStart);
            for child in nested.iter() {
                push_value_as_tokens(child, out)?;
            }
            out.push(Token::VectorEnd);
        }
        ValueData::ExactScalar(_) => {
            return Err(AjisaiError::from(
                "cannot execute an ExactScalar element: it has no number-literal \
                 lexeme (LANG.VALUES.EXACT — a literal denotes a rational only)",
            ));
        }
    }
    Ok(())
}
