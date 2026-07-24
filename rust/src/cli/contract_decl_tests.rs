//! Tests for opt-in `#:contract` declaration checking (`cli/contract_decl.rs`).

use super::contract_decl::{check_contract_decls, parse_contract_directives};
use super::contract_linearity::Linearity;
use super::contract_space::SpaceClass;
use super::explain::Lang;
use super::plan_check::Severity;
use crate::interpreter::word_contract::ContractPurity;

fn errors(source: &str) -> Vec<String> {
    check_contract_decls(source, Lang::En)
        .findings
        .into_iter()
        .filter(|f| f.severity == Severity::Error)
        .map(|f| f.message)
        .collect()
}

fn is_clean(source: &str) -> bool {
    let check = check_contract_decls(source, Lang::En);
    !check.violated
}

#[test]
fn parses_a_full_declaration() {
    let (decls, errs) = parse_contract_directives("#:contract INC ( 1 -- 1 ) pure nil-free\n");
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    assert_eq!(decls.len(), 1);
    let d = &decls[0];
    assert_eq!(d.name, "INC");
    assert_eq!(d.arity, Some((1, 1)));
    assert_eq!(d.purity, Some(ContractPurity::Pure));
    assert_eq!(d.nil_free, Some(true));
}

#[test]
fn parses_partial_declarations_and_zero_counts() {
    let (decls, errs) = parse_contract_directives("#:contract W ( 0 -- 2 ) may-nil\n");
    assert!(errs.is_empty());
    assert_eq!(decls[0].arity, Some((0, 2)));
    assert_eq!(decls[0].nil_free, Some(false));
    assert_eq!(decls[0].purity, None);
}

#[test]
fn parses_linearity_terms() {
    for (term, expected) in [
        ("linear", Linearity::Linear),
        ("affine", Linearity::Affine),
        ("droppable", Linearity::Droppable),
    ] {
        let src = format!("#:contract H ( 0 -- 1 ) effectful {term}\n");
        let (decls, errs) = parse_contract_directives(&src);
        assert!(errs.is_empty(), "unexpected errors for `{term}`: {errs:?}");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].linearity, Some(expected), "term `{term}`");
        // The other axes still parse alongside linearity.
        assert_eq!(decls[0].arity, Some((0, 1)));
    }
}

#[test]
fn linearity_is_optional_and_defaults_to_none() {
    let (decls, errs) = parse_contract_directives("#:contract W ( 1 -- 1 ) pure\n");
    assert!(errs.is_empty());
    assert_eq!(decls[0].linearity, None);
}

#[test]
fn declared_linearity_surfaces_as_a_note_never_an_error() {
    // Increment 1 records the axis without inference, so a declared linearity
    // must never produce an `error` finding (which would break `check`'s exit
    // code) — only an informational note.
    let src = "#:contract MAKE ( 0 -- 1 ) affine\n{ [ 42 ] } 'MAKE' DEF\n";
    let check = check_contract_decls(src, Lang::En);
    assert!(
        !check.violated,
        "a recorded linearity must not violate the contract check"
    );
    assert!(
        check
            .findings
            .iter()
            .any(|f| f.severity == Severity::Note && f.message.contains("affine")),
        "the declared linearity should be surfaced as a note: {:?}",
        check.findings.iter().map(|f| &f.message).collect::<Vec<_>>()
    );
}

#[test]
fn keep_on_a_handle_discharger_violates_linear() {
    // KEEP on KILL retains the handle after its one permitted consumption.
    let errs = errors("#:contract LEAKY linear\n{ KEEP KILL } 'LEAKY' DEF\n");
    assert_eq!(errs.len(), 1, "expected one linearity error, got: {errs:?}");
    assert!(errs[0].contains("KEEP") && errs[0].contains("linear"));
}

#[test]
fn keep_on_a_handle_discharger_violates_affine_too() {
    let errs = errors("#:contract LEAKY affine\n{ KEEP AWAIT } 'LEAKY' DEF\n");
    assert_eq!(errs.len(), 1, "expected one linearity error, got: {errs:?}");
}

#[test]
fn keep_on_an_observer_is_not_a_violation() {
    // STATUS/MONITOR read a handle without consuming it, so KEEP is the correct
    // idiom and must never be flagged.
    assert!(is_clean("#:contract PEEK linear\n{ KEEP STATUS } 'PEEK' DEF\n"));
}

#[test]
fn eat_on_a_discharger_is_not_a_violation() {
    // The proper consume: EAT on KILL discharges the handle exactly once.
    assert!(is_clean("#:contract CLEAN linear\n{ EAT KILL } 'CLEAN' DEF\n"));
}

