//! Static Core Word execution-form and flow-mass contracts.
//!
//! Invariant: flow mass is derived only from the generated stack arity; dynamic
//! and control arities never acquire a guessed fixed contract.

use crate::kernel::generated::{Arity, GeneratedWord};
use serde::Serialize;

/// Static mass contract: a word's flow-mass relationship. `consumes` operands
/// are read and removed, and `produces` results are pushed
/// (LANG.STACK.CONSUMPTION). This is the machine-readable form of the "arity /
/// consumption / production / bifurcation" declaration; the NIL-projection part
/// of LANG.MACHINE.WORD is carried by `nil_policy`.
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

/// The mass contract implied by a Word's declared stack arity.
///
/// `MassContract` is the analyzers' vocabulary — they need one bit, "is this
/// arity statically pinned". An arity that is not pinned is `variable`: it is
/// decided by the data. The `control` shape, for a directive that was not a
/// stack operation at all, went with the one Word that had it (`OR-NIL`).
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
