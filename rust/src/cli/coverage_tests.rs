//! Tests for the `ajisai coverage` aggregation (`cli/coverage.rs`).
//!
//! The counting rules under test are the memo definitions
//! (`docs/dev/capability-transition-measurement-design.md` §4,
//! TRANSITION_METRICS_VERSION = 1): denominator = word occurrences only
//! (no literals, no modifiers, no structural tokens), covered = resolves
//! to complete §7.14 contract metadata.

use super::coverage::{analyze, Coverage, OccurrenceKind};
use crate::interpreter::Interpreter;

fn coverage_of(source: &str) -> Coverage {
    let tokens = crate::tokenizer::tokenize(source).expect("test source must tokenize");
    let interp = Interpreter::new();
    analyze(&interp, &tokens)
}

fn kind_count(coverage: &Coverage, kind: OccurrenceKind) -> u64 {
    coverage
        .by_kind
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, n)| *n)
        .unwrap_or(0)
}

#[test]
fn literals_are_excluded_and_corewords_are_covered() {
    // `1 2 +`: two number literals (excluded) and one coreword occurrence.
    let coverage = coverage_of("1 2 +");
    assert_eq!(coverage.total, 1);
    assert_eq!(coverage.covered, 1);
    assert_eq!(kind_count(&coverage, OccurrenceKind::Core), 1);
    assert!(coverage.uncovered.is_empty());
}

#[test]
fn modifiers_are_excluded_from_the_denominator() {
    let coverage = coverage_of("[ 1 2 ] STAK KEEP +");
    assert_eq!(coverage.excluded_modifiers, 2);
    assert_eq!(coverage.total, 1);
    assert_eq!(coverage.covered, 1);
}

#[test]
fn nil_coalesce_is_structural_in_both_spellings() {
    // `^` and the spelled-out `VENT` both tokenize as NilCoalesce (SPEC §6.4),
    // so neither enters the count; only DIV does. The two spellings are the
    // same control directive and must be counted identically.
    let sugar = coverage_of("1 0 / ^ 99");
    assert_eq!(sugar.total, 1);
    assert_eq!(sugar.covered, 1);
    let spelled = coverage_of("1 0 / VENT 99");
    assert_eq!(spelled.total, 1);
    assert_eq!(spelled.covered, 1);
}

#[test]
fn def_defined_user_words_are_uncovered() {
    let coverage = coverage_of("{ 2 * } 'DOUBLE' DEF\n3 DOUBLE DOUBLE");
    // Counted occurrences: `*` (MUL), DEF, DOUBLE ×2.
    assert_eq!(coverage.total, 4);
    assert_eq!(coverage.covered, 2);
    assert_eq!(kind_count(&coverage, OccurrenceKind::UserDefined), 2);
    let uncovered = &coverage.uncovered;
    assert_eq!(uncovered.len(), 1);
    assert_eq!(uncovered[0].word, "DOUBLE");
    assert_eq!(uncovered[0].count, 2);
    assert_eq!(uncovered[0].kind, OccurrenceKind::UserDefined);
}

#[test]
fn imported_module_short_names_are_covered_as_module_words() {
    let coverage = coverage_of("'math' IMPORT 2 SQRT");
    // Counted: IMPORT, SQRT.
    assert_eq!(coverage.total, 2);
    assert_eq!(coverage.covered, 2);
    assert!(kind_count(&coverage, OccurrenceKind::Module) >= 1);
}

#[test]
fn qualified_module_words_are_covered() {
    let coverage = coverage_of("'math' IMPORT 2 3 MATH@POW");
    assert_eq!(coverage.covered, coverage.total);
    assert!(kind_count(&coverage, OccurrenceKind::Module) >= 1);
}

#[test]
fn user_dictionary_references_are_uncovered() {
    let coverage = coverage_of("MYDICT@FOO");
    assert_eq!(coverage.total, 1);
    assert_eq!(coverage.covered, 0);
    assert_eq!(kind_count(&coverage, OccurrenceKind::UserDictionary), 1);
}

#[test]
fn unknown_words_are_uncovered_not_an_error() {
    let coverage = coverage_of("1 FROBNICATE");
    assert_eq!(coverage.total, 1);
    assert_eq!(coverage.covered, 0);
    assert_eq!(kind_count(&coverage, OccurrenceKind::Unknown), 1);
    assert_eq!(coverage.uncovered[0].word, "FROBNICATE");
}

#[test]
fn empty_program_yields_zero_over_zero() {
    let coverage = coverage_of("1 2 [ 3 ] 'text'");
    assert_eq!(coverage.total, 0);
    assert_eq!(coverage.covered, 0);
    assert!(coverage.uncovered.is_empty());
}

#[test]
fn json_shape_carries_version_ratio_and_breakdown() {
    let coverage = coverage_of("{ 2 * } 'DOUBLE' DEF\n3 DOUBLE");
    let json = coverage.to_json();
    assert_eq!(
        json["transitionMetricsVersion"],
        super::coverage::TRANSITION_METRICS_VERSION
    );
    assert_eq!(json["covered"], 2);
    assert_eq!(json["total"], 3);
    assert_eq!(json["ratioDisplay"], "2/3");
    assert_eq!(json["breakdown"]["core"], 2);
    assert_eq!(json["breakdown"]["userDefined"], 1);
    assert_eq!(json["uncovered"][0]["word"], "DOUBLE");
    assert_eq!(json["uncovered"][0]["kind"], "userDefined");
    assert_eq!(json["uncovered"][0]["count"], 1);
}

#[test]
fn aliases_canonicalize_before_classification() {
    // `%` normalizes to MOD; `&` to AND — both covered corewords, and the
    // uncovered list reports canonical names, never surface aliases.
    let coverage = coverage_of("7 3 % TRUE &");
    assert_eq!(coverage.covered, coverage.total);
}