#[test]
fn droppable_opts_out_of_the_discipline() {
    // Even a KEEP-on-discharge body is only a note under `droppable`.
    let check = check_contract_decls("#:contract LOOSE droppable\n{ KEEP KILL } 'LOOSE' DEF\n", Lang::En);
    assert!(!check.violated, "droppable must not raise an error");
    assert!(check
        .findings
        .iter()
        .any(|f| f.severity == Severity::Note && f.message.contains("droppable")));
}

#[test]
fn parses_space_class_terms() {
    for (term, expected) in [
        ("space:const", SpaceClass::Const),
        ("space:linear", SpaceClass::Linear),
        ("space:superlinear", SpaceClass::Superlinear),
        ("space:unbounded", SpaceClass::Unbounded),
    ] {
        let src = format!("#:contract W ( 1 -- 1 ) pure {term}\n");
        let (decls, errs) = parse_contract_directives(&src);
        assert!(errs.is_empty(), "unexpected errors for `{term}`: {errs:?}");
        assert_eq!(decls.len(), 1);
        assert_eq!(decls[0].space, Some(expected), "term `{term}`");
        // The other axes still parse alongside the space class.
        assert_eq!(decls[0].arity, Some((1, 1)));
        assert_eq!(decls[0].purity, Some(ContractPurity::Pure));
    }
}

#[test]
fn space_class_coexists_with_linearity_without_collision() {
    // `space:linear` must not be confused with the bare `linear` linearity term.
    let (decls, errs) = parse_contract_directives("#:contract W linear space:linear\n");
    assert!(errs.is_empty(), "unexpected errors: {errs:?}");
    assert_eq!(decls[0].linearity, Some(Linearity::Linear));
    assert_eq!(decls[0].space, Some(SpaceClass::Linear));
}

#[test]
fn space_is_optional_and_defaults_to_none() {
    let (decls, errs) = parse_contract_directives("#:contract W ( 1 -- 1 ) pure\n");
    assert!(errs.is_empty());
    assert_eq!(decls[0].space, None);
}

#[test]
fn a_looser_space_declaration_is_verified_never_an_error() {
    // Declaring a looser class than the inferred bound (here `space:unbounded`
    // over a const word) over-approximates the truth, so it holds: a verified
    // note, never an error.
    let src = "#:contract MAKE ( 0 -- 1 ) space:unbounded\n{ [ 42 ] } 'MAKE' DEF\n";
    let check = check_contract_decls(src, Lang::En);
    assert!(
        !check.violated,
        "a looser space class must not violate the contract check"
    );
    assert!(
        check
            .findings
            .iter()
            .any(|f| f.severity == Severity::Note && f.message.contains("space:unbounded")),
        "the declared space class should be surfaced as a note: {:?}",
        check
            .findings
            .iter()
            .map(|f| &f.message)
            .collect::<Vec<_>>()
    );
}

#[test]
fn declared_const_over_a_literal_range_is_verified() {
    // `[ 0 10 ] RANGE` materializes a compile-time-fixed length: a `space:const`
    // declaration is proved to hold.
    let src = "#:contract SEQ space:const\n{ [ 0 10 ] RANGE } 'SEQ' DEF\n";
    assert!(is_clean(src), "a literal-driven RANGE is provably const");
}

#[test]
fn declared_const_over_an_input_driven_range_is_an_error() {
    // A bare `RANGE` materializes a value-driven length: declaring `space:const`
    // is a provable violation (the inference has an exact unbounded witness).
    let src = "#:contract SEQ space:const\n{ RANGE } 'SEQ' DEF\n";
    let errs = errors(src);
    assert!(
        errs.iter().any(|m| m.contains("space:const")
            && m.contains("space:unbounded")
            && m.contains("violation")),
        "expected a space-contract violation error, got: {errs:?}"
    );
}

#[test]
fn declared_const_over_an_elementwise_linear_word_is_an_error() {
    // `[ 1 ] ADD` broadcasts against an input vector: worst-case linear, and
    // provably so. Declaring `space:const` is a violation.
    let src = "#:contract INC space:const\n{ [ 1 ] ADD } 'INC' DEF\n";
    let errs = errors(src);
    assert!(
        errs.iter().any(|m| m.contains("space:const") && m.contains("violation")),
        "expected a space-contract violation error, got: {errs:?}"
    );
}

