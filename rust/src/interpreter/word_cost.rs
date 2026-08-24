//! Static time-cost inference (Phase 5 of `docs/dev/
//! competitive-advantage-work-order-2026-08.md`; design rationale in
//! `docs/dev/cost-contract-design.md`).
//!
//! Mirrors `word_space.rs`'s shape for a different resource: instead of
//! bounding a word's *materialization* as a function of input size, this
//! bounds its charged *cost* — `ResourceUsage`'s three counters
//! (`executionSteps` / `numericWork` / `collectionWork`) — the same way, on
//! the same class lattice, joined during the same single-pass token walk
//! `word_contract.rs` already performs.
//!
//! A dependency's class is refined against the operand provenance that walk
//! already computes, so a compile-time-literal operand collapses a class the
//! same way it does for space (`[ 0 10 ] RANGE`). The provenance is *read
//! from* `SpaceSim` (`word_space::OperandProfile`) rather than tracked a
//! second time here: one slot stack, two bounds. This is what upholds the
//! "never a false error" invariant `word_space.rs`'s module comment states —
//! see `refine_axis` for why taking a class un-refined broke it in both
//! directions.

use crate::kernel::generated::{generated_word_by_id, CostAxis, WordId};
use crate::types::Token;

use super::word_contract::WordContract;
use super::word_space::{OperandProfile, SpaceClass};

/// Growth class of a charged quantity as a function of a word's input.
///
/// Generated from spec/words.schema.json's own `class` enum, so the runtime
/// vocabulary cannot be narrower than the canonical one, and its declaration
/// order **is** the lattice order the join widens along. Identical in shape to
/// `word_space::SpaceClass`, which stays a separate type because the two model
/// different resources and Phase 5 does not generalize them into one
/// (`docs/dev/cost-contract-design.md` §2).
pub(crate) use crate::kernel::generated::CostClass;

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

