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
pub fn builtin_purity(key: BuiltinExecutorKey) -> PurityInfo {
    use BuiltinExecutorKey::*;
    use EvalCost::*;
    use Purity::*;

    let pure_trivial = PurityInfo {
        purity: Pure,
        cost: Trivial,
        order_sensitive: false,
    };
    let pure_light = PurityInfo {
        purity: Pure,
        cost: Light,
        order_sensitive: false,
    };
    let unk_medium = PurityInfo {
        purity: Unknown,
        cost: Medium,
        order_sensitive: false,
    };
    let unk_med_ord = PurityInfo {
        purity: Unknown,
        cost: Medium,
        order_sensitive: true,
    };
    let unk_heavy = PurityInfo {
        purity: Unknown,
        cost: Heavy,
        order_sensitive: true,
    };
    let imp_light = PurityInfo {
        purity: Impure,
        cost: Light,
        order_sensitive: true,
    };
    let imp_heavy = PurityInfo {
        purity: Impure,
        cost: Heavy,
        order_sensitive: true,
    };

    match key {
        // ── Pure arithmetic ───────────────────────────────────────────────
        Add | Sub | Mul | Div | Mod | Floor | Ceil | Round | Sqrt | SqrtEps => pure_trivial,
        Interval | Lower | Upper | Width | IsExact => pure_light,

        // ── Pure comparison ───────────────────────────────────────────────
        Eq | Lt | Le => pure_trivial,

        // ── Pure logic ────────────────────────────────────────────────────
        And | Or | Not => pure_trivial,

        // ── Pure constants ────────────────────────────────────────────────
        True | False | Nil | Idle => pure_trivial,

        // ── Pure casts / string ops ───────────────────────────────────────
        Str | Num | Bool | Chr | Chars | Join => pure_light,

        // ── Pure vector ops ───────────────────────────────────────────────
        Get | Length | Concat | Reverse | Range | Reorder | Sort => pure_light,
        Take | Split | Insert | Replace | Remove | Collect => pure_light,

        // ── Pure tensor ops ───────────────────────────────────────────────
        Shape | Rank | Reshape | Transpose | Fill => pure_light,

        // ── Hash: deterministic → pure ────────────────────────────────────
        Hash => pure_light,

        // ── Higher-order: purity depends on callback → unknown ────────────
        // (the word itself is fine but the callback may be impure)
        Map | Filter | Any | All | Count => unk_medium,

        // order matters for fold / scan / unfold (left-to-right accumulation)
        Fold | Scan | Unfold => unk_med_ord,

        // ── Control flow: unknown ─────────────────────────────────────────
        Cond | Exec | Eval => unk_heavy,

        // ── Dictionary mutators: impure ───────────────────────────────────
        Def | Del | Import | ImportOnly | Force | Lookup => imp_heavy,

        // ── I/O: impure ───────────────────────────────────────────────────
        Print => imp_heavy,

        // ── Time / randomness: non-deterministic ─────────────────────────
        Now | Datetime | Timestamp | Csprng => imp_light,

        // ── Concurrency: impure ───────────────────────────────────────────
        Spawn | Await | Status | Kill | Monitor | Supervise => imp_heavy,
    }
}

/// Look up `PurityInfo` for a builtin word by its source-level name.
///
/// Returns `None` for user-defined or unknown words; use `infer_purity`
/// to handle those.
pub fn purity_by_name(name: &str) -> Option<PurityInfo> {
    if let Some(info) = crate::builtins::lookup_builtin_spec(name)
        .and_then(|spec| spec.executor_key)
        .map(builtin_purity)
    {
        return Some(info);
    }

    match name.to_uppercase().as_str() {
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
        "HASH" | "CRYPTO@HASH" | "SORT" | "ALGO@SORT" => Some(PurityInfo {
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
