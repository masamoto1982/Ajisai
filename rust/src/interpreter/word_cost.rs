//! Static time-cost inference (Phase 5 of `docs/dev/
//! competitive-advantage-work-order-2026-08.md`; design rationale in
//! `docs/dev/cost-contract-design.md`).
//!
//! Mirrors `word_space.rs`'s shape for a different resource: instead of
//! bounding a word's *materialization* as a function of input size, this
//! bounds its charged *cost* — `ResourceUsage`'s three counters
//! (`executionSteps` / `numericWork` / `collectionWork`) — the same way, on
//! the same class lattice, joined during the same single-pass token walk
//! `word_contract.rs` already performs. Unlike `word_space.rs`, this module
//! does not refine a bound from an operand's literal-vs-input provenance
//! (`docs/dev/cost-contract-design.md` §6): every dependency's class is
//! taken as declared, which can only make a bound *looser*, never produce a
//! false `Error` — the same invariant `word_space.rs`'s module comment
//! states, inherited here by construction rather than by re-deriving it.

use crate::kernel::generated::WordId;
use crate::types::Token;

use super::word_contract::WordContract;

/// Growth class of a charged quantity as a function of a word's input.
/// Identical lattice to `word_space::SpaceClass`; kept as its own type
/// because the two model different resources and Phase 5 does not
/// generalize the two into one (`docs/dev/cost-contract-design.md` §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum CostClass {
    Const,
    Linear,
    Superlinear,
    Unbounded,
}

impl CostClass {
    pub(crate) fn from_str(s: &str) -> Option<CostClass> {
        match s {
            "const" => Some(CostClass::Const),
            "linear" => Some(CostClass::Linear),
            "superlinear" => Some(CostClass::Superlinear),
            "unbounded" => Some(CostClass::Unbounded),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            CostClass::Const => "const",
            CostClass::Linear => "linear",
            CostClass::Superlinear => "superlinear",
            CostClass::Unbounded => "unbounded",
        }
    }
}

/// A joined bound on all three declarable axes (`docs/dev/
/// cost-contract-design.md` §1). Each axis is `(class, exact)`: `class` is a
/// sound upper bound, `exact` records that some contribution *provably
/// attains* it — licensing a declaration `Error` rather than a `Note` (§3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CostBound {
    pub steps: (CostClass, bool),
    pub numeric: (CostClass, bool),
    pub collection: (CostClass, bool),
}

impl CostBound {
    pub(crate) const IDENTITY: CostBound = CostBound {
        steps: (CostClass::Const, true),
        numeric: (CostClass::Const, true),
        collection: (CostClass::Const, true),
    };
    pub(crate) const CONSERVATIVE: CostBound = CostBound {
        steps: (CostClass::Unbounded, false),
        numeric: (CostClass::Unbounded, false),
        collection: (CostClass::Unbounded, false),
    };

    /// Monotone per-axis join, identical rule to `SpaceBound::join`: the
    /// class widens to the max; at an equal class, `exact` is the OR of the
    /// two (one attaining contribution proves the join attains it).
    pub(crate) fn join(&mut self, other: CostBound) {
        Self::join_axis(&mut self.steps, other.steps);
        Self::join_axis(&mut self.numeric, other.numeric);
        Self::join_axis(&mut self.collection, other.collection);
    }

    fn join_axis(axis: &mut (CostClass, bool), other: (CostClass, bool)) {
        if other.0 > axis.0 {
            *axis = other;
        } else if other.0 == axis.0 {
            axis.1 |= other.1;
        }
    }
}