#[test]
fn an_unprovable_tighter_declaration_is_a_note_not_an_error() {
    // A higher-order word's inferred bound is an *unproven* upper bound
    // (`Unbounded`, inexact): a tighter `space:linear` declaration might still
    // hold, so the checker abstains with a note rather than a false error.
    let src = "#:contract M space:linear\n{ [ 1 ] MAP } 'M' DEF\n";
    let check = check_contract_decls(src, Lang::En);
    assert!(
        !check.violated,
        "an unprovable space bound must never raise a false error"
    );
    assert!(
        check
            .findings
            .iter()
            .any(|f| f.severity == Severity::Note
                && f.message.contains("cannot verify")
                && f.message.contains("space:linear")),
        "expected a cannot-verify note: {:?}",
        check.findings.iter().map(|f| &f.message).collect::<Vec<_>>()
    );
}

#[test]
fn reports_malformed_directives() {
    assert!(!parse_contract_directives("#:contract\n").1.is_empty());
    assert!(
        !parse_contract_directives("#:contract W ( 1 1 )\n")
            .1
            .is_empty(),
        "missing -- must be reported"
    );
    assert!(
        !parse_contract_directives("#:contract W bogus\n")
            .1
            .is_empty(),
        "unknown term must be reported"
    );
    assert!(
        !parse_contract_directives("#:contract W ( 1 -- x )\n")
            .1
            .is_empty(),
        "non-integer count must be reported"
    );
}

#[test]
fn a_correct_declaration_passes_clean() {
    // INC: ( 1 -- 1 ) pure, propagates NIL (nil-free: does not manufacture it).
    assert!(is_clean(
        "#:contract INC ( 1 -- 1 ) pure nil-free\n{ [ 1 ] ADD } 'INC' DEF\n"
    ));
}

#[test]
fn wrong_arity_is_an_error() {
    let errs = errors("#:contract INC ( 2 -- 1 ) pure\n{ [ 1 ] ADD } 'INC' DEF\n");
    assert!(
        errs.iter().any(|m| m.contains("arity")),
        "expected an arity error, got {errs:?}"
    );
}

#[test]
fn declaring_pure_on_an_effectful_word_is_an_error() {
    let errs = errors("#:contract SAY pure\n{ PRINT } 'SAY' DEF\n");
    assert!(
        errs.iter()
            .any(|m| m.contains("pure") && m.contains("effectful")),
        "expected a purity error, got {errs:?}"
    );
}

#[test]
fn declaring_nil_free_on_a_nil_creating_word_is_an_error() {
    // DIV is a CreatesNil word (division by zero bubbles to NIL).
    let errs = errors("#:contract SAFE nil-free\n{ DIV } 'SAFE' DEF\n");
    assert!(
        errs.iter().any(|m| m.contains("nil-free")),
        "expected a nil-free error, got {errs:?}"
    );
}

#[test]
fn may_nil_documents_intent_without_erroring() {
    // A NIL-creating word declared may-nil is consistent — no error.
    assert!(is_clean(
        "#:contract SAFE ( 2 -- 1 ) may-nil\n{ DIV } 'SAFE' DEF\n"
    ));
}

#[test]
fn an_unknown_declared_word_is_an_error() {
    let errs = errors("#:contract GHOST ( 1 -- 1 )\n{ [ 1 ] ADD } 'INC' DEF\n");
    assert!(
        errs.iter()
            .any(|m| m.contains("GHOST") && m.contains("no such word")),
        "expected an unknown-word error, got {errs:?}"
    );
}

#[test]
fn conservative_inference_downgrades_a_mismatch_to_a_note() {
    // A directly recursive word infers a conservative, dynamic contract, so a
    // fixed-arity declaration cannot be *disproven* — it is a note, not an error.
    let check = check_contract_decls(
        "#:contract REC ( 1 -- 1 ) pure\n{ REC } 'REC' DEF\n",
        Lang::En,
    );
    assert!(
        !check.violated,
        "conservative mismatch must not be an error"
    );
    assert!(
        check.findings.iter().any(|f| f.severity == Severity::Note),
        "expected a note for the unverifiable declaration"
    );
}

#[test]
fn no_directives_means_no_findings() {
    let check = check_contract_decls("{ [ 1 ] ADD } 'INC' DEF\n", Lang::En);
    assert!(check.findings.is_empty());
    assert!(!check.violated);
}

#[test]
fn checks_multiple_words_independently() {
    let source = "#:contract INC ( 1 -- 1 ) pure nil-free\n\
                  #:contract SAY effectful\n\
                  { [ 1 ] ADD } 'INC' DEF\n\
                  { PRINT } 'SAY' DEF\n";
    // INC is correct; SAY is correctly declared effectful — both clean.
    assert!(is_clean(source));
}
