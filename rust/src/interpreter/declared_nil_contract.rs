//! What a Word's declared operand roles oblige before its primitive runs.
//!
//! `spec/words.json` declares, per Word, what it does with each operand
//! (`stack.operands`, LANG.FAILURE.PASSTHROUGH): a `data` operand is read, so
//! an absent one makes the result that absence; an `element` is carried
//! without being read, so a NIL there is an ordinary value; a `control`
//! operand — a block, a name, a message — cannot be absent, so a NIL there is
//! malformed use; a `truth` operand reads NIL as UNKNOWN. This is the one
//! place that reads those roles, so no executor can quietly disagree with the
//! canon, and two Words that treat an operand alike treat a NIL there alike.

use crate::error::{AjisaiError, Result};
use crate::kernel::generated::{Arity, GeneratedWord, OperandRole};
use crate::types::Value;

use super::Interpreter;

/// What the Word's declared operand roles require of the operands on the
/// stack, decided before its primitive is reached.
///
/// The passthrough arm names the projected NIL by its stack position rather
/// than carrying the value: the decision is made from a borrow of the stack,
/// and a position keeps the whole enum a couple of words wide.
enum NilContract {
    /// The declaration places no obligation here; run the primitive.
    Run,
    /// A NIL in a `control` position. `offset` is its position within the
    /// declared arity window, left to right in source order (0 = the
    /// first-pushed operand).
    Reject { offset: usize },
    /// A NIL in a `data` position is the Word's result; it flows through in
    /// place of running the primitive. `operands` is the declared operand
    /// window to unwind, `nil_index` the stack index of the NIL that becomes
    /// the result.
    PassThrough { operands: usize, nil_index: usize },
}

/// The declared condition a Word raises for a NIL in a `control` position:
/// the same condition its primitive raises for any other operand that is not
/// a block, a name or a message there, read from that Word's own `errorWhen`.
///
/// A table rather than a derivation because `errorWhen` lists every condition
/// a Word can raise, not which one belongs to which position (`DEF` declares
/// six, of which `invalidDefinitionBody` is its block's and `nonText` its
/// name's). `every_program_position_has_a_condition` pins the table against
/// the registry, so a new `control` position without an arm fails the build's
/// tests rather than panicking at runtime.
fn nil_rejection_error(word_name: &str, offset: usize) -> AjisaiError {
    const CODE: &str = "expected a Vector ([ ... ]) as the code operand, got NIL";
    match (word_name, offset) {
        ("EXEC", 0) | ("MAP", 1) | ("FILTER", 1) | ("FOLD", 2) | ("SCAN", 2) => {
            AjisaiError::declared("notExecutable", CODE)
        }
        ("CONTRACT", 0) => {
            AjisaiError::declared("notASymbol", "expected a Symbol naming a Word, got NIL")
        }
        ("FAIL", 0) => AjisaiError::declared("nonText", "expected a String message, got NIL"),
        ("ABSENT", 0) => AjisaiError::declared("nonText", "expected a String reason, got NIL"),
        ("BIND", 1) => AjisaiError::declared(
            "nonText",
            "expected a name (String) or a Vector of names, got NIL",
        ),
        ("DEF", 0) => AjisaiError::declared(
            "invalidDefinitionBody",
            "expected a Vector [ ... ] definition body, got NIL",
        ),
        ("DEF", 1) | ("DEL", 0) => {
            AjisaiError::declared("nonText", "expected a name (String), got NIL")
        }
        (word, offset) => unreachable!(
            "no declared condition registered for a NIL in {word}'s program operand {offset} — \
             add one to declared_nil_contract::nil_rejection_error"
        ),
    }
}

