//! The static-footprint axis of a `#:contract` declaration (Phase 2 of the
//! structural-memory-safety roadmap; see `docs/dev/space-contract-design.md`).
//! Kept in its own module — like `contract_linearity.rs` — so `contract_decl.rs`
//! stays within the §14.1 file-size budget and the space discipline has a home
//! as it grows into the footprint inference of increment 2.2.
//!
//! Increment 2.1 (this file) parses and records the axis. The inference that
//! assigns each word a space class and widens it over a user word's body is not
//! wired yet, so a declared class is surfaced as a `note`, never a false
//! `error`, preserving the "unprovable declaration is a note" invariant of the
//! declaration checker.

use super::contract_decl::{ContractDecl, DeclFinding};
use super::explain::Lang;
use super::plan_check::Severity;

/// Declared growth class of a word's *extra materialization* as a function of
/// its input. Ordered tightest → loosest; inference (2.2) widens monotonically.
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

/// Record a declared space class (Phase 2, increment 2.1). Footprint inference
/// is not wired yet, so this only acknowledges the declaration as a `note`; it
/// never raises an `error`.
pub(crate) fn check_space(
    decl: &ContractDecl,
    space: SpaceClass,
    lang: Lang,
    findings: &mut Vec<DeclFinding>,
) {
    findings.push(DeclFinding {
        severity: Severity::Note,
        message: match lang {
            Lang::Ja => format!(
                "`#:contract {}`: 空間クラス `{}` を記録しました(未検証: フットプリント推論は未実装)。",
                decl.name,
                space.as_str()
            ),
            Lang::En => format!(
                "`#:contract {}`: recorded space class `{}` (unverified: footprint inference is not implemented yet).",
                decl.name,
                space.as_str()
            ),
        },
    });
}
