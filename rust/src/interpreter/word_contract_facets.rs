//! The facets of an inferred contract, in the registry's own vocabulary.
//!
//! Split out of `word_contract.rs` for the file-size budget; the types are
//! re-exported from there.

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
