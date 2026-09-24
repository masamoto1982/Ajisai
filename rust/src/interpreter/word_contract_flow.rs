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
//! # Why `OR-NIL` and `|` give up rather than guess
//!
//! `OR-NIL` and `|` (the COND clause separator) select between paths whose
//! stack heights genuinely differ, so no single fixed arity describes them.
//! Measured: `2 3 1 0 DIV OR-NIL ADD` leaves one value (the NIL is dropped and
//! the fallback unit `ADD` runs), while `2 3 4 2 DIV OR-NIL ADD` leaves three
//! (the value stands and the fallback unit is skipped). The old walk ignored
//! both tokens outright and counted the fallback unit as an unconditional
//! push, so `1 0 DIV OR-NIL 9` inferred `( 0 -- 2 )` for a body that leaves
//! one value.
//!
//! `Dynamic` is the honest answer, but `Dynamic` alone still licenses a hard
//! error against a declared fixed arity. So the two cases are kept apart:
//! `dynamic` records a flow *derived* from a dependency's own `Dynamic` mass
//! contract (a proof, which may license an error), while `unmodelled` records
//! that this simulation gave up (a gap, which may only produce a note). The
//! same split `word_space` draws between a bound and its `exact` witness.

use super::word_contract::ContractFlow;
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
            // One depth over both literals: what matters to the flow is that
            // a literal's interior pushes nothing and its close pushes one
            // value, which is as true of `{ ... }` as of `[ ... ]`.
            Token::VectorStart | Token::RecordStart => self.vector_depth += 1,
            Token::VectorEnd | Token::RecordEnd => self.close(),
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

    /// A resolved dependency call: it consumes its operands and pushes its
    /// results.
    pub(crate) fn feed_word(&mut self, flow: &ContractFlow) {
        if self.in_literal() {
            return;
        }
        let ContractFlow::Fixed { consumes, produces } = flow else {
            self.dynamic = true;
            return;
        };
        if self.height < *consumes {
            self.required = self.required.saturating_add(consumes - self.height);
            self.height = 0;
        } else {
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

/// Names a `BIND` in the body made. The rest of the body reads each as one
/// value; looking it up in the dictionary instead would report a bound name as
/// an unresolved Word and give up on the arity (the lexicon-emergence pilot's
/// finding M-2).
pub(crate) type BoundNames = std::collections::HashSet<String>;

/// At a `BIND`, record the names it binds: the String just before it, or each
/// String of the Vector literal just before it (`[ 'A' 'B' ] BIND`).
pub(crate) fn note_bound_names(
    bound: &mut BoundNames,
    canonical: &str,
    tokens: &[Token],
    idx: usize,
) {
    if canonical != "BIND" {
        return;
    }
    match idx.checked_sub(1).map(|j| (j, &tokens[j])) {
        Some((_, Token::String(name))) => {
            bound.insert(name.to_uppercase());
        }
        Some((close, Token::VectorEnd)) => {
            let mut depth = 0usize;
            for token in tokens[..=close].iter().rev() {
                match token {
                    Token::VectorEnd => depth += 1,
                    Token::VectorStart => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    Token::String(name) if depth == 1 => {
                        bound.insert(name.to_uppercase());
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}
