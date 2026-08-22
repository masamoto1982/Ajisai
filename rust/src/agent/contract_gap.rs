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
//! A fifth was added when the stack-flow simulation stopped guessing at the
//! constructs it cannot model (`word_contract_flow.rs`):
//!
//!  * the body reaches a control directive whose paths differ in stack height
//!    (`^`, `|`), or an unbalanced `[`/`{` delimiter, so no fixed arity
//!    describes it (`UnmodelledControlFlow`).
//!
//! It earns an id of its own rather than being folded into `ConservativeSeed`
//! for the reason that seed is named after: `ConservativeSeed` says inference
//! fell back to `WordContract::conservative()`, which this site does not do —
//! it produces a perfectly ordinary contract whose *flow* alone is
//! unmodelled. Reusing that id would make `byGap` count two different
//! situations as one, which is exactly the telemetry the gap ids exist to
//! keep apart.
//!
//! These five and no others: a sixth incompleteness source is a design
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
    UnmodelledControlFlow,
}

impl GapCode {
    const ALL: &'static [GapCode] = &[
        GapCode::UnresolvedWord,
        GapCode::RecursiveDependency,
        GapCode::DependencyUnknown,
        GapCode::ConservativeSeed,
        GapCode::UnmodelledControlFlow,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            GapCode::UnresolvedWord => "gap.unresolvedWord",
            GapCode::RecursiveDependency => "gap.recursiveDependency",
            GapCode::DependencyUnknown => "gap.dependencyUnknown",
            GapCode::ConservativeSeed => "gap.conservativeSeed",
            GapCode::UnmodelledControlFlow => "gap.unmodelledControlFlow",
        }
    }

    /// The gap a protocol string names, or `None` when it names none.
    /// `contract_decl::check_contract_decls` uses this to recover the
    /// `GapCode` a `DeclFinding::code` string already carries, so a
    /// declaration's per-file `CheckOutcome::Nil` payload does not require a
    /// second, parallel source of the same fact.
    pub(crate) fn from_str(s: &str) -> Option<GapCode> {
        Self::ALL.iter().copied().find(|gap| gap.as_str() == s)
    }
}

/// Which of `LANG.CONTRACT.CHECK`'s three results one `#:contract`
/// declaration landed in — the vocabulary of `LANG.FAILURE.TRICHOTOMY`
/// applied at check time rather than run time (Phase 4). Not derived from a
/// `Vec<DeclFinding>` by counting severities: one declaration can contribute
/// more than one finding (e.g. a purity *and* a nil-free mismatch), which
/// would double-count it — the caller classifies each declaration once, from
/// the findings that one declaration's check produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CheckOutcome {
    /// verified — inference produced a contract and it matched the
    /// declaration. The value is the inferred contract itself.
    Value,
    /// cannot verify — a well-formed check could not decide. The reason is
    /// the gap id (Phase 3) inference recorded.
    Nil(GapCode),
    /// violated — the declaration contradicts a proven contract.
    Error,
}

impl CheckOutcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            CheckOutcome::Value => "value",
            CheckOutcome::Nil(_) => "nil",
            CheckOutcome::Error => "error",
        }
    }
}

/// The file-wide `outcome` (Step 4.2): not a choice, a derivation from
/// `LANG.FAILURE`. ERROR propagates and halts evaluation, so one error
/// anywhere decides the whole file; NIL flows downstream only once nothing
/// halted first, so it decides the file only when no error is present;
/// nothing outstanding (or an empty declaration set) is a value. Which
/// specific gap a returned `Nil` carries is not itself meaningful here — the
/// file-level `outcome` field is the bare `"value"`/`"nil"`/`"error"` string
/// (`as_str`), never a payload — so any cannot-verify declaration may stand
/// in for the file.
pub(crate) fn fold_outcomes(outcomes: &[CheckOutcome]) -> CheckOutcome {
    if outcomes.iter().any(|o| matches!(o, CheckOutcome::Error)) {
        return CheckOutcome::Error;
    }
    if let Some(nil) = outcomes.iter().find(|o| matches!(o, CheckOutcome::Nil(_))) {
        return *nil;
    }
    CheckOutcome::Value
}

/// The `gapSummary` object (Step 3.4): a tally of the three
/// `LANG.CONTRACT.CHECK` results over `outcomes` (one per declaration), plus
/// a stable-ordered (`BTreeMap`, never `HashMap`) breakdown of which gap id
/// every cannot-verify *finding* cited. `codes` is intentionally a separate
/// per-finding sequence rather than derived from `outcomes`: a declaration
/// can carry more than one cannot-verify finding (e.g. arity, purity, and
/// nil-free all unverifiable on the same recursive word), and `byGap` counts
/// each of those, not one per declaration — changing that would be a
/// backward-incompatible change to a field Phase 3 already shipped.
pub(crate) fn gap_summary_json(
    outcomes: &[CheckOutcome],
    codes: impl Iterator<Item = &'static str>,
) -> serde_json::Value {
    let verified = outcomes
        .iter()
        .filter(|o| matches!(o, CheckOutcome::Value))
        .count();
    let cannot_verify = outcomes
        .iter()
        .filter(|o| matches!(o, CheckOutcome::Nil(_)))
        .count();
    let violated = outcomes
        .iter()
        .filter(|o| matches!(o, CheckOutcome::Error))
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

/// One `contractDecls.declarations[]` entry (Step 4.3): the word, its
/// outcome, and — only for the two outcomes that carry one — the reason
/// (`nil`) or category (`error`).
pub(crate) fn declaration_json(word: &str, outcome: CheckOutcome) -> serde_json::Value {
    match outcome {
        CheckOutcome::Value => serde_json::json!({ "word": word, "outcome": "value" }),
        CheckOutcome::Nil(gap) => serde_json::json!({
            "word": word,
            "outcome": "nil",
            "reason": gap.as_str(),
        }),
        CheckOutcome::Error => serde_json::json!({
            "word": word,
            "outcome": "error",
            // `ErrorCategory` (rust/src/error.rs) is the *runtime* error
            // registry; a contract violation is not a runtime error, so this
            // is a literal string rather than a variant of that enum
            // (Phase 4 pitfall B).
            "category": "contractViolation",
        }),
    }
}
