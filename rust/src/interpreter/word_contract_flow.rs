//! Stack-flow (arity) simulation for contract inference.
//!
//! Split out of `word_contract.rs`, where the same job was done by a
//! `FlowAccumulator` that read *every* token of a word body as if it stood at
//! the top level. That reading was unsound, because `[ ... ]` is a **literal
//! whose interior is not code**: it is evaluated only if some higher-order
//! Word later runs it as the Vector it built, never at the point it is
//! written — `[ 1 2 ADD ]` is the three-element vector `[ 1/1 2/1 'ADD' ]`,
//! not a call to `ADD`.
//!
//! A literal pushes exactly one value and consumes nothing, whatever it
//! contains. The old walk instead counted each interior `Number`/`String` as
//! a push and *applied* each interior `Symbol`'s arity, so `[ [ 1 2 ] ]`
//! `'PAIR' DEF` inferred `( 0 -- 2 )` for a word that produces one vector,
//! and `[ [ 2 MUL ] MAP ] 'DOUBLE-ALL' DEF` inferred `( 2 -- 1 )` for a word
//! whose true arity is `( 1 -- 1 )`. Those inferences are reported at
//! `ContractConfidence::Complete`, so a *correct* `#:contract` declaration was
//! rejected as a proven violation — a false `error`, which
//! `LANG.CONTRACT.CHECK` forbids and which `word_space.rs`'s module comment
//! states as the module-wide "never a false error" invariant.
//!
//! This simulation therefore tracks the two literal depths the way `SpaceSim`
//! already does, and updates the height only at depth zero, where the tokens
//! really are code.
//!
//! # Why `^` and `|` give up rather than guess
//!
//! `^` (OR-NIL) and `|` (the COND clause separator) select between paths whose
//! stack heights genuinely differ, so no single fixed arity describes them.
//! Measured: `2 3 1 0 DIV ^ ADD` leaves one value (the NIL is dropped and the
//! fallback unit `ADD` runs), while `2 3 4 2 DIV ^ ADD` leaves three (the
//! value stands and the fallback unit is skipped). The old walk ignored both
//! tokens outright and counted the fallback unit as an unconditional push, so
//! `1 0 DIV ^ 9` inferred `( 0 -- 2 )` for a body that leaves one value.
//!
//! `Dynamic` is the honest answer, but `Dynamic` alone still licenses a hard
//! error against a declared fixed arity. So the two cases are kept apart:
//! `dynamic` records a flow *derived* from a dependency's own `Dynamic` mass
//! contract (a proof, which may license an error), while `unmodelled` records
//! that this simulation gave up (a gap, which may only produce a note). The
//! same split `word_space` draws between a bound and its `exact` witness.
//!
//! # `KEEP` is a modifier, not a Word with an arity
//!
//! `KEEP` carries the registry arity `( 0 -- 0 )`, because it is not a Word
//! that moves values: it prefixes the next Word and makes that Word read its
//! operands without consuming them (`LANG.MODIFIERS.CONSUMPTION`). Applying
//! its `( 0 -- 0 )` and then the next Word's arity unchanged describes
//! neither. Measured: `2 3 KEEP ADD` leaves `2 3 5`, so a body of
//! `KEEP ADD` is `( 2 -- 3 )` — while the old walk inferred `( 2 -- 1 )` and rejected the
//! true declaration as a violation.
//!
//! The modifier is therefore held as a pending flag and applied to the next
//! Word as `( c -- c + p )`. The flag survives literals, line breaks and
//! delimiters, and a second `KEEP` is idempotent — all three measured
//! (`2 3 KEEP 4 ADD` leaves `2 3 4 7`; `2 3 KEEP\nADD` and
//! `2 3 KEEP KEEP ADD` both leave `2 3 5`). A flag still pending at the end
//! of a body is a no-op there too (`1 KEEP` leaves `1`).

use super::word_contract::ContractFlow;
use crate::kernel::generated::{generated_word, WordId};
use crate::types::Token;

/// Execution-free stack-flow simulation over a word body's token stream, fed
/// by the contract-inference walk alongside `SpaceSim` and `CostSim`.
#[derive(Default)]
pub(crate) struct FlowSim {
    /// The flow is data-dependent because a dependency's own mass contract is
    /// `Dynamic`. This is a *derived* fact, not a gap: it may license a
    /// declaration error.
    dynamic: bool,
    /// This simulation could not model the body (a control directive whose
    /// paths differ in height, or an unbalanced delimiter). Reported as a gap
    /// so the declaration check can only ever produce a note.
    unmodelled: bool,
    /// Values the body needs beneath what it pushed for itself.
    required: u16,
    /// Simulated stack height contributed by the body so far.
    height: u16,
    vector_depth: u32,
    /// A `KEEP` has been read and applies to the next Word.
    pending_keep: bool,
}

