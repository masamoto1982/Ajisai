//! Static space-growth inference (Phase 2.2 of the structural-memory-safety
//! roadmap; see `docs/dev/space-contract-design.md`).
//!
//! Assigns every built-in a coarse growth class and infers a user word's class
//! by joining its body's *applied* dependency contributions during the same
//! execution-free token walk the contract inference already performs
//! (`word_contract.rs`). The domain is deliberately provenance-aware: a
//! materializer whose operand is a compile-time literal contributes `const`
//! (`[ 0 10 ] RANGE` is input-independent), while the same word fed an input
//! value is provably `unbounded` (`X RANGE` materializes a length set by the
//! *value* of `X`). Everything the simulation cannot prove degrades to a sound
//! upper bound with `exact = false`, so the declaration checker can only raise
//! an `error` on a provable violation — the module-wide "never a false error"
//! invariant. `feed_word` hands that provenance to `word_cost::CostSim` too.

use crate::kernel::generated::WordId;
use crate::types::Token;

use super::word_contract::{
    ContractConfidence, ContractDeterminism, ContractFlow, ContractPurity, WordContract,
};

/// Growth class of a word's *extra materialization* as a function of its input.
/// Ordered tightest → loosest; the derived `Ord` is the widening order.
///
/// `Const`       — O(1) new nodes, independent of input size.
/// `Linear`      — O(n) in the total input size.
/// `Superlinear` — grows faster than input but still a function of it.
/// `Unbounded`   — materialization is set by a *value* (e.g. a numeric operand
///                 of `RANGE`/`FILL`), so no static bound over input size exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpaceClass {
    Const,
    Linear,
    Superlinear,
    Unbounded,
}

/// A joined space bound: `class` is a sound upper bound on the word's growth;
/// `exact` records that some contribution *provably attains* `class`, so a
/// declaration below it is a real violation (error), not merely unverifiable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpaceBound {
    pub class: SpaceClass,
    pub exact: bool,
}

impl SpaceBound {
    pub(crate) const IDENTITY: SpaceBound = SpaceBound {
        class: SpaceClass::Const,
        exact: true,
    };
    pub(crate) const CONSERVATIVE: SpaceBound = SpaceBound {
        class: SpaceClass::Unbounded,
        exact: false,
    };

    /// Monotone join: the class widens to the max; the join is exact when a
    /// contribution *at* the max is exact (it alone proves attainment — the
    /// contributions below the max cannot change the class).
    fn join(&mut self, other: SpaceBound) {
        if other.class > self.class {
            *self = other;
        } else if other.class == self.class {
            self.exact |= other.exact;
        }
    }
}

/// Authored space classification of a built-in: `(class, tight)`. `class` is a
/// sound *upper* bound on the word's extra materialization as a function of its
/// operands; `tight` asserts the class is *attained* on some worst-case operand,
/// which is what licenses an `error` (rather than a note) when the operand is
/// provably an input value. When in doubt a word is classified with a generous
/// class and `tight = false`, which can never produce a false error — only a
/// "cannot verify" note.
fn builtin_space(id: WordId) -> (SpaceClass, bool) {
    use SpaceClass::*;
    use WordId::*;
    match id {
        // Exact rational arithmetic: elementwise over vectors and digit growth
        // are both O(input); a vector operand attains the bound.
        Add | Sub | Mul | Div => (Linear, true),
        // Comparisons and logic may produce elementwise results; O(input),
        // not audited as tight.
        Eq | Lt | Le | Gt | Gte | Neq | And | Or | Not => (Linear, false),
        // Higher-order and dynamic-control words run caller-supplied bodies a
        // data-dependent number of times: no static bound.
        Map | Filter | Fold | Any | All => (Unbounded, false),
        Exec | Cond => (Unbounded, false),
        // Structure access/observation: shares persistent structure, O(1) new.
        // `Probe` walks the block's tokens once without evaluating them, so
        // its output scales with the block's own size rather than with
        // anything unbounded.
        Get | Length | Probe => (Const, false),
        NilCheck | NilReason => (Const, false),
        True | False | Nil | Pi => (Const, false),
        // Structure builders bounded by their operands' total size.
        Concat | Reverse => (Linear, true),
        Take | Collect => (Linear, false),
        // The value-driven materializers: a numeric operand's *value* sets the
        // materialized length (Phase 3 gives these the runtime water level).
        Range | Fill => (Unbounded, true),
        // Rounding/number casts: output bounded by operand digit count.
        Floor | Round | Quantize | Mod => (Linear, false),
        Str | Num | Chars | Tokenize | Trim => (Linear, false),
        // Repetition can multiply sizes (k × separator).
        Join => (Superlinear, false),
        // Dictionary registration copies bounded structure.
        Bind | Def => (Linear, false),
        Del => (Const, false),
        Print => (Linear, false),
        // The Words promoted out of the deleted MATH and ALGO modules.
        Abs | Neg | Min | Max | Sqrt => (Linear, false),
        Sort | Order => (Linear, true),
        IndexOf => (Linear, false),
        // Ordering, grouping and shape Words: the result is bounded by the
        // operands' total size, and a vector operand attains the bound.
        Unique | Tally | Zip | Put | Group => (Linear, true),
        // A fold to one value; the accumulator's digit count grows with the
        // input, so linear rather than constant.
        Sum => (Linear, false),
        // A value-driven materializer like RANGE and FILL: the *count*
        // operand's value sets the length, so it takes the runtime water level
        // rather than a static bound.
        Random => (Unbounded, true),
        // The positional control directives (SPEC §6.4) never reach a
        // primitive: the execution loop interprets them against the source
        // stream, so they materialize nothing.
        LazyNextUnitFallback | SetConsumptionKeep => (Const, false),
    }
}

