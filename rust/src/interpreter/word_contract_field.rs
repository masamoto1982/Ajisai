//! The `field` axis of an inferred contract (LANG.CONTRACT.FIELD): whether a
//! body can answer a point over zero from operands that hold none, and which
//! of its Words or literals make it so.
//!
//! Split from `word_contract.rs`, which walks the body; this module answers
//! the two questions the walk asks of one token at a time.
//!
//! **A literal operand can keep a `leaving` Word inside the field.** `DIV` and
//! `POW` are `leaving` because of one operand each: a zero divisor, a negative
//! exponent over a zero base. When the token written immediately before the
//! call is a number literal, that literal *is* the top of the stack when the
//! call runs — nothing can come between them — so the inference can read the
//! operand without running anything. `x 2 DIV` divides by two and `x 2 POW`
//! squares, and neither can leave the field from a finite `x`; `x 0 DIV` and
//! `x -1 POW` still can, and so does a divisor the body computes, since the
//! inference does not evaluate. The refinement is the registry's own reason
//! for `leaving` read at the one place it is decidable, not a special case.

use super::word_contract::{AccumulatedContract, WordContract};
use crate::coreword_registry::FieldClosure;
use crate::types::{Token, Value, ValueData};

/// Whether a literal token pushes one of the three points over zero, at any
/// depth. A body that writes `1/0` leaves the field as surely as one that
/// writes `1 0 DIV`. A lexeme that does not parse is not counted: running it
/// raises before anything is pushed.
pub(super) fn pushes_point_over_zero(token: &Token) -> bool {
    match token {
        Token::Number(literal) => literal.value().is_some_and(|f| !f.is_finite()),
        Token::Value(value) => holds_point_over_zero(value),
        _ => false,
    }
}

fn holds_point_over_zero(value: &Value) -> bool {
    match &value.data {
        ValueData::Scalar(f) => !f.is_finite(),
        ValueData::Vector(items) => items.iter().any(holds_point_over_zero),
        ValueData::Tensor { data, .. } => !data.all_finite(),
        ValueData::Record(record) => record
            .keys()
            .iter()
            .chain(record.values())
            .any(holds_point_over_zero),
        _ => false,
    }
}

/// Whether the call of `name`, written right after `prev`, stays in the field
/// although `name` itself is `leaving`: a `DIV` by a finite non-zero literal,
/// or a `POW` to a finite non-negative literal exponent.
fn literal_operand_keeps_field(name: &str, prev: Option<&Token>) -> bool {
    let Some(Token::Number(literal)) = prev else {
        return false;
    };
    let Some(operand) = literal.value() else {
        return false;
    };
    if !operand.is_finite() {
        return false;
    }
    match name {
        "DIV" => !operand.is_zero(),
        "POW" => operand.signum() != std::cmp::Ordering::Less,
        _ => false,
    }
}

/// Record `exit` as a place the body leaves the field, once.
fn note_field_exit(exits: &mut Vec<String>, exit: &str) {
    if !exits.iter().any(|e| e == exit) {
        exits.push(exit.to_string());
    }
}

impl AccumulatedContract {
    /// [`Self::widen_with`] for a call of `name` written right after `prev`.
    /// A `leaving` callee is an exit from the field unless the literal before
    /// it decides the operand that would have taken it there
    /// (`word_contract_field::literal_operand_keeps_field`).
    pub(crate) fn widen_with_call(
        &mut self,
        name: &str,
        other: &WordContract,
        prev: Option<&Token>,
    ) {
        let field = self.field;
        self.widen_with(other);
        if other.field == FieldClosure::Leaving {
            if literal_operand_keeps_field(name, prev) {
                self.field = field;
            } else {
                note_field_exit(&mut self.field_exits, name);
            }
        }
    }

    pub(super) fn note_literal_over_zero(&mut self, token: &Token) {
        self.field = FieldClosure::Leaving;
        let spelled = match token {
            Token::Number(literal) => literal.lexeme(),
            _ => "a literal over zero",
        };
        note_field_exit(&mut self.field_exits, spelled);
    }
}
