//! The resource-ownership axis of a `#:contract` declaration (Phase 1 of the
//! structural-memory-safety roadmap; see
//! `docs/dev/structural-memory-safety-roadmap.md`). Split out of
//! `contract_decl.rs` to keep that file within the §14.1 file-size budget and
//! to give the linearity discipline its own home as it grows into the
//! handle-obligation inference of increment 2.
//!
//! Increment 2 adds the first *enforcing* check: a handle is a linear resource,
//! so applying the `KEEP` consumption modifier to a handle-discharging word
//! (`KILL` / `AWAIT`) retains the handle past its one permitted consumption —
//! a use-after-discharge / duplication that `linear` (and `affine`) forbid.
//! Because a modifier is its own token that binds the operating word that
//! follows it (`1 2 KEEP ADD`), this is detectable directly on a word's body
//! tokens with no execution and no false positives on valid programs. Deeper
//! flow-sensitive tracking (a handle dropped, or discharged across a call
//! boundary) is deferred to a later increment.

use super::contract_decl::{ContractDecl, DeclFinding};
use super::explain::Lang;
use super::plan_check::Severity;
use crate::core_word_aliases::canonicalize_core_word_name;
use crate::interpreter::Interpreter;
use crate::types::Token;

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

/// The resource role of a runtime word with respect to a handle. Words outside
/// this set do not touch a handle obligation. Canonical (alias-resolved) names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HandleRole {
    /// Mints a fresh handle obligation (`SPAWN`, `SUPERVISE`).
    Create,
    /// Consumes a handle, discharging its obligation (`KILL`, `AWAIT`).
    Discharge,
    /// Reads a handle without consuming it (`STATUS`, `MONITOR`); `KEEP` here is
    /// the *correct* idiom, so it is never a violation.
    Observe,
}

/// Classify a canonical word name by its handle role, or `None` if it does not
/// touch a handle.
pub(crate) fn handle_role(canonical: &str) -> Option<HandleRole> {
    match canonical {
        "SPAWN" | "SUPERVISE" => Some(HandleRole::Create),
        "KILL" | "AWAIT" => Some(HandleRole::Discharge),
        "STATUS" | "MONITOR" => Some(HandleRole::Observe),
        _ => None,
    }
}

/// Does the word body apply `KEEP` to a handle-discharging word?
///
/// A consumption modifier is its own token that binds the operating word that
/// follows it, so we carry a `pending_keep` flag across a run of modifier
/// tokens and test it when the next operating word turns out to be a discharge.
/// `EAT` on the same word cancels the intent (it is an explicit consume), and
/// any non-modifier, non-discharge token resets the pending flag — both keep
/// the check free of false positives.
pub(crate) fn body_keeps_a_discharged_handle(tokens: &[Token]) -> bool {
    let mut pending_keep = false;
    for token in tokens {
        let Token::Symbol(symbol) = token else {
            pending_keep = false;
            continue;
        };
        let canonical = canonicalize_core_word_name(symbol);
        match canonical.as_ref() {
            "KEEP" => pending_keep = true,
            // `EAT` overrides a preceding `KEEP` on the same word; the target
            // axis (`TOP`/`STAK`) is orthogonal and leaves the flag untouched.
            "EAT" => pending_keep = false,
            "TOP" | "STAK" => {}
            other => {
                if pending_keep && handle_role(other) == Some(HandleRole::Discharge) {
                    return true;
                }
                pending_keep = false;
            }
        }
    }
    false
}

/// Enforce the handle-linearity discipline for a declared word (Phase 1,
/// increment 2). The only enforcing check today is sound and flow-insensitive:
/// applying `KEEP` to a handle-discharging word retains the handle past its one
/// permitted consumption, which `linear`/`affine` forbid. `droppable` opts out,
/// so it is only acknowledged. Anything not provably a violation stays a `note`,
/// preserving the "never a false error" invariant of the declaration checker.
pub(crate) fn check_linearity(
    interp: &Interpreter,
    decl: &ContractDecl,
    linearity: Linearity,
    lang: Lang,
    findings: &mut Vec<DeclFinding>,
) {
    let enforcing = matches!(linearity, Linearity::Linear | Linearity::Affine);
    let keeps_a_discharged_handle = enforcing
        && interp
            .resolve_word_entry_readonly(&decl.name)
            .map(|(_, def)| {
                def.lines
                    .iter()
                    .any(|line| body_keeps_a_discharged_handle(&line.body_tokens))
            })
            .unwrap_or(false);

    if keeps_a_discharged_handle {
        findings.push(DeclFinding {
            severity: Severity::Error,
            message: match lang {
                Lang::Ja => format!(
                    "`#:contract {}`: `{}` を宣言していますが、ハンドルを破棄する語(`KILL`/`AWAIT`)に `KEEP` を適用しており、消費後もハンドルが残ります(線形性違反)。",
                    decl.name,
                    linearity.as_str()
                ),
                Lang::En => format!(
                    "`#:contract {}`: declared `{}` but applies `KEEP` to a handle-discharging word (`KILL`/`AWAIT`), retaining the handle after its single consumption (linearity violation).",
                    decl.name,
                    linearity.as_str()
                ),
            },
        });
        return;
    }

    let note = match (linearity, lang) {
        (Linearity::Droppable, Lang::Ja) => format!(
            "`#:contract {}`: 線形性 `droppable`(規律から除外)を記録しました。",
            decl.name
        ),
        (Linearity::Droppable, Lang::En) => format!(
            "`#:contract {}`: recorded linearity `droppable` (opts out of the discipline).",
            decl.name
        ),
        (_, Lang::Ja) => format!(
            "`#:contract {}`: 線形性 `{}` を記録しました(KEEP によるハンドル破棄違反は検出されず。フロー全体の追跡は今後の増分)。",
            decl.name,
            linearity.as_str()
        ),
        (_, Lang::En) => format!(
            "`#:contract {}`: recorded linearity `{}` (no KEEP-on-discharge violation found; full flow-sensitive tracking is a later increment).",
            decl.name,
            linearity.as_str()
        ),
    };
    findings.push(DeclFinding {
        severity: Severity::Note,
        message: note,
    });
}
