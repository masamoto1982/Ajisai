//! The helpers of contract inference that `word_contract.rs` calls but does
//! not itself hold (that file sits at the per-file line budget): the facet
//! vocabulary an inferred contract is stated in, the data-or-code
//! classification of a `[ ... ]` literal, the resolve-then-widen step for a
//! Symbol inside a code operand, and `CONTRACT`'s block-inference entry point.
//!
//! What a resolved dependency contributes to the accumulator during contract
//! inference (`word_contract.rs`'s widen step) — two independent decisions,
//! both about the *acc-relevant* axes (purity/effects/capabilities/
//! determinism/order/nil/confidence/gaps) only. `flow`/`space`/`cost` are
//! unaffected by either and keep reading `dep_contract` directly.
//!
//! # A Symbol inside `[ ... ]` may or may not be a call
//!
//! Before the CodeBlock/Vector unification (`docs/dev/type-unification-
//! work-order-2026-08.md`), bracket spelling told data from code directly:
//! `[ ... ]` never ran, `{ ... }` always could. `[ ]` is now the only bracket
//! code is written in, used for data as well (and `{ ... }` spells a Record,
//! which is never code), so the question this module
//! answers — "does the Symbol at this position ever actually run?" — can no
//! longer be read off which character opened the group. It is still
//! answerable, from the fixed-position-operand convention the higher-order
//! Words share: a `[ ... ]` immediately followed by one of
//! `MAP`/`FILTER`/`FOLD`/`SCAN` (or `EXEC`/`CONTRACT`) *is* that Word's
//! code operand, and that Word will run it. Any other `[ ... ]` is
//! inert data: `[ 'a' PRINT 'b' ]` *is* `[ 'a' 'PRINT' 'b' ]`, PRINT never
//! resolves or runs, so widening the accumulator with it would be a false
//! `error` — a body that never prints inferred `effectful` against a correct
//! `pure` declaration.
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

use std::collections::HashSet;
use std::sync::Arc;

use crate::agent::contract_gap::GapCode;
use crate::types::{Token, WordDefinition};

use super::word_contract::{static_word_contract, AccumulatedContract, WordContract};
use super::Interpreter;

// The facets of an inferred contract, in the registry's own vocabulary.
//
// Split out of `word_contract.rs` for the file-size budget; the types are
// re-exported from there.
// The inferred facets speak the registry's own vocabulary
// (`spec/words.schema.json`): a User Word's or a block's contract and a Core
// Word's are answered with the same keys and the same values, so a caller
// compares them without translating. Each enum is ordered tightest to
// loosest, and the derived `Ord` is the join a body's contract widens by.

/// `purity`: a block is never `conditional` — its body is known, and the
/// inference walks it — so only the two ends of the registry's scale occur.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContractPurity {
    Pure,
    Effectful,
}

impl ContractPurity {
    pub const fn as_spec_str(self) -> &'static str {
        match self {
            ContractPurity::Pure => "pure",
            ContractPurity::Effectful => "effectful",
        }
    }

    pub fn from_spec_str(s: &str) -> Option<Self> {
        [ContractPurity::Pure, ContractPurity::Effectful]
            .into_iter()
            .find(|p| p.as_spec_str() == s)
    }
}

/// `determinism`: what else, beyond the operands, decides the result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContractDeterminism {
    Deterministic,
    StateRelative,
    HostRelative,
}

impl ContractDeterminism {
    pub const fn as_spec_str(self) -> &'static str {
        match self {
            ContractDeterminism::Deterministic => "deterministic",
            ContractDeterminism::StateRelative => "stateRelative",
            ContractDeterminism::HostRelative => "hostRelative",
        }
    }

    pub fn from_spec_str(s: &str) -> Option<Self> {
        [
            ContractDeterminism::Deterministic,
            ContractDeterminism::StateRelative,
            ContractDeterminism::HostRelative,
        ]
        .into_iter()
        .find(|d| d.as_spec_str() == s)
    }
}

/// `partiality`: `projecting` when some call can answer a reasoned NIL of
/// its own, `partial` when one can raise on operands of the right kind, and
/// `total` otherwise — the registry's derivation, with `projecting` taking
/// precedence exactly as it does there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContractPartiality {
    Total,
    Partial,
    Projecting,
}

