//! Static Core Word execution-form and flow-mass contracts.
//!
//! Invariant: flow mass is derived only from the generated stack arity; dynamic
//! and control arities never acquire a guessed fixed contract.

use crate::kernel::generated::{Arity, GeneratedWord};
use serde::Serialize;

/// Static mass contract (SPEC §13.1): a word's flow-mass relationship under the
/// default target/consume mode. `consumes` operands are read and `produces` results
/// are pushed; under `KEEP` the `consumes` operands are additionally retained
/// (bifurcation, §13.2). This is the machine-readable form of the §13.1 "arity /
/// consumption / production / bifurcation" declaration; the NIL-projection part
/// of §13.1 is carried by `nil_policy`.
///
/// `Dynamic` marks a data-dependent arity (e.g. `COLLECT`'s count-driven gather
/// or runtime-shaped vector ops) that is not statically pinned; the static
/// mass-conservation validator abstains on `Dynamic` words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum MassContract {
    Fixed { consumes: u8, produces: u8 },
    Dynamic,
}

impl MassContract {
    /// `(consumes, produces)` when the contract is statically fixed.
    pub fn fixed(self) -> Option<(u8, u8)> {
        match self {
            MassContract::Fixed { consumes, produces } => Some((consumes, produces)),
            MassContract::Dynamic => None,
        }
    }
}

/// How a Coreword takes effect, as a machine-readable signal independent of the
/// human-facing `stack_effect` prose.
///
/// Most words are ordinary `RuntimeWord`s dispatched by name and consuming/
/// producing stack values. The lazy control directive of SPEC §6.4 is not: the
/// tokenizer emits it as a dedicated token (`^`/`VENT` -> `NilCoalesce`) and the
/// execution loop interprets the *following source unit* positionally rather
/// than popping operands. This enum lets generators and consistency tests assert
/// that classification instead of parsing the prose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum ExecutionForm {
    /// Ordinary word: dispatched by name, operates on stack operands.
    RuntimeWord,
    /// Lazy NIL-coalescing control directive: inspects the stack top and, if it
    /// is non-NIL, keeps it and skips the following source unit *unevaluated*;
    /// if it is NIL, discards it and evaluates the following source unit as the
    /// fallback. The fallback is source that follows the directive, never a
    /// value already on the stack (e.g. `VENT` / `^`).
    LazyNextUnitFallback,
}

/// The mass contract implied by a Word's declared stack arity.
///
/// `MassContract` is the analyzers' vocabulary — they need one bit, "is this
/// arity statically pinned" — while the specification distinguishes *why* an
/// arity is not pinned (`variable` data-dependence vs a `control` directive
/// that is not a stack operation at all). Both collapse to `Dynamic` here, so
/// the analyzers keep the question they can answer and the distinction stays
/// available in the registry for anything that needs it.
pub(super) fn mass_from_arity(word: &GeneratedWord) -> MassContract {
    match (word.stack_inputs, word.stack_outputs) {
        (Arity::Fixed(consumes), Arity::Fixed(produces)) => {
            MassContract::Fixed { consumes, produces }
        }
        _ => MassContract::Dynamic,
    }
}

/// The canonical mass contract for a Coreword, keyed by its canonical name.
/// Unknown or non-core names conservatively return `Dynamic`.
pub fn mass_contract(name: &str) -> MassContract {
    let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
    crate::kernel::generated::generated_word(&canonical)
        .map(mass_from_arity)
        .unwrap_or(MassContract::Dynamic)
}

/// Read runtime partiality from the canonical Word contract.
pub(crate) const fn partiality_from_contract(word: &GeneratedWord) -> super::Partiality {
    word.partiality
}

/// Derive the audit safety band: effects are boundary operations, while pure
/// total Words are A and pure partial/projecting Words are B.
pub(crate) const fn safety_from_contract(word: &GeneratedWord) -> super::SafetyLevel {
    if !word.effects.is_empty() {
        super::SafetyLevel::D
    } else {
        match partiality_from_contract(word) {
            super::Partiality::Total => super::SafetyLevel::A,
            super::Partiality::Partial | super::Partiality::Projecting => super::SafetyLevel::B,
        }
    }
}

/// Preview is permitted exactly for effect-free pure Words.
pub(crate) const fn safe_preview_from_contract(word: &GeneratedWord) -> bool {
    word.effects.is_empty() && matches!(word.purity, crate::kernel::generated::Purity::Pure)
}

/// Stability is a projection of the audit safety band, never a parallel label.
pub(crate) const fn stability_from_contract(word: &GeneratedWord) -> &'static str {
    match safety_from_contract(word) {
        super::SafetyLevel::A | super::SafetyLevel::B => "stable",
        super::SafetyLevel::C | super::SafetyLevel::D | super::SafetyLevel::Quarantined => {
            "experimental"
        }
    }
}

/// Positional control is identified by the canonical executor identity.
pub(crate) const fn execution_form_from_contract(word: &GeneratedWord) -> ExecutionForm {
    match word.id {
        crate::kernel::generated::WordId::LazyNextUnitFallback => {
            ExecutionForm::LazyNextUnitFallback
        }
        _ => ExecutionForm::RuntimeWord,
    }
}