/// A space-specific stack arity for a built-in whose `mass` contract is
/// `Dynamic` but whose *stack* arity is nonetheless fixed and known here. This
/// lets the simulation inspect the operand provenance of the value-driven
/// materializers — where a compile-time-literal operand collapses the class
/// from `Unbounded` to `Const` (`[ 0 10 ] RANGE`) — even though their `mass`
/// is conservatively `Dynamic`. Every other Dynamic-mass word is soundly
/// handled by the degrade-on-dynamic path.
fn space_arity_override(id: WordId) -> Option<(u16, u16)> {
    match id {
        WordId::Range | WordId::Fill => Some((1, 1)),
        _ => None,
    }
}

/// Space classification for a resolved built-in word, by canonical name.
/// A name the registry does not know is conservatively unclassified.
pub(crate) fn builtin_space_for(name: &str) -> (SpaceClass, bool) {
    match crate::kernel::generated::generated_word(name) {
        Some(word) => builtin_space(word.id),
        None => (SpaceClass::Unbounded, false),
    }
}

/// The space-model stack arity of a resolved built-in, or `None` when the model
/// has no fixed arity for it (so the simulation falls back to the contract flow).
fn builtin_space_arity(name: &str) -> Option<(u16, u16)> {
    crate::kernel::generated::generated_word(name).and_then(|word| space_arity_override(word.id))
}

/// What the simulation knows about one simulated stack slot.
#[derive(Debug, Clone, Copy)]
struct Slot {
    /// The value is a compile-time constant (value *and* size).
    lit: bool,
    /// The value is exactly a word input, moved untouched.
    input: bool,
    /// Sound upper bound on the slot's structural size as f(input size).
    size: SpaceClass,
}

const fn slot(lit: bool, input: bool, size: SpaceClass) -> Slot {
    Slot { lit, input, size }
}

/// A compile-time constant operand.
const LIT_SLOT: Slot = slot(true, false, SpaceClass::Const);
/// A word input, moved untouched.
const INPUT_SLOT: Slot = slot(false, true, SpaceClass::Linear);
/// Provenance lost: assume the worst about its size.
const UNKNOWN_SLOT: Slot = slot(false, false, SpaceClass::Unbounded);

/// The per-word contribution facts the simulation needs about a resolved
/// dependency, projected from its (builtin or inferred) contract.
pub(crate) struct DepSpace {
    pub flow: ContractFlow,
    /// A space-model stack arity that overrides `flow` when present (used for the
    /// value-driven materializers whose `mass` is `Dynamic`). `None` = use `flow`.
    pub arity_override: Option<(u16, u16)>,
    pub class: SpaceClass,
    /// Builtin: authored `tight`; user word: its inferred `space_exact`.
    pub tight: bool,
    /// True when the dependency is a built-in, whose class is a function of
    /// its operands (so a literal operand refines the contribution down).
    /// A user word's class is taken as-is — its growth may be internal.
    pub operand_driven: bool,
    /// Constant-folding licence: pure + deterministic + fully inferred.
    pub foldable: bool,
}

