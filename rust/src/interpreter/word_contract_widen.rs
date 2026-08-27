//! What a resolved dependency contributes to the accumulator during contract
//! inference (`word_contract.rs`'s widen step) — two independent decisions,
//! both about the *acc-relevant* axes (purity/effects/capabilities/
//! determinism/order/nil/confidence/gaps) only. `flow`/`space`/`cost` are
//! unaffected by either and keep reading `dep_contract` directly.
//!
//! # A Symbol inside `[ ... ]` may or may not be a call
//!
//! Before the CodeBlock/Vector unification (`docs/dev/type-unification-
//! work-order-2026-08.md`) and `{ }`'s later retirement, bracket spelling
//! told data from code directly: `[ ... ]` never ran, `{ ... }` always could.
//! `[ ]` is now the only bracket, used for both, so the question this module
//! answers — "does the Symbol at this position ever actually run?" — can no
//! longer be read off which character opened the group. It is still
//! answerable, from the same fixed-position-operand convention `COND` was
//! redesigned to share with `MAP`/`FILTER`/`FOLD`/`ANY`/`ALL`
//! (`compiled_plan.rs`'s `lower_cond_dispatch` doc comment): a `[ ... ]`
//! immediately followed by one of those Words (or `EXEC`/`PROBE`) *is* that
//! Word's code operand, and that Word will run it. Any other `[ ... ]` is
//! inert data: `[ 'a' PRINT 'b' ]` *is* `[ 'a' 'PRINT' 'b' ]`, PRINT never
//! resolves or runs, so widening the accumulator with it would be a false
//! `error` — a body that never prints inferred `effectful` against a correct
//! `pure` declaration, the same character of bug §1 of `docs/dev/
//! competitive-advantage-round2-2026-08.md` found on the *flow* axis.
//!
//! # Classification is top-down, not per-bracket
//!
//! Whether a `[ ... ]` is a code operand is decided once, from its own
//! enclosing position, and then applies to everything nested inside it:
//! once a group is inert data, nothing written inside it ever runs either,
//! however code-shaped it looks — `[ [ 2 MUL ] MAP ]` sitting inert as data
//! never runs `MAP` any more than it runs `MUL`. So a group nested inside a
//! `Data` group is always `Data` too, regardless of what follows its own
//! close; only a group whose *enclosing* context is the body's own top level
//! or another `Code` group gets to ask the "what follows my close" question
//! at all. Measured: `[ { PRINT } { 1 } ]` (pre-retirement spelling) built
//! `[ [ PRINT ] [ 1 ] ]`, a vector holding two Vectors, neither of which had
//! run — building the vector does not run it, whatever is nested inside.
//!
//! Arity is unaffected by any of this: a `[ ... ]` literal always pushes
//! exactly one value, whatever it contains and whatever consumes it
//! afterward (`word_contract_flow.rs`'s `FlowSim` already reads only vector
//! *depth*, not classification). Space/cost likewise keep treating a code
//! operand as opaque, attributed at the higher-order Word's own call site,
//! not unrolled here (`word_space.rs`, `word_cost.rs`) — only the widen step
//! needs the `Code`/`Data` distinction, to decide whether a Symbol nested in
//! a code operand still contributes its dependency's contract here, eagerly,
//! since no builtin's own registered contract describes what a *caller-
//! supplied* code operand does.

use crate::types::Token;

/// Which `[ ... ]`, if any, a token sits inside, and whether that vector is
/// inert data or a code operand about to run. See the module doc.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum LiteralContext {
    /// Not inside any `[ ... ]`: an ordinary body token.
    TopLevel,
    /// Inside a `[ ... ]` that is inert data, or nested inside one: never
    /// resolved or called, whatever it contains.
    Data,
    /// Inside a `[ ... ]` that is the fixed-position code operand of an
    /// immediately following higher-order Word: that Word will actually run
    /// this content.
    Code,
}

impl LiteralContext {
    /// True inside any `[ ... ]`, code or data — the fact `FlowSim`/
    /// `SpaceSim`/`CostSim` need for arity/space/cost (a literal always
    /// pushes one value and is opaque to those models regardless of
    /// classification).
    pub(super) fn in_vector_literal(self) -> bool {
        self != LiteralContext::TopLevel
    }
}

