use crate::error::{AjisaiError, NilReason, Result};
use crate::interpreter::cast::cast_value_helpers::{
    apply_unary_cast, format_fraction_to_string, format_value_to_string_repr, is_boolean_value,
    is_number_value,
};
use crate::interpreter::value_extraction_helpers::{create_number_value, value_as_string};
use crate::interpreter::Interpreter;
use crate::semantic::Recoverability;
use crate::types::fraction::Fraction;
use crate::types::{Value, ValueData};

/// Whether any number inside `value` has no lexeme in the sealed numeric
/// grammar — an irrational the source language cannot spell.
///
/// The grammar writes integers and ratios and nothing else, so `2 SQRT` has no
/// faithful text. `STR` used to answer with a continued-fraction convergent
/// anyway: `'665857/470832'`, a *different number*, with no error and no NIL.
/// That is precisely the silent-wrong-answer failure LANG.FAILURE.TRICHOTOMY
/// rules out, and it was worst where it was least visible — `STR` is how a
/// runtime value becomes a `number` token for `REFLECT`, so building a partial
/// application over an exact irrational quietly replaced it with a rational
/// look-alike, in the one value class exactness is the whole point of.
fn has_no_exact_lexeme(value: &Value) -> bool {
    match &value.data {
        ValueData::ExactScalar(exact) => exact.to_fraction().is_none(),
        ValueData::Vector(children) => children.iter().any(has_no_exact_lexeme),
        ValueData::Boolean(_)
        | ValueData::Text(_)
        | ValueData::Scalar(_)
        | ValueData::Tensor { .. }
        | ValueData::Nil
        | ValueData::Symbol(_) => false,
    }
}

fn convert_value_to_string(val: &Value) -> Result<Value> {
    if val.is_nil() {
        return Ok(Value::nil_inheriting_absence_from(val));
    }

    if val.is_text() {
        return Ok(val.clone());
    }

    if is_number_value(val) {
        if let Some(f) = val.as_scalar() {
            let string_repr = format_fraction_to_string(f);
            return Ok(Value::from_string(&string_repr));
        }
    }

    // No lexeme exists, so there is no text to answer with. `STR` projects the
    // same reason `NUM` projects for text that denotes no number: the two are
    // inverses, and this is the direction of the round trip that has no
    // encoding. A program that wants a rational stand-in asks for one by name
    // with `QUANTIZE`, where the denominator is the caller's choice and the
    // approximation is visible in the source.
    if has_no_exact_lexeme(val) {
        return Ok(Value::nil_with_reason(
            NilReason::InvalidEncoding,
            Recoverability::Recoverable,
        ));
    }

    let string_repr = format_value_to_string_repr(val);
    Ok(Value::from_string(&string_repr))
}

pub fn op_str(interp: &mut Interpreter) -> Result<()> {
    apply_unary_cast(interp, convert_value_to_string)
}

/// `NUM` accepts exactly the lexemes a source literal accepts (LANG.SOURCE.TEXT):
/// the sealed numeric grammar is the one definition of what text denotes a
/// number, so `'.5' NUM` fails for the same reason `.5` is not a literal.
///
/// The gate is deliberately the tokenizer's own validator rather than
/// `Fraction::from_str`, which is the *internal* value parser and stays more
/// permissive — it accepts truncated decimals and underscore digit separators
/// (`1_000`), neither of which is Ajisai source. Nothing outside the program can
/// reach this Word, because the vocabulary has no input Word: every operand was
/// written as a literal or built by the program, so leniency here would only
/// create a second numeric language for a program to trip over.
fn convert_value_to_number(val: &Value) -> Result<Value> {
    if val.is_text() {
        let s = value_as_string(val).unwrap_or_default();
        let parsed = if crate::tokenizer::is_number_token_lexeme(&s) {
            Fraction::from_str(&s).ok()
        } else {
            None
        };
        match parsed {
            Some(fraction) => return Ok(create_number_value(fraction)),
            None => {
                return Ok(Value::nil_with_reason(
                    NilReason::InvalidEncoding,
                    Recoverability::Recoverable,
                ));
            }
        }
    }

    if is_number_value(val) {
        return Ok(val.clone());
    }
    if is_boolean_value(val) {
        return Err(AjisaiError::declared(
            "nonText",
            "NUM: expected String, got Boolean",
        ));
    }
    if val.is_nil() {
        return Err(AjisaiError::declared(
            "nonText",
            "NUM: expected String, got Nil",
        ));
    }
    Err(AjisaiError::declared(
        "nonText",
        "NUM: expected String input",
    ))
}

pub fn op_num(interp: &mut Interpreter) -> Result<()> {
    apply_unary_cast(interp, convert_value_to_number)
}