impl ContractPartiality {
    pub const fn as_spec_str(self) -> &'static str {
        match self {
            ContractPartiality::Total => "total",
            ContractPartiality::Partial => "partial",
            ContractPartiality::Projecting => "projecting",
        }
    }

    pub fn from_spec_str(s: &str) -> Option<Self> {
        [
            ContractPartiality::Total,
            ContractPartiality::Partial,
            ContractPartiality::Projecting,
        ]
        .into_iter()
        .find(|p| p.as_spec_str() == s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContractConfidence {
    Complete,
    Conservative,
}

impl ContractConfidence {
    pub const fn as_spec_str(self) -> &'static str {
        match self {
            ContractConfidence::Complete => "complete",
            ContractConfidence::Conservative => "conservative",
        }
    }
}

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
/// operand is code they actually execute — the higher-order Words, with
/// `EXEC`/`CONTRACT` taking their sole operand the same way.
///
/// `SELECT` is deliberately absent: its operands are values, not code. That
/// is the whole of the difference between it and the `COND` it replaced, and
/// it is why branching no longer needs a special case anywhere in this file.
fn consumes_preceding_as_code(canonical_name: &str) -> bool {
    matches!(
        canonical_name,
        "MAP" | "FILTER" | "FOLD" | "SCAN" | "EXEC" | "CONTRACT"
    )
}

/// Whether the Symbol at `idx`, named `canonical_name`, runs code this walk
/// never read: a Word that runs its operand as code (not `CONTRACT`, which
/// only reads it) whose operand is anything but the `[ ... ]` literal written
/// immediately before it. A Vector taken out of data (`[ [ [ 42 PRINT ] ] ]
/// 0 GET EXEC`), a bound name, or a dependency's result are all code the walk
/// saw only as inert data, so it cannot say what running them does.
pub(super) fn runs_unread_code(
    tokens: &[Token],
    contexts: &[LiteralContext],
    idx: usize,
    canonical_name: &str,
) -> bool {
    if !consumes_preceding_as_code(canonical_name) || canonical_name == "CONTRACT" {
        return false;
    }
    let read_literal = idx.checked_sub(1).is_some_and(|prev| {
        tokens[prev] == Token::VectorEnd && contexts[prev] == LiteralContext::Code
    });
    !read_literal
}

/// The Symbol at `from` — `None` if the
/// body ends first or a non-Symbol token comes first (a code-consuming Word
/// is always named directly; nothing else can be "what follows").
fn next_symbol_from(tokens: &[Token], from: usize) -> Option<&str> {
    match tokens.get(from) {
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
/// Every code operand in the vocabulary is now one `[ ... ]` a named Word
/// follows, so "what follows my close" decides every group with no exception
/// to carry. `COND` was the exception: its clauses were a Vector *of*
/// clause Vectors, each of which it ran without any symbol following it, so
/// a direct child of that wrapper had to be forced to `Code` against the
/// ordinary rule.
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
    for (i, t) in tokens.iter().enumerate() {
        let enclosing = level_stack
            .last()
            .copied()
            .unwrap_or(LiteralContext::TopLevel);
        match t {
            Token::VectorStart => {
                let this_level = if enclosing == LiteralContext::Data {
                    LiteralContext::Data
                } else {
                    match close_of[i].and_then(|close| next_symbol_from(tokens, close + 1)) {
                        Some(name)
                            if consumes_preceding_as_code(
                                &crate::word_name::canonical_word_name(name),
                            ) =>
                        {
                            LiteralContext::Code
                        }
                        _ => LiteralContext::Data,
                    }
                };
                contexts[i] = enclosing;
                level_stack.push(this_level);
            }
            Token::VectorEnd => {
                contexts[i] = level_stack.pop().unwrap_or(LiteralContext::TopLevel);
            }
            _ => {
                contexts[i] = enclosing;
            }
        }
    }
    contexts
}

// Resolves a `[ ... ]` code operand's Symbols into the enclosing Word's
// contract, split from `word_contract.rs` to keep that file under the
// per-file line budget (the file-size budget in docs/dev/specification-implementation-rules.md). The algorithm itself is unchanged: this is
// the same resolve-then-widen step `infer_word_contract_inner` already runs
// for an ordinary body-level dependency, applied instead to a Symbol found
// inside a literal that `word_contract_widen.rs` classified as
// `LiteralContext::Code`.
impl Interpreter {
    /// Resolve `symbol` and widen `acc` with its contract, for a Symbol found
    /// inside a `[ ... ]` classified `LiteralContext::Code` (`word_contract_
    /// widen.rs`) — a fixed-position code operand the enclosing call (`MAP`,
    /// `EXEC`, ...) will actually run, unlike an ordinary data literal. Only
    /// `acc` is touched: arity/space/cost already treated the whole literal
    /// as one opaque value (the caller's `flow`/`sim`/`cost_sim.feed_literal`
    /// calls), and stay attributed at that Word's own call site rather than
    /// unrolled here, exactly as for a ordinary body-level dependency's own
    /// internal cost.
    pub(crate) fn widen_with_code_operand_symbol(
        &mut self,
        symbol: &str,
        visiting: &mut HashSet<String>,
        acc: &mut AccumulatedContract,
        complete: &mut bool,
    ) {
        let canonical = crate::word_name::canonical_word_name(symbol);
        let Some((dep_name, dep_def)) = self.resolve_word_entry(&canonical) else {
            *complete = false;
            acc.note_unresolved_word();
            return;
        };
        let dep_contract = if dep_def.is_builtin {
            Arc::new(static_word_contract(&dep_name, &dep_def))
        } else if visiting.contains(dep_name.as_ref()) {
            *complete = false;
            acc.gaps.push(GapCode::RecursiveDependency);
            let mut placeholder =
                WordContract::conservative(self.contract_cache_key(&dep_name, &dep_def));
            placeholder.gaps.clear();
            Arc::new(placeholder)
        } else {
            match self.infer_word_contract_inner(&dep_name, &dep_def, visiting) {
                Some(contract) => contract,
                None => {
                    *complete = false;
                    acc.gaps.push(GapCode::DependencyUnknown);
                    return;
                }
            }
        };
        acc.widen_with(&dep_contract);
    }

    /// Widen `acc` for a code operand this walk never read
    /// (`word_contract_widen::runs_unread_code`): nothing is known about what
    /// it does, so the widening is the conservative contract's, and the
    /// inference is incomplete for a reason of its own.
    pub(crate) fn widen_with_unread_code_operand(
        &self,
        acc: &mut AccumulatedContract,
        complete: &mut bool,
    ) {
        *complete = false;
        let mut unknown = WordContract::conservative(super::word_contract::leaf_cache_key(
            "unread-code-operand".to_string(),
        ));
        unknown.gaps.clear();
        acc.widen_with(&unknown);
        acc.gaps.push(GapCode::UnmodelledControlFlow);
    }
}

// `CONTRACT`'s block-inference entry point, split from `word_contract.rs` to keep
// that file under the per-file line budget (the file-size budget in docs/dev/specification-implementation-rules.md). The algorithm itself is
// unchanged: this is a thin adapter that lets `infer_word_contract_inner`
// walk an anonymous CodeBlock's tokens the same way it already walks a
// named dictionary Word's body.
impl Interpreter {
    /// The same walk `infer_word_contract` runs for a named dictionary Word,
    /// run instead over an anonymous CodeBlock's own tokens. The block is
    /// wrapped in a throwaway `WordDefinition` that is never inserted into
    /// the dictionary — probing resolves the names the block calls but
    /// writes nothing back, matching `CONTRACT`'s declared purity.
    ///
    /// The synthetic definition's `registration_order` is freshly drawn from
    /// the interpreter's own counter (`next_registration_order`) on every
    /// call. That is not incidental: `contract_cache_key` falls back to
    /// `"unidentified:{name}:{registration_order}"` whenever `word_identity`
    /// has nothing to look up — true for every anonymous block, which is
    /// never named — and two different code blocks that happen to call the
    /// same dependencies would otherwise collide on the same cache key and
    /// silently return each other's inferred contract. A fresh order per
    /// call makes that collision impossible at the cost of never sharing the
    /// cache across probes, which is the correct trade for a Word whose
    /// input is, by construction, unnamed.
    pub(crate) fn infer_contract_for_block(&mut self, tokens: &[Token]) -> Arc<WordContract> {
        let def = Arc::new(WordDefinition {
            body: Arc::from(tokens),
            is_builtin: false,
            description: None,
            dependencies: HashSet::new(),
            text_references: HashSet::new(),
            registration_order: self.next_registration_order(),
            compiled_plan: None,
            generated: None,
        });
        let mut visiting = HashSet::new();
        self.infer_word_contract_inner("", &def, &mut visiting)
            .expect("a freshly synthesized WordDefinition always yields Some")
    }
}
