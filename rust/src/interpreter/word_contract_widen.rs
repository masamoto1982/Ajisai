//! What a resolved dependency contributes to the accumulator during contract
//! inference (`word_contract.rs`'s widen step) — two independent decisions,
//! both about the *acc-relevant* axes (purity/effects/capabilities/
//! determinism/order/nil/confidence/gaps) only. `flow`/`space`/`cost` are
//! unaffected by either and keep reading `dep_contract` directly.
//!
//! # Vector depth: a Symbol inside `[ ... ]` is a spelling, not a call
//!
//! `[ ... ]` is a data literal; `{ ... }` is a code literal whose interior
//! runs only when something later executes it. A bare Symbol written inside
//! a vector literal is desugared to its own name as a string —
//! `[ 'a' PRINT 'b' ]` *is* `[ 'a' 'PRINT' 'b' ]`, PRINT never resolves or
//! runs — so widening the accumulator with it was a false `error`: a body
//! that never prints inferred `effectful` against a correct `pure`
//! declaration, the same character of bug §1 of `docs/dev/
//! competitive-advantage-round2-2026-08.md` found on the *flow* axis.
//!
//! The gate is a **flat vector-nesting counter that ignores `{`/`}`
//! entirely** — not a bracket-kind stack that would treat a `{ }` opened
//! while already inside `[ ]` as re-entering "code." It is tempting to
//! reach for a stack (an inner `{ }` genuinely does mean code, once outside
//! any vector — see the `{ { 2 MUL } MAP }` case below), but a `{`/`}` found
//! *while a Vector literal is already being collected* still does not need
//! one: `Interpreter::collect_vector_with_depth` (`vector_literal.rs`)
//! captures it as a genuine `CodeBlock` element (its own
//! `Token::BlockStart`/`Token::BlockEnd` arm mirrors the top-level capture in
//! `execution_loop.rs`), but that element is inert data until something
//! later extracts and `EXEC`s it — building the vector does not run it.
//! Measured: `[ { PRINT } { 1 } ]` evaluates to `[ { PRINT } { 1 } ]`, a
//! vector holding two `CodeBlock`s, neither of which has run (`COLLECT`
//! reaches a *running* quotation the same way, by gathering already-evaluated
//! stack *values* and later `GET`+`EXEC`ing one — a wholly different,
//! non-literal path this module does not need to special-case: the `{ }`
//! there is written outside any `[ ]`, at vector depth 0, and widens
//! normally). So a Symbol lexically inside an unclosed `[` never executes at
//! that point in the source, whatever punctuation surrounds it: either it is
//! bare and denotes its own name, or it sits inertly inside a captured
//! `CodeBlock` element — and the widen gate only ever needs the one fact
//! `collect_vector` itself keys of, "are we inside an unclosed `[`", to
//! suppress it. A bracket-kind stack would answer a different, unneeded
//! question (whether the *immediately enclosing* delimiter is `{` or `[`)
//! and would undercount vector depth on the way back out, so a *later*,
//! genuinely top-level Symbol after the vector closes could inherit a stale
//! bracket kind. The flat counter has neither failure mode.
//!
//! # `REFLECT`: the sole crossing back from data to code
//!
//! Every *other* way a body can reach a `CodeBlock` value is visible here
//! regardless of provenance — a literal `{ }` outside any vector is scanned
//! unconditionally, and a value produced by calling another resolved word
//! already carries that word's own (over-)approximated effects. But
//! `REFLECT` can turn a `Vector` whose elements are `String` tokens into a
//! `CodeBlock` whose Word names were never `Symbol` tokens this walk could
//! see. Trusting `REFLECT`'s own registry contract — genuinely
//! `pure`/`nil-free`/deterministic, for the `CodeBlock`→data direction — for
//! *what the reflected value does once something runs it* let a body that
//! prints through `REFLECT EXEC` infer as `pure`/`complete`: not a false
//! `error` but a false *verified*, which is worse, since `check --contract`
//! exists precisely so a caller can trust a `pure` declaration without
//! running the code. See `contract_gap.rs`'s `OpaqueReflection` doc comment.

use crate::agent::contract_gap::GapCode;
use crate::kernel::generated::{generated_word, WordId};
use crate::types::Token;

use super::word_contract::{
    ContractConfidence, ContractDeterminism, ContractPurity, NilBehavior, OrderSensitivity,
    WordContract,
};

/// How many `[` are open and not yet closed. `{`/`}` do not change this —
/// see the module doc comment for why that is the *correct* reading of
/// `collect_vector_with_depth`, not a simplification of it.
#[derive(Default)]
pub(super) struct VectorScope {
    depth: u32,
}

impl VectorScope {
    pub(super) fn new() -> Self {
        VectorScope::default()
    }

    pub(super) fn feed_structural(&mut self, token: &Token) {
        match token {
            Token::VectorStart => self.depth = self.depth.saturating_add(1),
            // A stray close past zero is a body the tokenizer already
            // rejects as unbalanced elsewhere; saturating is a safe no-op
            // rather than a panic.
            Token::VectorEnd => self.depth = self.depth.saturating_sub(1),
            _ => {}
        }
    }

    /// True while at least one `[` is open: the current Symbol is
    /// vector-literal content, never resolved or called.
    pub(super) fn in_vector_literal(&self) -> bool {
        self.depth > 0
    }
}

/// Whether a canonical Word name is `REFLECT`. Read through the generated
/// registry rather than compared as a string, mirroring
/// `word_contract_flow::is_keep_modifier`.
pub(super) fn is_reflect(name: &str) -> bool {
    matches!(
        generated_word(name).map(|word| word.id),
        Some(WordId::Reflect)
    )
}

/// The acc-relevant contribution `REFLECT` widens the accumulator with,
/// instead of its own optimistic registry contract. `flow`/`space`/`cost`
/// are carried over unchanged from `dep_contract` — those axes are sound
/// regardless of which direction `REFLECT` ran — only the axes that would
/// otherwise claim knowledge of what the reflected value *does* fall back to
/// the conservative ceiling `WordContract::conservative()` uses elsewhere.
pub(super) fn opaque_reflection_contract(dep_contract: &WordContract) -> WordContract {
    WordContract {
        purity: ContractPurity::Effectful,
        effects: vec!["reflection".to_string()],
        determinism: ContractDeterminism::NonDeterministic,
        order_sensitivity: OrderSensitivity::OrderSensitive,
        nil_behavior: NilBehavior::MayCreate,
        confidence: ContractConfidence::Conservative,
        gaps: vec![GapCode::OpaqueReflection],
        ..dep_contract.clone()
    }
}