/// Canonical names of Words whose immediately preceding fixed-position
/// operand is code they actually execute — the convention `COND` was
/// redesigned to share with `MAP`/`FILTER`/`FOLD`/`ANY`/`ALL`
/// (`compiled_plan.rs`'s `lower_cond_dispatch`); `EXEC`/`PROBE` take their
/// sole operand the same way.
fn consumes_preceding_as_code(canonical_name: &str) -> bool {
    matches!(
        canonical_name,
        "MAP" | "FILTER" | "FOLD" | "ANY" | "ALL" | "EXEC" | "PROBE" | "COND"
    )
}

/// The first Symbol at or after `from`, skipping `LineBreak`s — `None` if the
/// body ends first or a non-Symbol token comes first (a code-consuming Word
/// is always named directly; nothing else can be "what follows").
fn next_symbol_from(tokens: &[Token], from: usize) -> Option<&str> {
    let mut i = from;
    while let Some(Token::LineBreak) = tokens.get(i) {
        i += 1;
    }
    match tokens.get(i) {
        Some(Token::Symbol(s)) => Some(s),
        _ => None,
    }
}

/// Classify every token of one body line by which `[ ... ]`, if any, it sits
/// inside. Two passes: first find each `[`'s matching `]` (a plain stack
/// scan), then assign contexts top-down so a `Data` ancestor forces `Data`
/// all the way down, and only a group whose enclosing context still allows
/// execution looks at what follows its own close.
///
/// `COND`'s clauses operand needs one further wrinkle: it is one Vector of
/// clause Vectors (`[ [ guard | body ] [ guard | body ] ... ]`), and `COND`
/// runs every guard and the winning body — not "whatever the clause Vector's
/// own close is followed by" (nothing follows an array element
/// positionally; the consumption is `COND`'s own internal iteration, not a
/// following symbol). So a direct child of a Vector that is itself `COND`'s
/// operand is unconditionally `Code`, and only *its* children resume the
/// ordinary "what follows my close" rule (a clause body can itself contain
/// an ordinary `[ ... ] MAP`, classified normally from there).
pub(super) fn classify_vector_positions(tokens: &[Token]) -> Vec<LiteralContext> {
    let mut close_of: Vec<Option<usize>> = vec![None; tokens.len()];
    let mut open_stack: Vec<usize> = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        match t {
            Token::VectorStart => open_stack.push(i),
            Token::VectorEnd => {
                if let Some(open) = open_stack.pop() {
                    close_of[open] = Some(i);
                }
            }
            _ => {}
        }
    }

    let mut contexts = vec![LiteralContext::TopLevel; tokens.len()];
    let mut level_stack: Vec<LiteralContext> = Vec::new();
    // Parallel to `level_stack`: whether the level at that depth is COND's
    // own clauses wrapper, so its direct children skip the lookahead.
    let mut is_cond_wrapper: Vec<bool> = Vec::new();
    for (i, t) in tokens.iter().enumerate() {
        let enclosing = level_stack
            .last()
            .copied()
            .unwrap_or(LiteralContext::TopLevel);
        let enclosing_is_cond_wrapper = is_cond_wrapper.last().copied().unwrap_or(false);
        match t {
            Token::VectorStart => {
                contexts[i] = enclosing;
                let (this_level, this_is_cond_wrapper) = if enclosing == LiteralContext::Data {
                    (LiteralContext::Data, false)
                } else if enclosing_is_cond_wrapper {
                    (LiteralContext::Code, false)
                } else {
                    match close_of[i].and_then(|close| next_symbol_from(tokens, close + 1)) {
                        Some(name) => {
                            let canonical =
                                crate::core_word_aliases::canonicalize_core_word_name(name);
                            if canonical.as_ref() == "COND" {
                                (LiteralContext::Code, true)
                            } else if consumes_preceding_as_code(&canonical) {
                                (LiteralContext::Code, false)
                            } else {
                                (LiteralContext::Data, false)
                            }
                        }
                        None => (LiteralContext::Data, false),
                    }
                };
                level_stack.push(this_level);
                is_cond_wrapper.push(this_is_cond_wrapper);
            }
            Token::VectorEnd => {
                contexts[i] = level_stack.pop().unwrap_or(LiteralContext::TopLevel);
                is_cond_wrapper.pop();
            }
            _ => {
                contexts[i] = enclosing;
            }
        }
    }
    contexts
}
