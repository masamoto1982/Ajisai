//! The `#:contract` check as every host operation that would *run* a program
//! sees it (LANG.CONTRACT.CHECK is a pre-execution check): whether a source
//! opted in, and the error report a violated declaration answers with.
//!
//! `check` has verified declarations since the directive existed, and
//! `compute` did not: a program that declared `inputs=2` for a Word whose body
//! took one ran, answered a value, and left no trace of the lie anywhere in
//! its envelope — while the README promised a check "before anything runs".
//! Both now run the same check first, and both answer a violation in the one
//! shape every other error has.

use super::contract_decl::{check_contract_decls, ContractDeclCheck, DeclFinding, Severity};
use super::contract_gap::CheckOutcome;
use super::report::Report;
use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::{DebugDiagnosis, ErrorLocusKind, ErrorPhase};
use crate::interpreter::Interpreter;

/// Whether `source` carries any `#:contract` directive — well-formed or not —
/// decided from the text alone, so `compute` and `outcomes` can tell an
/// opted-in program apart from every other one without inferring anything.
pub(crate) fn declares_contracts(source: &str) -> bool {
    source
        .lines()
        .any(|line| line.trim_start().starts_with("#:contract"))
}

/// The declaration check for a program that opted in, or `None` for one that
/// wrote no directive: `compute` attaches `contractDecls` exactly when there
/// is a declaration to report on.
pub(crate) fn declared_contract_check(source: &str) -> Option<ContractDeclCheck> {
    declares_contracts(source).then(|| check_contract_decls(source))
}

/// The error report for a violated declaration: the one shape every other
/// error has (LANG.FAILURE.TRICHOTOMY), with the declaration findings beside
/// it as `contractDecls`.
///
/// `check` and `compute` both answer with this. `check` used to answer a
/// violation with `status: error` and nothing else at the top level — no
/// `message`, no `diagnosis`, no `aiDiagnostic` — so a reader following the
/// documented order ("on error, read `diagnosis.why`") found nothing, and
/// the information sat only in `contractDecls.findings[].message`.
/// `source` is `Some` for `compute`, which receipts the refusal as it would
/// any error, and `None` for `check`, which never executes.
pub(crate) fn violation_report(
    interp: &Interpreter,
    check: &ContractDeclCheck,
    source: Option<&str>,
) -> Report {
    let violations: Vec<&DeclFinding> = check
        .findings
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .collect();
    // The Word the first violated declaration names, when it names one that
    // is defined: the locus of the failure. A malformed directive or a
    // declaration for an undefined Word has no Word to point at.
    let word = check
        .decl_outcomes
        .iter()
        .find(|(_, outcome)| matches!(outcome, CheckOutcome::Error))
        .map(|(word, _)| word.clone());
    let message = format!(
        "Contract declaration violated: {}",
        violations
            .iter()
            .map(|finding| finding.message.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );
    let category = ErrorCategory::ContractViolation;
    let mut diagnosis = DebugDiagnosis::from_error_category(
        ErrorPhase::CheckContract,
        word.as_deref(),
        Some(&category),
        None,
        0,
        0,
        Some(message.clone()),
    );
    if word.is_some() {
        // The registry cannot know a User Word; the declaration names one.
        diagnosis.where_.kind = ErrorLocusKind::UserWord;
    }
    for finding in &violations {
        diagnosis
            .evidence
            .push(format!("violation={}", finding.message));
    }
    let mut report = super::error_report(
        interp,
        &diagnosis,
        Some(&category),
        message,
        Vec::new(),
        Vec::new(),
        source,
    );
    report.contract_decls = Some(check.to_json());
    report
}
