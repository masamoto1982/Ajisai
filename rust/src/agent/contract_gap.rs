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
//! A fifth names what the walk cannot see:
//!
//!  * the body runs code the walk never read, so it cannot say what that
//!    code does (`UnmodelledControlFlow`). The stack-flow simulation
//!    (`word_contract_flow.rs`) raises it when no fixed arity describes a
//!    body, and the widen step (`word_contract_widen::runs_unread_code`)
//!    raises it when a Word that runs its operand as code is handed anything
//!    but the `[ ... ]` literal written before it — a Vector taken out of
//!    data, a bound name, a dependency's result. Every such operand was seen,
//!    if at all, as inert data, so trusting the walk there would infer a
//!    false `pure` for `[ [ [ 42 PRINT ] ] 0 GET EXEC ]`.
//!
//! It earns an id of its own rather than being folded into `ConservativeSeed`
//! for the reason that seed is named after: `ConservativeSeed` says inference
//! fell back to `WordContract::conservative()` for a whole Word, which this
//! one does not do — the rest of the body is still read, and only the part
//! the walk could not see is widened to the unknown. Reusing that id would make `byGap` count two different
//! situations as one, which is exactly the telemetry the gap ids exist to
//! keep apart.
//!
//! A sixth, `OpaqueReflection`, closed a soundness hole around `REFLECT` —
//! a call that could turn a `Vector` whose elements were `String` tokens
//! into a `CodeBlock` whose Word names this walk had never seen as `Symbol`
//! tokens, so trusting `REFLECT`'s own registry contract for what the
//! reflected value did once run could infer a false `pure`/`complete`.
//! Retired along with `REFLECT` itself (CodeBlock/Vector unification,
//! docs/dev/type-unification-work-order-2026-08.md): every Vector is
//! already visible to this walk the ordinary way, so there is no longer a
//! second, opaque path back into executable code for it to guard against.
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
//!
//! The `cost` axis of a `#:contract` declaration (Phase 5 of `docs/dev/
//! competitive-advantage-work-order-2026-08.md`; design rationale in
//! `docs/dev/cost-contract-design.md`) is parsed and checked here too, beside
//! the gap ids and the declaration JSON the rest of `contract_decl.rs`'s
//! result is folded into — split out of that file to keep it within the
//! file-size budget in docs/dev/specification-implementation-rules.md.

use super::contract_decl::{DeclFinding, Severity};
use crate::interpreter::word_cost::{CostBound, CostClass};

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
/// more than one finding (e.g. a purity *and* a partiality mismatch), which
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
/// partiality all unverifiable on the same recursive word), and `byGap` counts
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

/// The `cost` term's three declarable axes — each `Some` axis is checked
/// against `WordContract::cost`; each `None` axis is left unchecked, the
/// same "omitted term is not checked" rule the other axes already follow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CostDecl {
    pub steps: Option<CostClass>,
    pub numeric: Option<CostClass>,
    pub collection: Option<CostClass>,
}

/// Parse the `axis=class` terms following a `cost` keyword, starting at
/// `rest[i]`. Returns the index just past the last consumed term, or an
/// error message on the first malformed term or an empty term list.
pub(crate) fn parse_cost_terms(
    rest: &[&str],
    mut i: usize,
    name: &str,
    cost: &mut CostDecl,
) -> Result<usize, String> {
    let mut saw_axis = false;
    while i < rest.len() {
        let Some((axis, class_word)) = rest[i].split_once('=') else {
            break;
        };
        // A top-level key ends the group: it is the next term
        // (`inputs=1`, `purity=pure`, …).
        if matches!(
            axis,
            "inputs" | "outputs" | "purity" | "partiality" | "field" | "determinism"
        ) {
            break;
        }
        let Some(class) = CostClass::from_spec_str(class_word) else {
            return Err(format!(
                "`#:contract {name}`: unknown cost class `{class_word}` (expected `const`/`linear`/`superlinear`/`unbounded`)"
            ));
        };
        match axis {
            "steps" => cost.steps = Some(class),
            "numeric" => cost.numeric = Some(class),
            "collection" => cost.collection = Some(class),
            other_axis => {
                return Err(format!(
                    "`#:contract {name}`: unknown cost axis `{other_axis}` (expected `steps`/`numeric`/`collection`)"
                ));
            }
        }
        saw_axis = true;
        i += 1;
    }
    if !saw_axis {
        return Err(format!(
            "`#:contract {name}`: `cost` with no `axis=class` term"
        ));
    }
    Ok(i)
}

/// Check every declared cost axis on `decl_cost` against `contract_cost`,
/// pushing a finding for each axis the inferred bound exceeds.
pub(crate) fn check_cost_decl(
    name: &str,
    decl_cost: CostDecl,
    contract_cost: CostBound,
    code: Option<&'static str>,
    findings: &mut Vec<DeclFinding>,
) {
    if let Some(declared) = decl_cost.steps {
        check_cost_axis(name, "steps", declared, contract_cost.steps, code, findings);
    }
    if let Some(declared) = decl_cost.numeric {
        check_cost_axis(
            name,
            "numeric",
            declared,
            contract_cost.numeric,
            code,
            findings,
        );
    }
    if let Some(declared) = decl_cost.collection {
        check_cost_axis(
            name,
            "collection",
            declared,
            contract_cost.collection,
            code,
            findings,
        );
    }
}

/// Check one `cost` axis (Step 5.5). Unlike inputs/outputs, purity, partiality and determinism, severity
/// here is driven by *this axis's own* `exact` bit, not the word's overall
/// `ContractConfidence` — `word_space`'s "never a false error" invariant
/// (`docs/dev/cost-contract-design.md` §3): a mismatch is only ever a proven
/// violation when the inferred class is provably attained.
fn check_cost_axis(
    name: &str,
    axis: &str,
    declared: CostClass,
    inferred: (CostClass, bool),
    code: Option<&'static str>,
    findings: &mut Vec<DeclFinding>,
) {
    let (inferred_class, exact) = inferred;
    if inferred_class > declared {
        findings.push(DeclFinding {
            severity: if exact {
                Severity::Error
            } else {
                Severity::Note
            },
            message: format!(
                "`#:contract {name}`: declared cost {axis}={} but inferred {}{}.",
                declared.as_spec_str(),
                inferred_class.as_spec_str(),
                if exact { "" } else { " (unverified)" }
            ),
            code: if exact { None } else { code },
        });
    }
}
