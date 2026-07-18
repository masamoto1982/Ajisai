/// Purity metadata for every builtin word in the Ajisai vocabulary.
///
/// # Purity categories
/// - `Pure`   — deterministic, no observable side effects; safe to cache and reorder.
/// - `Impure` — has observable side effects (I/O, time, randomness, dictionary mutation).
/// - `Unknown` — purity depends on runtime arguments (e.g. higher-order words whose
///              callback may be impure) or is otherwise unanalysable statically.
///
/// For user-defined words, use `infer_purity` to propagate conservatively from
/// component words.
use crate::builtins::BuiltinExecutorKey;

// ── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purity {
    Pure,
    Impure,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalCost {
    /// Constant-time arithmetic / boolean ops.
    Trivial,
    /// Small fixed-overhead operations (casts, single-element lookups).
    Light,
    /// Linear collection traversals.
    Medium,
    /// Unbounded / recursive / I-O-bound operations.
    Heavy,
}

#[derive(Debug, Clone, Copy)]
pub struct PurityInfo {
    pub purity: Purity,
    pub cost: EvalCost,
    /// If `true`, the result depends on evaluation order relative to other
    /// words and must not be reordered.
    pub order_sensitive: bool,
}

// ── Builtin key → PurityInfo ──────────────────────────────────────────────────

/// Return `PurityInfo` for a given `BuiltinExecutorKey`.
///
/// Phase 3 keeps this as a compatibility adapter, but the authored metadata now
/// lives on `BuiltinSpec` so optimizers and the word registry do not maintain
/// separate builtin purity tables.
pub fn builtin_purity(key: BuiltinExecutorKey) -> PurityInfo {
    let spec = crate::builtins::builtin_specs()
        .iter()
        .find(|spec| spec.executor_key == Some(key));
    spec.map(purity_info_from_spec).unwrap_or(PurityInfo {
        purity: Purity::Unknown,
        cost: EvalCost::Heavy,
        order_sensitive: true,
    })
}

fn purity_info_from_spec(spec: &crate::builtins::BuiltinSpec) -> PurityInfo {
    let purity = match spec.purity {
        crate::coreword_registry::WordPurity::Pure => Purity::Pure,
        crate::coreword_registry::WordPurity::Observable
        | crate::coreword_registry::WordPurity::Effectful => Purity::Impure,
    };
    PurityInfo {
        purity,
        cost: spec.eval_cost,
        order_sensitive: spec.order_sensitive,
    }
}

/// Look up `PurityInfo` for a builtin word by its source-level name.
///
/// Returns `None` for user-defined or unknown words; use `infer_purity`
/// to handle those.
pub fn purity_by_name(name: &str) -> Option<PurityInfo> {
    let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
    if let Some(info) = crate::builtins::lookup_builtin_spec(&canonical)
        .and_then(|spec| spec.executor_key)
        .map(builtin_purity)
    {
        return Some(info);
    }

    match canonical.as_ref() {
        "NOW" | "DATETIME" | "TIMESTAMP" | "TIME@NOW" | "TIME@DATETIME" | "TIME@TIMESTAMP" => {
            Some(PurityInfo {
                purity: Purity::Impure,
                cost: EvalCost::Light,
                order_sensitive: true,
            })
        }
        "CSPRNG" | "CRYPTO@CSPRNG" => Some(PurityInfo {
            purity: Purity::Impure,
            cost: EvalCost::Light,
            order_sensitive: true,
        }),
        // Serial I/O drives external hardware: always impure, order-sensitive,
        // and never eligible for speculative reordering or caching.
        "SERIAL@LIST-PORTS" | "SERIAL@OPEN" | "SERIAL@CONFIGURE" | "SERIAL@WRITE"
        | "SERIAL@READ" | "SERIAL@FLUSH" | "SERIAL@CLOSE" => Some(PurityInfo {
            purity: Purity::Impure,
            cost: EvalCost::Heavy,
            order_sensitive: true,
        }),
        "HASH" | "CRYPTO@HASH" | "SORT" | "ALGO@SORT" => Some(PurityInfo {
            purity: Purity::Pure,
            cost: EvalCost::Light,
            order_sensitive: false,
        }),
        "SQRT" | "SQRT_EPS" | "INTERVAL" | "LOWER" | "UPPER" | "WIDTH" | "IS_EXACT"
        | "MATH@SQRT" | "MATH@SQRT-EPS" | "MATH@INTERVAL" | "MATH@LOWER" | "MATH@UPPER"
        | "MATH@WIDTH" | "MATH@IS-EXACT" => Some(PurityInfo {
            purity: Purity::Pure,
            cost: EvalCost::Light,
            order_sensitive: false,
        }),
        _ => None,
    }
}

// ── Inference for user-defined words ─────────────────────────────────────────

/// Conservatively infer purity from the purities of component words.
///
/// Rules (most-conservative wins):
/// 1. Any `Impure` component → result is `Impure`.
/// 2. Any `Unknown` component → result is `Unknown`.
/// 3. All `Pure` → result is `Pure`.
pub fn infer_purity(components: &[Purity]) -> Purity {
    if components.iter().any(|&p| p == Purity::Impure) {
        return Purity::Impure;
    }
    if components.iter().any(|&p| p == Purity::Unknown) {
        return Purity::Unknown;
    }
    Purity::Pure
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_purity_adapter_is_derived_from_builtin_spec() {
        for spec in crate::builtins::builtin_specs() {
            let Some(key) = spec.executor_key else {
                continue;
            };
            let info = builtin_purity(key);
            assert_eq!(info.cost, spec.eval_cost, "{} cost drift", spec.name);
            assert_eq!(
                info.order_sensitive, spec.order_sensitive,
                "{} order sensitivity drift",
                spec.name
            );
            let expected = match spec.purity {
                crate::coreword_registry::WordPurity::Pure => Purity::Pure,
                crate::coreword_registry::WordPurity::Observable
                | crate::coreword_registry::WordPurity::Effectful => Purity::Impure,
            };
            assert_eq!(info.purity, expected, "{} purity drift", spec.name);
        }
    }
}
