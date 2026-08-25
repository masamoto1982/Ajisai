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
//! one: `Interpreter::collect_bracketed_with_depth` (`vector_literal.rs`)
//! captures it as a genuine nested Vector element (a `{ }`-spelled literal
//! builds the identical value a `[ ]`-spelled one would, since the
//! CodeBlock/Vector unification — docs/dev/type-unification-work-order-
//! 2026-08.md), but that element is inert data until something later
//! extracts and `EXEC`s it — building the vector does not run it. Measured:
//! `[ { PRINT } { 1 } ]` evaluates to `[ [ PRINT ] [ 1 ] ]`, a vector holding
//! two Vectors, neither of which has run (`COLLECT` reaches a *running*
//! quotation the same way, by gathering already-evaluated stack *values* and
//! later `GET`+`EXEC`ing one — a wholly different, non-literal path this
//! module does not need to special-case: the `{ }` there is written outside
//! any `[ ]`, at vector depth 0, and widens normally). So a Symbol lexically
//! inside an unclosed `[` never executes at that point in the source,
//! whatever punctuation surrounds it: either it is itself promoted to a
//! `Value::Symbol`, or it sits inertly inside a captured nested Vector
//! element — and the widen gate only ever needs the one fact
//! `collect_vector` itself keys off, "are we inside an unclosed `[`", to
//! suppress it. A bracket-kind stack would answer a different, unneeded
//! question (whether the *immediately enclosing* delimiter is `{` or `[`)
//! and would undercount vector depth on the way back out, so a *later*,
//! genuinely top-level Symbol after the vector closes could inherit a stale
//! bracket kind. The flat counter has neither failure mode.

use crate::types::Token;

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