impl Interpreter {
    /// What the Word's declared operand roles dictate for the operands
    /// currently on the stack.
    ///
    /// A NIL in a `control` position is refused first, wherever it sits: a
    /// block, name or message that is absent is malformed use whatever else
    /// is absent beside it (LANG.FAILURE.TRICHOTOMY puts an ERROR ahead of a
    /// NIL). Otherwise the leftmost NIL in a `data` position is the result,
    /// matching left-to-right evaluation order. `element` and `truth`
    /// positions leave the NIL to the primitive, which treats it as a value
    /// or as UNKNOWN. A Word of data-dependent arity carries no fixed operand
    /// window, so it is left to its executor.
    fn declared_nil_contract(&self, word: &GeneratedWord) -> NilContract {
        let Arity::Fixed(arity) = word.stack_inputs else {
            return NilContract::Run;
        };
        let arity = arity as usize;
        let operands = self.stack.as_slice();

        // Refusing to run touches nothing, so a short stack can be judged on
        // what it holds: the window is clamped rather than required.
        let available = arity.min(operands.len());
        let window = &operands[operands.len() - available..];
        let roles = &word.operand_roles[arity - available..];
        if let Some(offset) = window
            .iter()
            .zip(roles)
            .position(|(operand, role)| *role == OperandRole::Control && operand.is_nil())
        {
            return NilContract::Reject {
                offset: offset + (arity - available),
            };
        }

        // Passing a NIL through unwinds the operands and synthesises a
        // result, which needs the whole window present; a short stack is an
        // arity fault, left to the executor to report as underflow.
        if available < arity {
            return NilContract::Run;
        }
        match window.iter().zip(roles).position(|(operand, role)| {
            matches!(role, OperandRole::Data | OperandRole::Leaf) && operand.is_nil()
        }) {
            Some(offset) => NilContract::PassThrough {
                operands: arity,
                nil_index: operands.len() - arity + offset,
            },
            None => NilContract::Run,
        }
    }

    /// Yield the NIL at stack index `nil_index` as the Word's result without
    /// running its primitive, removing the declared operand window. The NIL is
    /// copied out before the unwind, since the unwind is what removes it.
    fn pass_nil_through(&mut self, operands: usize, nil_index: usize) {
        let result = Value::nil_inheriting_absence_from(&self.stack[nil_index]);
        let remaining = self.stack.len() - operands;
        self.stack.drain(remaining..);
        self.stack.push(result);
    }

    /// Settle the Word's declared NIL contract against the current stack.
    ///
    /// `None` means the declaration places no obligation here and the primitive
    /// must run; `Some(result)` is the Word's outcome, decided without running
    /// it. Every dispatch path must consult this — a path that skips it is a
    /// path on which the declaration is decorative again.
    pub(super) fn apply_declared_nil_contract(
        &mut self,
        word: &GeneratedWord,
    ) -> Option<Result<()>> {
        match self.declared_nil_contract(word) {
            NilContract::Run => None,
            // A NIL where a block, name or message belongs is refused with
            // the same declared condition the primitive raises for any other
            // operand that is not one, so the two mistakes read alike.
            NilContract::Reject { offset } => Some(Err(nil_rejection_error(word.name, offset))),
            NilContract::PassThrough {
                operands,
                nil_index,
            } => {
                self.pass_nil_through(operands, nil_index);
                Some(Ok(()))
            }
        }
    }
}

#[cfg(test)]
mod declared_nil_contract_tests {
    use super::nil_rejection_error;
    use crate::kernel::generated::{OperandRole, GENERATED_WORDS};

    /// Every `control` position has a registered condition for a NIL, and
    /// each is one its Word declares — `nil_rejection_error` panics on a
    /// missing arm, so this is what catches a new `control` operand before
    /// it ships.
    #[test]
    fn every_program_position_has_a_condition() {
        for word in GENERATED_WORDS {
            for (offset, role) in word.operand_roles.iter().enumerate() {
                if *role != OperandRole::Control {
                    continue;
                }
                let crate::error::AjisaiError::DeclaredCondition { condition, .. } =
                    nil_rejection_error(word.name, offset)
                else {
                    panic!("{} operand {offset}: not a declared condition", word.name);
                };
                assert!(
                    word.error_when.contains(&condition),
                    "{} operand {offset}: {condition} is not in its errorWhen",
                    word.name
                );
            }
        }
    }
}
