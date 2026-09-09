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
/// The passthrough arm names the projected NIL by its stack position rather
/// than carrying the value: the decision is made from a borrow of the stack,
/// and a position keeps the whole enum a couple of words wide.
enum NilContract {
    /// The declaration places no obligation here; run the primitive.
    Run,
    /// A NIL operand is malformed use for this Word. `offset` is its position
    /// within the declared arity window, left to right in source order (0 =
    /// the first-pushed operand) — the same indexing `nil_rejection_error`
    /// reads to pick the Word's own declared condition for that position.
    Reject { offset: usize },
    /// A NIL operand is the Word's result; it flows through in place of
    /// running the primitive. `operands` is the declared operand window to
    /// unwind, `nil_index` the stack index of the NIL that becomes the
    /// result.
    PassThrough { operands: usize, nil_index: usize },
}

/// The declared condition a `rejectNil` Word raises for a NIL found at
/// `offset` within its arity window (left to right in source order).
///
/// `apply_declared_nil_contract` decides *whether* a Word rejects a NIL —
/// that part reads only `spec/words.json`'s `nilPolicy` and is Word-agnostic.
/// *What it says* cannot be: every `rejectNil` Word's own primitive already
/// raises the right declared condition for a non-NIL malformed operand (a
/// `nonVector` LENGTH won't take, a `nonText` TRIM won't take), because
/// LANG.VALUES.NIL makes NIL as ordinary an operand as any other — a
/// `rejectNil` Word simply doesn't include it in its domain. So the message
/// this function builds for a *NIL* operand names the identical condition,
/// read here from each Word's own `errorWhen` (`spec/words.json`) rather than
/// generated generically, which is what left every one of these 13 Words
/// showing `structureError` for a NIL operand before this fix
/// (`docs/dev/auditable-kernel-work-order-2026-09.md` Phase 1).
///
/// This has to be a per-Word table rather than a derivation, the same way
/// `error_flow_trace_tests.rs`'s `declared_condition_tests` names call sites
/// by hand: `errorWhen` lists every condition a Word *can* raise, not which
/// one is *the* NIL-rejection condition when a Word raises more than one (`DEF`
/// declares `invalidName`, `protectedWord`, `definitionConflict`,
/// `selfReferentialDefinition`, `nonText` and `invalidDefinitionBody` — only
/// the last two are about operand shape at all, and which of those two
/// applies depends on *which* operand position is NIL). A new `rejectNil`
/// Word needs an arm added here — `declared_nil_contract_tests` pins the
/// current 13 so a missing arm is a compile error, not a silent fallback to
/// `structureError`.
fn nil_rejection_error(word_name: &str, offset: usize) -> AjisaiError {
    match (word_name, offset) {
        // Arity 1 — the whole window is offset 0.
        ("LENGTH", 0) => AjisaiError::declared(
            "nonVector",
            "LENGTH: expected a Vector, got Nil",
        ),
        ("REVERSE", 0) => AjisaiError::declared(
            "nonVector",
            "REVERSE: expected a Vector, got Nil",
        ),
        ("CHARS", 0) => AjisaiError::declared("nonText", "CHARS: expected String, got Nil"),
        ("JOIN", 0) => AjisaiError::declared("nonTextVector", "JOIN: expected Vector, got Nil"),
        ("TRIM", 0) => AjisaiError::declared("nonText", "TRIM: expected String, got Nil"),
        ("EXEC", 0) => AjisaiError::declared(
            "notExecutable",
            "EXEC: expected a Vector ([ ... ]) as the code operand, got Nil",
        ),
        ("PROBE", 0) => AjisaiError::declared("notExecutable", "PROBE requires a CodeBlock"),
        ("DEL", 0) => AjisaiError::declared("nonText", "expected a name (String), got Nil"),

        // Arity 2, uniform across both positions.
        ("RANDOM", _) => AjisaiError::declared("nonInteger", "expected an integer, got NIL"),
        ("CONCAT", _) => AjisaiError::declared(
            "nonVector",
            "CONCAT: expected two Vectors, got a non-vector operand",
        ),

        // Arity 2, position-dependent: offset 0 is the first-pushed (leftmost
        // in source) operand, offset 1 the one immediately before the Word.
        ("TAKE", 0) => AjisaiError::declared(
            "nonVector",
            "expected a Vector, got a non-vector value",
        ),
        ("TAKE", 1) => {
            AjisaiError::declared("invalidCount", "TAKE: expected an integer count, got NIL")
        }
        ("TOKENIZE", 0) => {
            AjisaiError::declared("nonText", "TOKENIZE: expected String, got Nil")
        }
        ("TOKENIZE", 1) => AjisaiError::declared(
            "nonTextSeparator",
            "TOKENIZE: expected separator String, got Nil",
        ),
        ("DEF", 0) => AjisaiError::declared(
            "invalidDefinitionBody",
            "DEF: expected a Vector [ ... ] definition body, got Nil",
        ),
        ("DEF", 1) => {
            AjisaiError::declared("nonText", "expected a name (String), got Nil")
        }

        (word, offset) => unreachable!(
            "no declared NIL-rejection condition registered for {word} at operand offset {offset} — \
             add one to declared_nil_contract::nil_rejection_error (see its doc comment)"
        ),
    }
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
                match operands[start..]
                    .iter()
                    .position(|operand| operand.is_nil())
                {
                    Some(offset) => NilContract::Reject { offset },
                    None => NilContract::Run,
                }
            }
            // A NIL operand *is* the result: it flows downstream carrying
            // its reason (LANG.FAILURE.PASSTHROUGH, LANG.FAILURE.PASSTHROUGH).
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
                // The leftmost NIL wins, matching left-to-right evaluation
                // order and the executor-level helpers it replaces.
                let window = operands.len() - arity;
                match operands[window..]
                    .iter()
                    .position(|operand| operand.is_nil())
                {
                    Some(offset) => NilContract::PassThrough {
                        operands: arity,
                        nil_index: window + offset,
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

    /// Yield the NIL at stack index `nil_index` as the Word's result without
    /// running its primitive, unwinding the declared operand window under the
    /// active consumption mode (LANG.MODIFIERS.CONSUMPTION): `EAT` removes the operands, `KEEP`
    /// leaves them in place. The NIL is copied out before the unwind, since
    /// the unwind is what removes it.
    fn pass_nil_through(&mut self, operands: usize, nil_index: usize) {
        let result = Value::nil_inheriting_absence_from(&self.stack[nil_index]);
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
            // A NIL operand is UNKNOWN, not a foreign type
            // (`LANG.VALUES.NIL`); this *Word* declares `nilPolicy:
            // rejectNil`, so its own domain simply doesn't include it — the
            // identical refusal a non-NIL operand outside that domain gets.
            // `nil_rejection_error` names the same declared condition the
            // Word's own primitive raises for that operand position, so a
            // NIL and a same-shaped non-NIL mistake are reported alike.
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
    use crate::kernel::generated::{Arity, NilPolicy, GENERATED_WORDS};

    /// Every `rejectNil` Word of fixed arity must have a registered condition
    /// for a NIL at each of its operand positions — `nil_rejection_error`
    /// panics on a missing arm, so this is what catches a new `rejectNil`
    /// Word (or a widened arity on an existing one) that forgot to add one,
    /// before it ships as a silent `structureError` regression.
    #[test]
    fn every_reject_nil_word_covers_every_operand_offset() {
        let mut checked = 0;
        for word in GENERATED_WORDS {
            if word.nil_policy != NilPolicy::RejectNil {
                continue;
            }
            let Arity::Fixed(arity) = word.stack_inputs else {
                continue;
            };
            for offset in 0..arity as usize {
                let _ = nil_rejection_error(word.name, offset);
                checked += 1;
            }
        }
        // The 13 fixed-arity `rejectNil` Words this table was built against
        // (`docs/dev/auditable-kernel-work-order-2026-09.md` Phase 1):
        // LENGTH, REVERSE, CHARS, JOIN, TRIM, EXEC, PROBE, DEL (1 operand
        // each) and RANDOM, CONCAT, TAKE, TOKENIZE, DEF (2 each) — 8 + 10.
        assert_eq!(
            checked, 18,
            "the set of fixed-arity rejectNil Words changed; update nil_rejection_error"
        );
    }
}
