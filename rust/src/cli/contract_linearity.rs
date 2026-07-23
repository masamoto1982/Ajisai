//! The resource-ownership axis of a `#:contract` declaration (Phase 1 of the
//! structural-memory-safety roadmap; see
//! `docs/dev/structural-memory-safety-roadmap.md`). Split out of
//! `contract_decl.rs` to keep that file within the §14.1 file-size budget and
//! to give the linearity discipline its own home as it grows into the
//! handle-obligation inference of increment 2.

/// Declared linearity of the resource a word produces or consumes. This axis
/// only constrains *resource* values — today the runtime handles
/// (`ProcessHandle` / `SupervisorHandle`, produced by `SPAWN`/`SUPERVISE`,
/// discharged by `KILL`/`AWAIT`/…). Ordinary immutable values are unaffected.
///
/// `linear`    — must be consumed exactly once (no leak, no reuse).
/// `affine`    — may be consumed at most once (leak allowed, reuse is not).
/// `droppable` — explicitly unconstrained; the escape hatch that opts a handle
///               back out of the discipline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Linearity {
    Linear,
    Affine,
    Droppable,
}

impl Linearity {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Linearity::Linear => "linear",
            Linearity::Affine => "affine",
            Linearity::Droppable => "droppable",
        }
    }
}

/// Parse a bare linearity term, or `None` if the word is not one.
pub(crate) fn linearity_from_word(word: &str) -> Option<Linearity> {
    match word {
        "linear" => Some(Linearity::Linear),
        "affine" => Some(Linearity::Affine),
        "droppable" => Some(Linearity::Droppable),
        _ => None,
    }
}