/// The classification spec/words.json declares for a built-in, read back as
/// the inference lattice's own representation.
///
/// The classes themselves are canonical data now, not a table written here:
/// they live on `GeneratedWord::cost`, projected from spec/words.json by
/// `scripts/generate-word-registry.mjs`, so the class a caller reads from
/// `word_contract` over MCP and the class this inference walk joins are the
/// same fact rather than two copies that can drift.
///
/// How those declared classes were arrived at — evidence, not guesswork, where
/// evidence exists:
///
/// * `steps` (executionSteps): a builtin dispatch is one step, full stop,
///   *unless* the builtin itself evaluates a caller-supplied block a
///   data-dependent number of times. Verified directly: `[ 1 2 3 ] { 2 MUL }
///   MAP` charges 4 steps (1 for MAP, 3 for the block's own MUL, once per
///   element) against `1 2 ADD`'s 1. Every other builtin is a primitive with
///   no such internal call, so it is `const` with the exactness witness.
/// * `numeric` (numericWork): only the exact-arithmetic path charges this
///   meter at all (`arithmetic::charge_binary_schema`, called from nowhere
///   outside `arithmetic.rs`). `runtime_limits.rs`'s own module comment states
///   `ADD`/`SUB`/`MUL`/`DIV` are "priced limb×limb, including addition and
///   subtraction"; `SUM` folds with that same charged schema per element
///   (`add_values_metered`); `QUANTIZE` was measured non-zero on a minimal
///   case. Every other word measured zero on a representative input — a
///   plausible `const` bound, but not `exact`, since one representative
///   measurement is evidence rather than a proof across every bit width.
/// * `collection` (collectionWork): the element-processing words scale with
///   their operand's *size*, measured directly (`SORT`/`CONCAT`/`GET`/`TAKE`/
///   `UNIQUE`/`TALLY`/`REVERSE`/`PUT`/`INDEX-OF` all charge proportional to
///   element count on a representative input). The remaining collection-shaped
///   words carry the same `linear` bound without the `exact` claim, not having
///   been individually confirmed to attain it; `JOIN` repeats a separator
///   between every pair, so it is `superlinear` in the worst case. `RANGE` and
///   `FILL` are the value-driven materializers: a *numeric operand's value*
///   sets the materialized length, so the charge is unbounded as a function of
///   input **size** — measured, `[ 0 10 ] RANGE` charges 187 and
///   `[ 0 20000 ] RANGE` charges 340,017 against the same 2-element operand;
///   `[ 300 300 0 ] FILL` charges 1,530,000. A compile-time literal operand
///   pins the amount, which `CostSim::feed_word` refines back down to `const`.
///
/// Anywhere evidence is absent the class is a plausible upper bound — never
/// looser than the word's own shape implies — carrying `exact = false`: sound
/// by construction (`docs/dev/cost-contract-design.md` §6), never a source of
/// a false declaration `Error`.
pub(crate) fn builtin_cost(id: WordId) -> CostBound {
    let declared = generated_word_by_id(id).cost;
    let axis = |a: CostAxis| (a.class, a.exact);
    CostBound {
        steps: axis(declared.steps),
        numeric: axis(declared.numeric),
        collection: axis(declared.collection),
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
/// dependency.
pub(crate) struct DepCost {
    pub cost: CostBound,
    /// True when the dependency is a built-in, whose classes are functions of
    /// its operands, so a compile-time-literal operand refines the
    /// contribution down. A user word's cost may be *internal* (a loop over
    /// its own construction), so its class is never refined downward — the
    /// same split `word_space::DepSpace::operand_driven` draws.
    pub operand_driven: bool,
}

impl DepCost {
    /// The dependency's own inferred bound. Reads `contract.cost` for both
    /// kinds: `word_contract::static_word_contract` already stored exactly
    /// `builtin_cost_for(name)` there, so re-deriving it per token would be a
    /// redundant registry lookup.
    pub(crate) fn of(contract: &WordContract, operand_driven: bool) -> Self {
        DepCost {
            cost: contract.cost,
            operand_driven,
        }
    }
}

/// The operand-size lattice (`word_space::SpaceClass`) read as a cost class.
/// The two lattices are identical in shape by construction (`docs/dev/
/// cost-contract-design.md` §2); this states the correspondence in one place
/// rather than merging the types the design deliberately keeps apart.
fn size_as_cost_class(size: SpaceClass) -> CostClass {
    match size {
        SpaceClass::Const => CostClass::Const,
        SpaceClass::Linear => CostClass::Linear,
        SpaceClass::Superlinear => CostClass::Superlinear,
        SpaceClass::Unbounded => CostClass::Unbounded,
    }
}

/// `class` charged against an operand of size bound `size` — the mirror of
/// `word_space::apply_to_size`.
fn apply_to_size(class: CostClass, size: CostClass) -> CostClass {
    match class {
        CostClass::Const => CostClass::Const,
        CostClass::Linear => size,
        CostClass::Superlinear => match size {
            CostClass::Const => CostClass::Const,
            CostClass::Linear | CostClass::Superlinear => CostClass::Superlinear,
            CostClass::Unbounded => CostClass::Unbounded,
        },
        CostClass::Unbounded => CostClass::Unbounded,
    }
}

/// One axis of a dependency's bound, refined against what actually feeds it.
///
/// This is the step Phase 5 originally omitted, and omitting it was unsound in
/// *both* directions: a bound left un-refined stays a looser class while
/// keeping its `exact` witness, and "looser class + exact" is precisely what
/// licenses a declaration `Error` — so `{ 1 2 ADD }` (a 0-input word whose
/// charge is a fixed constant) reported a hard error against the true
/// declaration `cost numeric=const`. Refining here restores the invariant
/// `word_space` already held: a witness survives only when the operands are
/// genuinely traced and the class was not refined away beneath it.
fn refine_axis(
    axis: (CostClass, bool),
    operand_driven: bool,
    all_lit: bool,
    all_traced: bool,
    size: CostClass,
) -> (CostClass, bool) {
    let (class, tight) = axis;
    if !operand_driven {
        return (class, tight && all_traced);
    }
    match class {
        CostClass::Const => (CostClass::Const, true),
        // Every operand is a compile-time constant, so the charge this call
        // makes is a compile-time constant too, whatever the class would be
        // for an input-derived operand.
        _ if all_lit => (CostClass::Const, true),
        CostClass::Linear | CostClass::Superlinear => {
            let applied = apply_to_size(class, size);
            (applied, tight && all_traced && applied == class)
        }
        CostClass::Unbounded => (CostClass::Unbounded, tight && all_traced),
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

    /// A resolved dependency call, refined against the operand provenance the
    /// space model already computed for the same call (`operands`) rather than
    /// re-deriving it from a second copy of the slot stack.
    pub(crate) fn feed_word(&mut self, dep: &DepCost, operands: OperandProfile) {
        let (all_lit, all_traced, size) = match operands {
            // Attributed at the higher-order word that runs the block, not here.
            OperandProfile::Skipped => return,
            // Counted, but no refinement and no witness is justified.
            OperandProfile::Unknown => (false, false, CostClass::Unbounded),
            OperandProfile::Known {
                all_lit,
                all_traced,
                max_size,
            } => (all_lit, all_traced, size_as_cost_class(max_size)),
        };
        let refine = |axis| refine_axis(axis, dep.operand_driven, all_lit, all_traced, size);
        self.bound.join(CostBound {
            steps: refine(dep.cost.steps),
            numeric: refine(dep.cost.numeric),
            collection: refine(dep.cost.collection),
        });
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