impl DepSpace {
    pub(crate) fn of_builtin(name: &str, contract: &WordContract) -> Self {
        let (class, tight) = builtin_space_for(name);
        DepSpace {
            flow: contract.flow.clone(),
            arity_override: builtin_space_arity(name),
            class,
            tight,
            operand_driven: true,
            foldable: contract.purity == ContractPurity::Pure
                && contract.determinism == ContractDeterminism::Deterministic,
        }
    }

    pub(crate) fn of_user_word(contract: &WordContract) -> Self {
        DepSpace {
            flow: contract.flow.clone(),
            arity_override: None,
            class: contract.space,
            tight: contract.space_exact,
            operand_driven: false,
            foldable: contract.purity == ContractPurity::Pure
                && contract.determinism == ContractDeterminism::Deterministic
                && contract.confidence == ContractConfidence::Complete,
        }
    }
}

/// `class` applied to an operand of size bound `size`: the materialization a
/// size-driven word performs on an operand no larger than `size`.
fn apply_to_size(class: SpaceClass, size: SpaceClass) -> SpaceClass {
    match class {
        SpaceClass::Const => SpaceClass::Const,
        SpaceClass::Linear => size,
        SpaceClass::Superlinear => match size {
            SpaceClass::Const => SpaceClass::Const,
            SpaceClass::Linear | SpaceClass::Superlinear => SpaceClass::Superlinear,
            SpaceClass::Unbounded => SpaceClass::Unbounded,
        },
        SpaceClass::Unbounded => SpaceClass::Unbounded,
    }
}

/// What this slot model learned about the operands of one dependency call.
/// Returned by `feed_word` so a second bound model over the same walk
/// (`word_cost::CostSim`) can refine its own classes against the identical
/// provenance instead of duplicating the slot stack and risking divergence.
#[derive(Debug, Clone, Copy)]
pub(crate) enum OperandProfile {
    /// Attributed, but no provenance is justified — a data-dependent arity, or
    /// a call among vector-literal elements the model does not track.
    Unknown,
    /// A fixed-arity call whose operand slots were all inspected.
    Known {
        all_lit: bool,
        all_traced: bool,
        max_size: SpaceClass,
    },
}

/// Execution-free space simulation over a word body's token stream, fed by the
/// contract-inference walk. Tracks slot provenance (literal / input / other) so
/// contributions can be *applied* to what actually feeds them, and degrades
/// soundly: any construct it cannot model clears the slots and poisons
/// underflow (an unknown slot, never a false `input`/`lit` tag).
pub(crate) struct SpaceSim {
    slots: Vec<Slot>,
    /// Underflow no longer provably reaches a word input (heights unknown).
    poisoned: bool,
    bound: SpaceBound,
    vector_depth: u32,
    vector_dirty: bool,
}

impl SpaceSim {
    pub(crate) fn new() -> Self {
        SpaceSim {
            slots: Vec::new(),
            poisoned: false,
            bound: SpaceBound::IDENTITY,
            vector_depth: 0,
            vector_dirty: false,
        }
    }

    fn pop(&mut self) -> Slot {
        match self.slots.pop() {
            Some(slot) => slot,
            None if self.poisoned => UNKNOWN_SLOT,
            None => INPUT_SLOT,
        }
    }

    fn degrade(&mut self) {
        self.slots.clear();
        self.poisoned = true;
    }

    /// A structural token outside any symbol dispatch. Vector literals
    /// collapse to one slot; their inner tokens are not simulated (any
    /// execution of one goes through a higher-order word, which is
    /// classified `Unbounded` at *its* call site).
    pub(crate) fn feed_structural(&mut self, token: &Token) {
        match token {
            Token::VectorStart => self.vector_depth += 1,
            Token::VectorEnd => {
                self.vector_depth = self.vector_depth.saturating_sub(1);
                if self.vector_depth == 0 {
                    if self.vector_dirty {
                        // A non-literal vector element is outside the model:
                        // account it as an unproven unbounded contribution.
                        self.bound.join(SpaceBound::CONSERVATIVE);
                        self.slots.push(UNKNOWN_SLOT);
                    } else {
                        self.slots.push(LIT_SLOT);
                    }
                    self.vector_dirty = false;
                }
            }
            // The lazy fallback unit of `OR-NIL` and COND clause separators change
            // heights along a path the linear walk cannot follow.
            Token::NilCoalesce | Token::CondClauseSep => self.degrade(),
            _ => {}
        }
    }

