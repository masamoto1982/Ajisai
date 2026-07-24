//! The static-footprint axis of a `#:contract` declaration (Phase 2 of the
//! structural-memory-safety roadmap; see `docs/dev/space-contract-design.md`).
//! Kept in its own module — like `contract_linearity.rs` — so `contract_decl.rs`
//! stays within the §14.1 file-size budget.
//!
//! Increment 2.1 parsed and recorded the axis. Increment 2.2 (this file) wires
//! the check to the inferred space bound (`crate::interpreter::word_space`): the
//! declaration checker compares the declared class against the class the
//! execution-free inference derives from the word body, and reports a
//! declaration the inference *provably* exceeds as an `error`. Soundness is
//! preserved by the inference's exactness witness — a mismatch is an `error`
//! only when the inferred class is provably *attained*; an unproven upper bound
//! yields a "cannot verify" note, never a false error.

use super::contract_decl::{ContractDecl, DeclFinding};
use super::explain::Lang;
use super::plan_check::Severity;
use crate::interpreter::word_contract::WordContract;
use crate::interpreter::word_space::SpaceClass as InferredClass;

/// Declared growth class of a word's *extra materialization* as a function of
/// its input. Ordered tightest → loosest; the inference (`word_space`) widens
/// monotonically along the same order.
///
/// `const`       — O(1) new nodes, independent of input size.
/// `linear`      — O(n) in the total input size.
/// `superlinear` — grows faster than input but still a function of it.
/// `unbounded`   — materialization is set by a *value* (e.g. a numeric operand
///                 of `RANGE`/`FILL`), so no static bound over input size exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpaceClass {
    Const,
    Linear,
    Superlinear,
    Unbounded,
}

impl SpaceClass {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SpaceClass::Const => "space:const",
            SpaceClass::Linear => "space:linear",
            SpaceClass::Superlinear => "space:superlinear",
            SpaceClass::Unbounded => "space:unbounded",
        }
    }

    /// The declared class as the inference lattice's value, so the two can be
    /// compared by the same widening order.
    fn as_inferred(self) -> InferredClass {
        match self {
            SpaceClass::Const => InferredClass::Const,
            SpaceClass::Linear => InferredClass::Linear,
            SpaceClass::Superlinear => InferredClass::Superlinear,
            SpaceClass::Unbounded => InferredClass::Unbounded,
        }
    }
}

fn inferred_str(class: InferredClass) -> &'static str {
    match class {
        InferredClass::Const => "space:const",
        InferredClass::Linear => "space:linear",
        InferredClass::Superlinear => "space:superlinear",
        InferredClass::Unbounded => "space:unbounded",
    }
}

/// Parse a `space:<class>` term, or `None` if the word is not one. The single
/// token form deliberately avoids colliding with the bare `linear` of the
/// linearity axis.
pub(crate) fn space_from_word(word: &str) -> Option<SpaceClass> {
    match word {
        "space:const" => Some(SpaceClass::Const),
        "space:linear" => Some(SpaceClass::Linear),
        "space:superlinear" => Some(SpaceClass::Superlinear),
        "space:unbounded" => Some(SpaceClass::Unbounded),
        _ => None,
    }
}

/// Check a declared space class against the inferred space bound (Phase 2.2).
///
/// The inferred `space` is a sound *upper* bound on the word's growth, and
/// `space_exact` records whether that bound is provably attained. Three cases:
///
/// - declared ≥ inferred upper bound → the declaration is *proved to hold*
///   (actual growth ≤ inferred ≤ declared): a verified note.
/// - declared < inferred upper bound, and the bound is exact → the word
///   *provably* grows faster than declared: an `error`.
/// - declared < inferred upper bound, but the bound is not exact → the upper
///   bound is unproven, so the declaration might still hold: a "cannot verify"
///   note, never a false error.
pub(crate) fn check_space(
    decl: &ContractDecl,
    space: SpaceClass,
    contract: &WordContract,
    lang: Lang,
    findings: &mut Vec<DeclFinding>,
) {
    let declared = space.as_inferred();
    let inferred = contract.space;

    if declared >= inferred {
        findings.push(DeclFinding {
            severity: Severity::Note,
            message: match lang {
                Lang::Ja => format!(
                    "`#:contract {}`: 空間クラス `{}` を検証しました(推論上界 `{}`)。",
                    decl.name,
                    space.as_str(),
                    inferred_str(inferred)
                ),
                Lang::En => format!(
                    "`#:contract {}`: verified space class `{}` (inferred upper bound `{}`).",
                    decl.name,
                    space.as_str(),
                    inferred_str(inferred)
                ),
            },
        });
        return;
    }

    // declared < inferred upper bound.
    if contract.space_exact {
        findings.push(DeclFinding {
            severity: Severity::Error,
            message: match lang {
                Lang::Ja => format!(
                    "`#:contract {}`: 空間クラス `{}` を宣言していますが、推論は `{}` を確定的に materialize します(空間契約違反)。",
                    decl.name,
                    space.as_str(),
                    inferred_str(inferred)
                ),
                Lang::En => format!(
                    "`#:contract {}`: declared space class `{}` but inference proves it materializes `{}` (space-contract violation).",
                    decl.name,
                    space.as_str(),
                    inferred_str(inferred)
                ),
            },
        });
    } else {
        findings.push(DeclFinding {
            severity: Severity::Note,
            message: match lang {
                Lang::Ja => format!(
                    "`#:contract {}`: 空間クラス `{}` を検証できません(推論上界 `{}` は確証なし)。",
                    decl.name,
                    space.as_str(),
                    inferred_str(inferred)
                ),
                Lang::En => format!(
                    "`#:contract {}`: cannot verify space class `{}` (inferred upper bound `{}` is unproven).",
                    decl.name,
                    space.as_str(),
                    inferred_str(inferred)
                ),
            },
        });
    }
}
