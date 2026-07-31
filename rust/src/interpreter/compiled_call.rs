//! Pre-resolved builtin call sites for compiled plans.
//!
//! A builtin call site, specialized once at compile time so the per-call
//! dispatch work (alias canonicalization, linear registry scan,
//! mode-preservation lookup) is never repeated at runtime.
//! This is the call-site analogue of the resolve cache's epoch discipline:
//! everything precomputed here depends only on static tables, never on
//! dictionary state, so no epoch guard is needed.
//!
//! See `docs/dev/hidden-class-shape-optimizations.md` for the design note.

use crate::error::Result;
use crate::kernel::generated::{generated_word, GeneratedWord};

use super::Interpreter;

#[derive(Debug)]
pub struct CompiledCall {
    /// Canonical builtin name (post-alias). Kept for the unresolved fallback
    /// path, diagnostics, and plan introspection.
    pub name: String,
    /// Pre-resolved contract, replacing the runtime alias scan + registry scan.
    /// `None` for a name the registry does not know — the fallback path reports
    /// it as an unknown word.
    pub word: Option<&'static GeneratedWord>,
    /// Precomputed `modules::is_mode_preserving_word(name)` so the post-call
    /// cleanup skips the per-call uppercase allocation.
    pub mode_preserving: bool,
}

impl CompiledCall {
    pub fn resolve(name: &str) -> Self {
        let canonical = crate::core_word_aliases::canonicalize_core_word_name(name).into_owned();
        let word = generated_word(&canonical);
        Self {
            mode_preserving: false,
            word,
            name: canonical,
        }
    }
}

/// Run a pre-resolved builtin call site. Mirrors `execute_builtin` exactly —
/// declared NIL contract, then executor dispatch — but
/// consumes the decisions `CompiledCall::resolve` already made instead of
/// re-scanning the alias and registry tables.
///
/// "Mirrors `execute_builtin` exactly" is the whole contract of this function,
/// and the NIL guard is part of what it has to mirror: a compiled body that
/// skipped the guard would let a Word behave one way when called directly and
/// another when called from inside a user word, which is precisely the
/// unobservability that compiling a body is required to preserve
/// (LANG.AUTHORITY.FREEDOM).
pub(crate) fn execute_compiled_call(interp: &mut Interpreter, call: &CompiledCall) -> Result<()> {
    let Some(word) = call.word else {
        return interp.execute_builtin_direct(&call.name);
    };
    if let Some(decided) = interp.apply_declared_nil_contract(word) {
        return decided;
    }
    interp.execute_builtin_by_id(word.id)
}
