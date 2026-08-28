//! What a Word's declared `nilPolicy` obliges before its primitive runs.
//!
//! `spec/words.json` declares, per Word, what a NIL operand means. Until
//! recently each executor decided that for itself, so the declaration was
//! decorative: `LENGTH` answered `0` for the length of a NIL while declaring
//! `rejectNil`, and `SORT` raised an error while declaring `passthrough`. Both
//! directions of that drift are settled here, in one place that reads the
//! declaration, so no executor can quietly disagree with the canon.

use crate::error::{AjisaiError, Result};
use crate::kernel::generated::{Arity, GeneratedWord, NilPolicy};
use crate::types::Value;

use super::{ConsumptionMode, Interpreter};

/// What a Word's declared `nilPolicy` requires of the operands on the stack,
/// decided before its primitive is reached.
///
/// The passthrough arm names the bubble by its stack position rather than
/// carrying the value: the decision is made from a borrow of the stack, and a
/// position keeps the whole enum a couple of words wide.
enum NilContract {
    /// The declaration places no obligation here; run the primitive.
    Run,
    /// A NIL operand is malformed use for this Word.
    Reject,
    /// A NIL operand is the Word's result; the bubble flows through in place
    /// of running the primitive. `operands` is the declared operand window to
    /// unwind, `bubble` the stack index of the NIL that becomes the result.
    PassThrough { operands: usize, bubble: usize },
}

impl Interpreter {
    /// What the Word's declared NIL contract dictates for the operands
    /// currently on the stack.
    ///
    /// `spec/words.json` declares, per Word, what a NIL operand means. Until
    /// recently each executor decided that for itself, so the declaration was
    /// decorative: `LENGTH` answered `0` for the length of a NIL while
    /// declaring `rejectNil`, and `SORT` raised an error while declaring
    /// `passthrough`. Both directions of that drift are settled here, in one
    /// place that reads the declaration, so no executor can quietly disagree
    /// with the canon.
    ///
    /// The guard reads the declaration and nothing else — no per-family
    /// exception table. A Word whose arity is data-dependent carries no fixed
    /// operand window, so it is left to its executor.
    fn declared_nil_contract(&self, word: &GeneratedWord) -> NilContract {
        let Arity::Fixed(arity) = word.stack_inputs else {
            return NilContract::Run;
        };
        let arity = arity as usize;
        let operands = self.stack.as_slice();

        match word.nil_policy {
            // `rejectNil` binds every operand position, not just the receiver,
            // so a NIL anywhere in the declared arity is malformed use. The
            // window is clamped rather than required: refusing to run touches
            // nothing, so a short stack can be judged on what it holds.
            NilPolicy::RejectNil => {
                let start = operands.len().saturating_sub(arity);
                if operands[start..].iter().any(|operand| operand.is_nil()) {
                    NilContract::Reject
                } else {
                    NilContract::Run
                }
            }
            // A NIL operand *is* the result: the bubble flows downstream
            // carrying its reason (SPEC §7.12, LANG.FAILURE.PASSTHROUGH).
            // `passthroughThenProject` differs only in what non-NIL operands
            // may yield, so a NIL input takes the same route — projecting an
            // absence leaves an absence.
            //
            // Unlike rejection this synthesises a result and unwinds the
            // operands, which needs the whole window present; a short stack is
            // an arity fault, left to the executor to report as underflow.
            NilPolicy::Passthrough | NilPolicy::PassthroughThenProject => {
                if operands.len() < arity {
                    return NilContract::Run;
                }
                // The leftmost bubble wins, matching left-to-right evaluation
                // order and the executor-level helpers it replaces.
                let window = operands.len() - arity;
                match operands[window..]
                    .iter()
                    .position(|operand| operand.is_nil())
                {
                    Some(offset) => NilContract::PassThrough {
                        operands: arity,
                        bubble: window + offset,
                    },
                    None => NilContract::Run,
                }
            }
            // `createsNil` and `preserveReason` describe what the Word does
            // with non-NIL operands; `consumeNil` and `inspectNil` make the NIL
            // itself the Word's subject. None of them constrain dispatch.
            //
            // `kleeneAbsorbing` (strong-Kleene `AND`/`OR`, LANG.VALUES.TRUTH)
            // cannot be decided from a NIL operand alone: whether it settles
            // to a definite result or to UNKNOWN depends on the *other*
            // operand (FALSE absorbs `AND`, TRUE absorbs `OR`), so dispatch
            // must always reach the primitive rather than pre-empt it the way
            // a blanket `passthrough` does.
            NilPolicy::CreatesNil
            | NilPolicy::ConsumeNil
            | NilPolicy::InspectNil
            | NilPolicy::PreserveReason
            | NilPolicy::KleeneAbsorbing => NilContract::Run,
        }
    }

    /// Yield the NIL at stack index `bubble` as the Word's result without
    /// running its primitive, unwinding the declared operand window under the
    /// active consumption mode (SPEC §5.2): `EAT` removes the operands, `KEEP`
    /// leaves them in place. The bubble is copied out before the unwind, since
    /// the unwind is what removes it.
    fn pass_nil_through(&mut self, operands: usize, bubble: usize) {
        let result = Value::nil_inheriting_absence_from(&self.stack[bubble]);
        if self.consumption_mode == ConsumptionMode::Consume {
            let remaining = self.stack.len() - operands;
            self.stack.drain(remaining..);
        }
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
            NilContract::Reject => Some(Err(AjisaiError::create_structure_error("a value", "NIL"))),
            NilContract::PassThrough { operands, bubble } => {
                self.pass_nil_through(operands, bubble);
                Some(Ok(()))
            }
        }
    }
}
