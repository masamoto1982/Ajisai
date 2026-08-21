//! Gap identifiers: the stable breakdown of `check --contract`'s "cannot
//! verify" result (Phase 3, `docs/dev/competitive-advantage-work-order-2026-08.md`).
//!
//! `LANG.CONTRACT.CHECK` fixes exactly three results — verified / cannot
//! verify / violated — and a gap identifier is the *reason* behind a "cannot
//! verify", never a fourth result: `violated` is still decided the same way
//! it always was (`findings.iter().any(|f| f.severity == Severity::Error)`),
//! untouched by this module. Contract inference (`word_contract.rs`) goes
//! conservative at exactly three sites plus one seed used elsewhere in the
//! same file:
//!
//!  * a symbol a word's body calls does not resolve to any word
//!    (`UnresolvedWord`);
//!  * a word's own inference is re-entered while it is still being inferred
//!    — direct or mutual recursion (`RecursiveDependency`);
//!  * a dependency's own inference could not complete, so nothing sound can
//!    be said about calling it (`DependencyUnknown`);
//!  * `WordContract::conservative()` is reached as a fallback seed rather
//!    than through one of the three sites above (`ConservativeSeed`).
//!
//! These four and no others: a fifth incompleteness source is a design
//! decision (which bucket does it belong in, or does it need one of its
//! own), not something to invent here silently.
//!
//! A gap identifier has the same character as a NIL reason
//! (`LANG.VALUES.NIL`): a human-readable message can be reworded without
//! notice, but this id names *why* inference gave up and stays stable across
//! that rewording — the same guarantee that makes `"error:<category>"` in
//! the Phase 2 semantics table meaningful across CI runs.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum GapCode {
    UnresolvedWord,
    RecursiveDependency,
    DependencyUnknown,
    ConservativeSeed,
}

impl GapCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            GapCode::UnresolvedWord => "gap.unresolvedWord",
            GapCode::RecursiveDependency => "gap.recursiveDependency",
            GapCode::DependencyUnknown => "gap.dependencyUnknown",
            GapCode::ConservativeSeed => "gap.conservativeSeed",
        }
    }
}

/// Which of `LANG.CONTRACT.CHECK`'s three results one `#:contract`
/// declaration landed in (`contract_decl::check_one`'s per-declaration
/// classification, Step 3.4). Not derived from a `Vec<DeclFinding>` by
/// counting severities: one declaration can contribute more than one finding
/// (e.g. a purity *and* a nil-free mismatch), which would double-count it —
/// the caller classifies each declaration once, from the findings that one
/// declaration's check produced.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeclOutcome {
    Verified,
    CannotVerify,
    Violated,
}

/// The `gapSummary` object (Step 3.4): a tally of the three
/// `LANG.CONTRACT.CHECK` results over `outcomes`, plus a stable-ordered
/// (`BTreeMap`, never `HashMap`) breakdown of which gap id every
/// cannot-verify finding cited.
pub(crate) fn gap_summary_json(
    outcomes: &[DeclOutcome],
    codes: impl Iterator<Item = &'static str>,
) -> serde_json::Value {
    let verified = outcomes
        .iter()
        .filter(|o| **o == DeclOutcome::Verified)
        .count();
    let cannot_verify = outcomes
        .iter()
        .filter(|o| **o == DeclOutcome::CannotVerify)
        .count();
    let violated = outcomes
        .iter()
        .filter(|o| **o == DeclOutcome::Violated)
        .count();
    let mut by_gap: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for code in codes {
        *by_gap.entry(code).or_insert(0) += 1;
    }
    serde_json::json!({
        "declarationsChecked": outcomes.len(),
        "verified": verified,
        "cannotVerify": cannot_verify,
        "violated": violated,
        "byGap": by_gap,
    })
}