    /// A `Number`/`String` literal token.
    pub(crate) fn feed_literal(&mut self) {
        if self.vector_depth == 0 {
            self.slots.push(LIT_SLOT);
        }
        // A literal inside a vector keeps the vector clean.
    }

    /// A symbol that failed to resolve: unknown flow and unknown growth.
    pub(crate) fn feed_unresolved(&mut self) {
        if self.vector_depth > 0 {
            self.vector_dirty = true;
            return;
        }
        self.bound.join(SpaceBound::CONSERVATIVE);
        self.degrade();
    }

    /// The caller stopped feeding this line mid-way (a dependency could not be
    /// inferred), so structural depths can no longer be trusted: pin the bound
    /// to the conservative top and resynchronize for whatever follows.
    pub(crate) fn abandon_line(&mut self) {
        self.bound.join(SpaceBound::CONSERVATIVE);
        self.degrade();
        self.vector_depth = 0;
        self.vector_dirty = false;
    }

    /// A resolved dependency call. Returns what was learned about its operand
    /// slots, for a second bound model riding the same walk.
    pub(crate) fn feed_word(&mut self, dep: &DepSpace) -> OperandProfile {
        if self.vector_depth > 0 {
            self.vector_dirty = true;
            return OperandProfile::Unknown;
        }
        let arity = dep.arity_override.or(match dep.flow {
            ContractFlow::Fixed { consumes, produces } => Some((consumes, produces)),
            ContractFlow::Dynamic => None,
        });
        let Some((consumes, produces)) = arity else {
            // Data-dependent arity: heights unknown from here on.
            self.bound.join(SpaceBound {
                class: dep.class,
                exact: false,
            });
            self.degrade();
            return OperandProfile::Unknown;
        };

        let mut operands = Vec::with_capacity(consumes as usize);
        for _ in 0..consumes {
            operands.push(self.pop());
        }

        let all_lit = operands.iter().all(|o| o.lit);
        let all_traced = operands.iter().all(|o| o.lit || o.input);
        let contribution = if dep.operand_driven {
            match dep.class {
                SpaceClass::Const => SpaceBound {
                    class: SpaceClass::Const,
                    exact: true,
                },
                SpaceClass::Linear | SpaceClass::Superlinear => {
                    if all_lit {
                        SpaceBound {
                            class: SpaceClass::Const,
                            exact: true,
                        }
                    } else {
                        let applied = operands
                            .iter()
                            .map(|o| apply_to_size(dep.class, o.size))
                            .max()
                            .unwrap_or(SpaceClass::Const);
                        SpaceBound {
                            class: applied,
                            exact: dep.tight && all_traced && applied == dep.class,
                        }
                    }
                }
                SpaceClass::Unbounded => {
                    // Value-driven materializer: a constant operand pins the
                    // materialized amount; an input operand provably does not.
                    if all_lit {
                        SpaceBound {
                            class: SpaceClass::Const,
                            exact: true,
                        }
                    } else {
                        SpaceBound {
                            class: SpaceClass::Unbounded,
                            exact: dep.tight && all_traced,
                        }
                    }
                }
            }
        } else {
            // A user word's growth may be internal (not operand-driven), so
            // its class is never refined downward; it is attained only when
            // the word receives genuine inputs and its own bound is exact.
            SpaceBound {
                class: dep.class,
                exact: dep.tight && all_traced,
            }
        };
        self.bound.join(contribution);

        let lit_out = all_lit && dep.foldable;
        let out_size = if lit_out {
            SpaceClass::Const
        } else {
            operands
                .iter()
                .map(|o| o.size)
                .chain(std::iter::once(contribution.class))
                .max()
                .unwrap_or(SpaceClass::Const)
        };
        for _ in 0..produces {
            self.slots.push(Slot {
                lit: lit_out,
                input: false,
                size: out_size,
            });
        }

        OperandProfile::Known {
            all_lit,
            all_traced,
            max_size: operands
                .iter()
                .map(|o| o.size)
                .max()
                .unwrap_or(SpaceClass::Const),
        }
    }

    pub(crate) fn finish(self) -> SpaceBound {
        self.bound
    }
}
