//! Tests for the light contract / flow-mass plan check (`super::plan_check`).
//! Pins the mass over-consumption verdict and the NIL-flow advisories against
//! the existing §13.1 validator and §7.14 `nil_policy` contracts. Design note:
//! `docs/dev/natural-language-surface-design.md` §4.

use super::explain::Lang;
use super::plan_check::{check_plan, Severity};
use crate::interpreter::Interpreter;

fn check(src: &str) -> super::plan_check::PlanCheck {
    let interp = Interpreter::new();
    check_plan(&interp, src).expect("well-formed source must tokenize")
}

#[test]
fn clean_plan_has_no_findings() {
    let result = check("[ 1 ] [ 2 ] +");
    assert!(!result.over_consumes);
    assert!(result.may_bubble.is_empty());
    assert!(result.findings(Lang::Ja).is_empty());
}

#[test]
fn over_consuming_plan_is_an_error() {
    // `+` reads two operands from an empty stack: a malformed plan.
    let result = check("+");
    assert!(result.over_consumes);
    assert!(result.min_depth < 0);
    let findings = result.findings(Lang::Ja);
    assert_eq!(findings[0].severity, Severity::Error);
}

#[test]
fn nil_source_without_fallback_is_advisory() {
    // DIV is nil_policy=CreatesNil; with no `^` the plan can bubble to NIL.
    let result = check("[ 1 ] [ 0 ] DIV");
    assert_eq!(result.may_bubble, vec!["DIV".to_string()]);
    assert!(!result.has_fallback);
    let advisory = result
        .findings(Lang::Ja)
        .into_iter()
        .find(|finding| finding.severity == Severity::Advisory);
    assert!(
        advisory.is_some(),
        "a CreatesNil word with no `^` must advise"
    );
}

#[test]
fn nil_source_with_vent_drops_the_advisory() {
    // The same flow with a `^` (VENT) fallback: DIV still can bubble, but the
    // fallback is present, so the unguarded-NIL advisory is not raised.
    let result = check("[ 1 ] [ 0 ] DIV ^ [ 99 ]");
    assert!(result.has_fallback);
    let advisory = result
        .findings(Lang::Ja)
        .into_iter()
        .find(|finding| finding.severity == Severity::Advisory && finding.message.contains("NIL"));
    assert!(
        advisory.is_none(),
        "a present `^` must suppress the unguarded-NIL advisory"
    );
}

#[test]
fn english_findings_are_ascii() {
    let result = check("+");
    for finding in result.findings(Lang::En) {
        assert!(
            finding.message.is_ascii(),
            "English finding must be ASCII: {}",
            finding.message
        );
    }
}