impl FlowSim {
    pub(crate) fn new() -> Self {
        FlowSim::default()
    }

    /// Inside a `[ ... ]`, where tokens are literal content
    /// rather than code and contribute nothing to the height.
    fn in_literal(&self) -> bool {
        self.vector_depth > 0
    }

    fn push_value(&mut self) {
        self.height = self.height.saturating_add(1);
    }

    /// A `Number`/`String` literal token.
    pub(crate) fn feed_literal(&mut self) {
        if !self.in_literal() {
            self.push_value();
        }
    }

    /// A structural token. The closing delimiter of a *top-level* literal is
    /// where its single value is pushed — not the opening one, so a body that
    /// never closes it pushes nothing and is caught by `finish`.
    pub(crate) fn feed_structural(&mut self, token: &Token) {
        match token {
            Token::VectorStart => self.vector_depth += 1,
            Token::VectorEnd => self.close(),
            // Both select between paths of differing height; see the module
            // comment. Inside a literal they are content, not control.
            Token::NilCoalesce | Token::CondClauseSep => {
                if !self.in_literal() {
                    self.unmodelled = true;
                }
            }
            Token::LineBreak | Token::Number(_) | Token::String(_) | Token::Symbol(_) => {}
        }
    }

    /// Close a `]`. A delimiter that closes nothing means the body is
    /// unbalanced, so every height from here on is a guess: give up rather
    /// than resynchronize onto an invented depth.
    fn close(&mut self) {
        if self.vector_depth == 0 {
            self.unmodelled = true;
            return;
        }
        self.vector_depth -= 1;
        if !self.in_literal() {
            self.push_value();
        }
    }

    /// A resolved dependency call, by canonical name so the one modifier can
    /// be recognized as a modifier rather than applied as an arity.
    pub(crate) fn feed_word(&mut self, name: &str, flow: &ContractFlow) {
        if self.in_literal() {
            return;
        }
        if is_keep_modifier(name) {
            self.pending_keep = true;
            return;
        }
        let keep = std::mem::take(&mut self.pending_keep);
        let ContractFlow::Fixed { consumes, produces } = flow else {
            self.dynamic = true;
            return;
        };
        if self.height < *consumes {
            self.required = self.required.saturating_add(consumes - self.height);
            // Under `KEEP` the operands stay: the ones just charged to
            // `required` are now sitting beneath the result, not gone.
            self.height = if keep { *consumes } else { 0 };
        } else if !keep {
            self.height -= consumes;
        }
        self.height = self.height.saturating_add(*produces);
    }

    /// A symbol that did not resolve, or a dependency whose own inference
    /// could not complete. Inside a literal the symbol is content, so the
    /// height is unaffected; the caller still records its own gap either way.
    pub(crate) fn go_dynamic(&mut self) {
        if !self.in_literal() {
            self.dynamic = true;
        }
    }

    /// The caller stopped feeding this line mid-way, so the depths no longer
    /// describe the source: resynchronize for whatever follows, exactly as
    /// `SpaceSim::abandon_line` does.
    pub(crate) fn abandon_line(&mut self) {
        self.dynamic = true;
        self.vector_depth = 0;
        self.pending_keep = false;
    }

    /// The inferred flow, plus whether the simulation gave up reaching it. A
    /// literal left open at the end of the body is unbalanced for the same
    /// reason `close` treats a stray delimiter as one.
    pub(crate) fn finish(self) -> (ContractFlow, bool) {
        let unmodelled = self.unmodelled || self.in_literal();
        let flow = if self.dynamic || unmodelled {
            ContractFlow::Dynamic
        } else {
            ContractFlow::Fixed {
                consumes: self.required,
                produces: self.height,
            }
        };
        (flow, unmodelled)
    }
}

/// Whether a canonical Word name is the consumption modifier. Read through the
/// generated registry rather than compared as a string, the way
/// `word_space::builtin_space_for` reads its own classification.
fn is_keep_modifier(name: &str) -> bool {
    matches!(
        generated_word(name).map(|word| word.id),
        Some(WordId::SetConsumptionKeep)
    )
}
