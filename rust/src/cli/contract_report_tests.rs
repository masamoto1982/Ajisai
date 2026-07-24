//! Tests for `ajisai contract` inferred-contract reporting (`contract_report.rs`).

use super::contract_report::{report_contracts, reports_json};

fn report_for(source: &str, name: &str) -> super::contract_report::WordReport {
    report_contracts(source)
        .into_iter()
        .find(|r| r.name == name)
        .unwrap_or_else(|| panic!("no report for {name}"))
}

#[test]
fn reports_a_pure_arithmetic_word() {
    let r = report_for("{ [ 1 ] ADD } 'INC' DEF\n", "INC");
    assert_eq!(r.arity, "( 1 -- 1 )");
    assert_eq!(r.purity, "pure");
    assert_eq!(r.determinism, "deterministic");
    assert_eq!(r.space, "space:linear");
    assert_eq!(r.confidence, "complete");
}

#[test]
fn reports_the_inferred_space_class_and_suggests_it_when_proven() {
    // A literal-driven RANGE is provably const, so the report both names the
    // class and codifies it in the suggested directive.
    let r = report_for("{ [ 0 10 ] RANGE } 'SEQ' DEF\n", "SEQ");
    assert_eq!(r.space, "space:const");
    assert!(
        r.suggested.contains("space:const"),
        "a proven space class should be suggested: {}",
        r.suggested
    );
}

#[test]
fn suggests_a_paste_ready_directive_that_the_checker_accepts() {
    let source = "{ [ 1 ] ADD } 'INC' DEF\n";
    let r = report_for(source, "INC");
    // The suggested directive is exactly a `#:contract` line for INC.
    assert!(
        r.suggested.starts_with("#:contract INC"),
        "unexpected suggestion: {}",
        r.suggested
    );
    // Feed the suggestion back through the checker: it must accept its own word.
    let round_trip = format!("{}\n{}", r.suggested, source);
    let check = super::contract_decl::check_contract_decls(&round_trip, super::explain::Lang::En);
    assert!(
        !check.violated,
        "the suggested directive must pass the checker, got {:?}",
        check
            .findings
            .iter()
            .map(|f| &f.message)
            .collect::<Vec<_>>()
    );
}

#[test]
fn reports_an_effectful_word_with_effects() {
    let r = report_for("{ PRINT } 'SAY' DEF\n", "SAY");
    assert_eq!(r.purity, "effectful");
    assert!(
        !r.effects.is_empty(),
        "an effectful word should list at least one effect"
    );
}

#[test]
fn a_recursive_word_is_reported_conservative() {
    let r = report_for("{ REC } 'REC' DEF\n", "REC");
    assert_eq!(r.confidence, "conservative");
    assert_eq!(r.arity, "dynamic");
}

#[test]
fn reports_every_defined_word_in_order() {
    let source = "{ [ 1 ] ADD } 'A' DEF\n{ [ 2 ] ADD } 'B' DEF\n";
    let reports = report_contracts(source);
    let names: Vec<_> = reports.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["A", "B"]);
}

#[test]
fn json_carries_the_rendered_fields() {
    let reports = report_contracts("{ [ 1 ] ADD } 'INC' DEF\n");
    let json = reports_json(&reports);
    let first = &json.as_array().unwrap()[0];
    assert_eq!(first["name"], "INC");
    assert_eq!(first["purity"], "pure");
    assert!(first["suggested"]
        .as_str()
        .unwrap()
        .contains("#:contract INC"));
}

#[test]
fn no_user_words_yields_an_empty_report() {
    assert!(report_contracts("1 2 ADD\n").is_empty());
}