/// Per-builtin classification, all three axes together. Grouped and
/// commented by *why*, mirroring `word_space::builtin_space`'s style.
///
/// Evidence, not guesswork, where evidence exists: `runtime_limits.rs`'s own
/// module comment states `Add`/`Sub`/`Mul`/`Div` are "priced limb×limb,
/// including addition and subtraction"; `SUM`'s metered fold
/// (`arithmetic::add_values_metered`) charges the identical schema per
/// element; direct measurement (`ajisai run --json`, `resourceUsage`) pins
/// the collection-processing words' `collectionWork` scaling and confirms
/// comparisons/`SQRT`/`MOD`/`FLOOR`/`ROUND`/`ABS`/`NEG`/`MIN`/`MAX` charge no
/// `numericWork` at all for a representative input. Elsewhere — any word not
/// individually measured or documented — the class is a plausible upper
/// bound (never a guess looser than what the word's own shape implies) with
/// `exact = false`: sound by construction (`docs/dev/cost-contract-design.md`
/// §6), never a source of a false declaration `Error`.
pub(crate) fn builtin_cost(id: WordId) -> CostBound {
    use CostClass::*;
    use WordId::*;

    // `steps` (executionSteps): a builtin dispatch is one step, full stop,
    // *unless* the builtin itself evaluates a caller-supplied block a
    // data-dependent number of times. Verified directly:
    // `[ 1 2 3 ] { 2 MUL } MAP` charges 4 steps (1 for MAP, 3 for the block's
    // own MUL, once per element) against `1 2 ADD`'s 1. Every other builtin
    // is a primitive with no such internal call, so it is `(Const, true)`
    // regardless of which axis is being read below.
    let steps = match id {
        Map | Filter | Fold | Any | All | Exec | Cond => (Unbounded, false),
        _ => (Const, true),
    };

    // `numeric` (numericWork): only the exact-arithmetic path charges this
    // meter at all (`arithmetic::charge_binary_schema`, called from nowhere
    // outside `arithmetic.rs`). `Add`/`Sub`/`Mul`/`Div` scale with operand
    // bit width (documented, and the meter's whole reason to exist);
    // `Sum` folds with that same charged schema per element
    // (`add_values_metered`); `Quantize` was measured non-zero on a minimal
    // case. Every other word measured zero on a representative input; that
    // is a plausible bound (none of them perform unbounded internal
    // arithmetic), so it earns `Const`, but not `exact` — a single
    // representative measurement is evidence, not a proof across every
    // bit width.
    let numeric = match id {
        Add | Sub | Mul | Div | Sum => (Linear, true),
        Quantize => (Linear, false),
        Map | Filter | Fold | Any | All | Exec | Cond => (Unbounded, false),
        _ => (Const, false),
    };

    // `collection` (collectionWork): the element-processing words scale with
    // their operand's size, measured directly (`SORT`/`CONCAT`/`GET`/
    // `RANGE`/`TAKE`/`UNIQUE`/`TALLY`/`REVERSE`/`PUT`/`INDEX-OF` all charge
    // collectionWork proportional to element count on a representative
    // input). The remaining collection-shaped words (`FILL`, `ZIP`, `GROUP`,
    // `JOIN`, `CHARS`, `TOKENIZE`, `TRIM`, `STR`, `NUM`, `BIND`, `DEF`,
    // `PRINT`) are given the same plausible `Linear` bound without the
    // `exact` claim, since they were not all individually confirmed to
    // *attain* it. `JOIN` can repeat a separator between every pair, which
    // is `Superlinear` rather than `Linear` in the worst case.
    let collection = match id {
        Concat | Reverse | Take | Collect | Range | Fill | Sort | Order | Unique | Tally | Zip
        | Put | Group | IndexOf | Get => (Linear, true),
        Join => (Superlinear, false),
        // `Reflect` converts a CodeBlock token sequence to/from a Vector, a
        // size-dependent conversion plausibly linear in body length —
        // measured for none of these, so `exact` stays false throughout.
        Chars | Tokenize | Trim | Str | Num | Bind | Def | Print | Reflect => (Linear, false),
        Map | Filter | Fold | Any | All | Exec | Cond => (Unbounded, false),
        // Constants, comparisons, logic, and exact arithmetic touch no
        // collection by construction — confirmed for the arithmetic and
        // comparison words directly (measured `collectionWork: 0`) —
        // earning the stronger exact claim; everything else not
        // individually reasoned through here falls to the plain,
        // non-exact `Const` default below.
        True | False | Nil | NilCheck | NilReason | Eq | Lt | Le | Gt | Gte | Neq | And | Or
        | Not | Add | Sub | Mul | Div | Mod | Floor | Round | Quantize | Abs | Neg | Min | Max
        | Sqrt | Length => (Const, true),
        _ => (Const, false),
    };

    CostBound {
        steps,
        numeric,
        collection,
    }
}

/// Cost classification for a resolved built-in word, by canonical name. A
/// name the registry does not know is conservatively unclassified — the same
/// fallback `word_space::builtin_space_for` uses.
pub(crate) fn builtin_cost_for(name: &str) -> CostBound {
    match crate::kernel::generated::generated_word(name) {
        Some(word) => builtin_cost(word.id),
        None => CostBound::CONSERVATIVE,
    }
}

/// The per-word contribution facts `CostSim` needs about a resolved
/// dependency — deliberately just the joined bound (`docs/dev/
/// cost-contract-design.md` §6: no operand-literal refinement in this
/// phase, unlike `word_space::DepSpace`).
pub(crate) struct DepCost {
    pub cost: CostBound,
}

impl DepCost {
    pub(crate) fn of_builtin(name: &str) -> Self {
        DepCost {
            cost: builtin_cost_for(name),
        }
    }

    pub(crate) fn of_user_word(contract: &WordContract) -> Self {
        DepCost {
            cost: contract.cost,
        }
    }
}

/// Execution-free cost simulation over a word body's token stream, fed from
/// the same `word_contract.rs` walk `SpaceSim` rides along with. A block's
/// interior is opaque here exactly as it is to `SpaceSim`: its cost is
/// attributed only where a higher-order word (already `Unbounded`) evaluates
/// it, never unrolled at the `DEF` site.
pub(crate) struct CostSim {
    bound: CostBound,
    block_depth: u32,
}

impl CostSim {
    pub(crate) fn new() -> Self {
        CostSim {
            bound: CostBound::IDENTITY,
            block_depth: 0,
        }
    }

    /// A `Number`/`String` literal: pushes a value, calls no word, costs
    /// nothing on any axis.
    pub(crate) fn feed_literal(&mut self) {}

    pub(crate) fn feed_structural(&mut self, token: &Token) {
        match token {
            Token::BlockStart => self.block_depth += 1,
            Token::BlockEnd => self.block_depth = self.block_depth.saturating_sub(1),
            // `^` and `|` branch along a path this linear walk cannot
            // follow, exactly as `SpaceSim::feed_structural` treats them.
            Token::NilCoalesce | Token::CondClauseSep if self.block_depth == 0 => {
                self.bound.join(CostBound::CONSERVATIVE);
            }
            _ => {}
        }
    }

    pub(crate) fn feed_unresolved(&mut self) {
        if self.block_depth > 0 {
            return;
        }
        self.bound.join(CostBound::CONSERVATIVE);
    }

    pub(crate) fn feed_word(&mut self, dep: &DepCost) {
        if self.block_depth > 0 {
            return;
        }
        self.bound.join(dep.cost);
    }

    /// The caller stopped feeding this line mid-way (a dependency could not
    /// be inferred): pin to the conservative top, same as `SpaceSim`.
    pub(crate) fn abandon_line(&mut self) {
        self.bound.join(CostBound::CONSERVATIVE);
        self.block_depth = 0;
    }

    pub(crate) fn finish(self) -> CostBound {
        self.bound
    }
}
