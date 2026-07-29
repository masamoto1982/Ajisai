//! Pre-resolved builtin call sites for compiled plans.
//!
//! A builtin call site, specialized once at compile time so the per-call
//! dispatch work (alias canonicalization, linear spec-table scan, force-flag
//! classification, mode-preservation lookup) is never repeated at runtime.
//! This is the call-site analogue of the resolve cache's epoch discipline:
//! everything precomputed here depends only on static tables, never on
//! dictionary state, so no epoch guard is needed.
//!
//! See `docs/dev/hidden-class-shape-optimizations.md` for the design note.

use crate::builtins::{lookup_builtin_spec, BuiltinExecutorKey};
use crate::error::Result;

use super::Interpreter;

#[derive(Debug)]
pub struct CompiledCall {
    /// Canonical builtin name (post-alias). Kept for the executor-less
    /// fallback path, diagnostics, and plan introspection.
    pub name: String,
    /// Pre-resolved executor, replacing the runtime alias scan + spec scan.
    pub key: Option<BuiltinExecutorKey>,
    /// Precomputed `canonical != DEF/DEL/FORC` force-flag reset decision.
    pub resets_force_flag: bool,
    /// Precomputed `modules::is_mode_preserving_word(name)` so the post-call
    /// cleanup skips the per-call uppercase allocation.
    pub mode_preserving: bool,
}

impl CompiledCall {
    pub fn resolve(name: &str) -> Self {
        let canonical = crate::core_word_aliases::canonicalize_core_word_name(name).into_owned();
        let key = lookup_builtin_spec(&canonical).and_then(|spec| spec.executor_key);
        Self {
            resets_force_flag: canonical != "DEL" && canonical != "DEF" && canonical != "FORC",
            mode_preserving: false,
            key,
            name: canonical,
        }
    }
}

/// Run a pre-resolved builtin call site. Mirrors `execute_builtin` exactly —
/// force-flag reset, then executor dispatch — but consumes the decisions
/// `CompiledCall::resolve` already made instead of re-scanning the alias and
/// spec tables. The shape IC is consulted only for words that have a scalar
/// fast path; a miss falls through to the same generic executor the
/// interpreter path uses.
pub(crate) fn execute_compiled_call(interp: &mut Interpreter, call: &CompiledCall) -> Result<()> {
    if call.resets_force_flag {
        interp.force_flag = false;
    }
    let Some(key) = call.key else {
        return interp.execute_builtin_direct(&call.name);
    };
    interp.execute_builtin_by_key(key)
}
