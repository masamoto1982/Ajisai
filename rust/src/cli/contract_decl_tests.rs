//! Tests for opt-in `#:contract` declaration checking (`cli/contract_decl.rs`).

use super::contract_decl::{check_contract_decls, parse_contract_directives};
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
