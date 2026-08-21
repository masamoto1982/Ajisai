//! Step 3.5 and Step 4.4 of
//! `docs/dev/competitive-advantage-work-order-2026-08.md`: gap identifiers
//! for `check --contract`'s "cannot verify" result (Phase 3), and stating
//! that result in the runtime trichotomy's own vocabulary (Phase 4).

#[cfg(test)]
mod contract_decl_tests {
    use crate::agent::api::check;
    use serde_json::Value as Json;

    fn contract_decls(source: &str) -> Json {
        check(source, true).to_json()["contractDecls"].clone()
    }

    fn exit_code(source: &str) -> i32 {
        check(source, true).exit_code()
    }

    #[test]
    fn recursive_word_reports_recursive_gap() {
        let source = "{ 1 SUB REC } 'REC' DEF\n#:contract REC ( 1 -- 1 ) pure nil-free";
        let decls = contract_decls(source);
        let findings = decls["findings"].as_array().expect("findings array");
        assert!(!findings.is_empty(), "expected at least one finding");
        for finding in findings {
            assert_eq!(finding["severity"], "note");
            assert_eq!(finding["code"], "gap.recursiveDependency");
        }
        assert_eq!(decls["gapSummary"]["cannotVerify"], 1);
        assert_eq!(decls["gapSummary"]["byGap"]["gap.recursiveDependency"], 3);
    }

    #[test]
    fn unresolved_word_reports_unresolved_gap() {
        // INNER is defined by a nested (non-top-level) DEF, so the
        // definitions pass that builds the check environment never
        // registers it — CALLER's own inference cannot resolve it.
        let source = "{ { 1 } 'INNER' DEF } 'OUTER' DEF\n{ INNER } 'CALLER' DEF\n#:contract CALLER ( 0 -- 1 ) pure nil-free";
        let decls = contract_decls(source);
        let findings = decls["findings"].as_array().expect("findings array");
        assert!(!findings.is_empty(), "expected at least one finding");
        for finding in findings {
            assert_eq!(finding["severity"], "note");
            assert_eq!(finding["code"], "gap.unresolvedWord");
        }
        assert_eq!(decls["gapSummary"]["cannotVerify"], 1);
        assert_eq!(decls["gapSummary"]["byGap"]["gap.unresolvedWord"], 1);
    }

    #[test]
    fn violated_declaration_has_no_gap_code() {
        // Inference is complete here (no recursion, no unresolved symbol), so
        // a mismatch is a proven violation, not a gap.
        let source = "{ 1 PRINT } 'F' DEF\n#:contract F ( 1 -- 0 ) pure";
        let decls = contract_decls(source);
        let findings = decls["findings"].as_array().expect("findings array");
        assert!(!findings.is_empty(), "expected at least one finding");
        for finding in findings {
            assert_eq!(finding["severity"], "error");
            assert_eq!(finding["code"], Json::Null);
        }
        assert_eq!(decls["gapSummary"]["violated"], 1);
        assert_eq!(decls["gapSummary"]["cannotVerify"], 0);
        assert_eq!(decls["gapSummary"]["byGap"].as_object().unwrap().len(), 0);
    }

    #[test]
    fn gap_summary_counts_add_up() {
        let source = "{ 1 SUB REC } 'REC' DEF
{ 1 PRINT } 'BAD' DEF
{ 1 SUB } 'GOOD' DEF
#:contract REC ( 1 -- 1 ) pure nil-free
#:contract BAD ( 1 -- 0 ) pure
#:contract GOOD ( 1 -- 1 ) pure nil-free";
        let decls = contract_decls(source);
        let summary = &decls["gapSummary"];
        let checked = summary["declarationsChecked"].as_i64().unwrap();
        let verified = summary["verified"].as_i64().unwrap();
        let cannot_verify = summary["cannotVerify"].as_i64().unwrap();
        let violated = summary["violated"].as_i64().unwrap();
        assert_eq!(checked, 3);
        assert_eq!(verified + cannot_verify + violated, checked);
        assert_eq!(verified, 1);
        assert_eq!(cannot_verify, 1);
        assert_eq!(violated, 1);
    }

    #[test]
    fn gap_summary_key_order_is_stable() {
        let source = "{ 1 SUB REC } 'REC' DEF
{ { 1 } 'INNER' DEF } 'OUTER' DEF
{ INNER } 'CALLER' DEF
#:contract REC ( 1 -- 1 ) pure nil-free
#:contract CALLER ( 0 -- 1 ) pure nil-free";
        let first = serde_json::to_string(&contract_decls(source)).unwrap();
        let second = serde_json::to_string(&contract_decls(source)).unwrap();
        assert_eq!(
            first, second,
            "two runs over identical source must render identically"
        );

        let by_gap_start = first.find("\"byGap\":{").expect("byGap object present");
        let by_gap_end = by_gap_start + first[by_gap_start..].find('}').unwrap();
        let by_gap = &first[by_gap_start..by_gap_end];
        let recursive_pos = by_gap
            .find("gap.recursiveDependency")
            .expect("gap.recursiveDependency present in byGap");
        let unresolved_pos = by_gap
            .find("gap.unresolvedWord")
            .expect("gap.unresolvedWord present in byGap");
        assert!(
            recursive_pos < unresolved_pos,
            "byGap keys are not in ascending order: {by_gap}"
        );
    }

    #[test]
    fn verified_declaration_is_a_value() {
        let source = "{ 1 SUB } 'GOOD' DEF\n#:contract GOOD ( 1 -- 1 ) pure nil-free";
        let decls = contract_decls(source);
        assert_eq!(decls["outcome"], "value");
        assert_eq!(decls["declarations"][0]["word"], "GOOD");
        assert_eq!(decls["declarations"][0]["outcome"], "value");
        assert!(decls["declarations"][0].get("reason").is_none());
        assert!(decls["declarations"][0].get("category").is_none());
    }

    #[test]
    fn unverifiable_declaration_is_a_nil_with_a_reason() {
        let source = "{ 1 SUB REC } 'REC' DEF\n#:contract REC ( 1 -- 1 ) pure nil-free";
        let decls = contract_decls(source);
        assert_eq!(decls["outcome"], "nil");
        assert_eq!(decls["declarations"][0]["word"], "REC");
        assert_eq!(decls["declarations"][0]["outcome"], "nil");
        assert_eq!(
            decls["declarations"][0]["reason"],
            "gap.recursiveDependency"
        );
    }

    #[test]
    fn violated_declaration_is_an_error() {
        let source = "{ 1 PRINT } 'F' DEF\n#:contract F ( 1 -- 0 ) pure";
        let decls = contract_decls(source);
        assert_eq!(decls["outcome"], "error");
        assert_eq!(decls["declarations"][0]["word"], "F");
        assert_eq!(decls["declarations"][0]["outcome"], "error");
        assert_eq!(decls["declarations"][0]["category"], "contractViolation");
    }

    #[test]
    fn error_dominates_nil_in_the_fold() {
        // REC is unverifiable (nil); F is a proven violation (error).
        let source = "{ 1 SUB REC } 'REC' DEF
{ 1 PRINT } 'F' DEF
#:contract REC ( 1 -- 1 ) pure nil-free
#:contract F ( 1 -- 0 ) pure";
        let decls = contract_decls(source);
        assert_eq!(decls["outcome"], "error");
    }

    #[test]
    fn nil_dominates_value_in_the_fold() {
        // REC is unverifiable (nil); GOOD verifies cleanly (value).
        let source = "{ 1 SUB REC } 'REC' DEF
{ 1 SUB } 'GOOD' DEF
#:contract REC ( 1 -- 1 ) pure nil-free
#:contract GOOD ( 1 -- 1 ) pure nil-free";
        let decls = contract_decls(source);
        assert_eq!(decls["outcome"], "nil");
    }

    #[test]
    fn empty_declarations_fold_to_value() {
        let source = "{ 1 SUB } 'GOOD' DEF";
        let decls = contract_decls(source);
        assert_eq!(decls["outcome"], "value");
        assert_eq!(decls["declarations"].as_array().unwrap().len(), 0);
    }

    /// The most important test in this phase: `nil` (cannot verify) must
    /// never fail the check, and the new `outcome` field must not change
    /// that — only a proven `error` does.
    #[test]
    fn outcome_does_not_change_the_exit_code() {
        let nil_only = "{ 1 SUB REC } 'REC' DEF\n#:contract REC ( 1 -- 1 ) pure nil-free";
        assert_eq!(exit_code(nil_only), 0, "cannot-verify must not fail check");

        let with_error = "{ 1 PRINT } 'F' DEF\n#:contract F ( 1 -- 0 ) pure";
        assert_eq!(
            exit_code(with_error),
            1,
            "a proven violation must fail check"
        );
    }

    // Phase 5 (Step 5.6): the `cost` declaration axis.

    #[test]
    fn const_word_verifies_const_cost() {
        let source =
            "{ 1 2 ADD } 'S' DEF\n#:contract S cost steps=const numeric=linear collection=const";
        let decls = contract_decls(source);
        assert_eq!(decls["outcome"], "value");
        assert_eq!(decls["findings"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn exact_mismatch_is_a_cost_error() {
        // ADD's numeric axis is provably linear (the meter's own reason to
        // exist), so declaring a tighter `const` bound is a proven violation.
        let source = "{ 1 2 ADD } 'S' DEF\n#:contract S cost numeric=const";
        let decls = contract_decls(source);
        let findings = decls["findings"].as_array().expect("findings array");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0]["severity"], "error");
        assert_eq!(findings[0]["code"], Json::Null);
        assert_eq!(decls["outcome"], "error");
    }

    #[test]
    fn inexact_mismatch_is_a_cost_note() {
        // MAP's steps axis is a sound but unproven `unbounded` upper bound
        // (the body could be trivial), so a tighter declaration is only ever
        // unverifiable, never a false error.
        let source = "{ [ 1 ] MAP } 'M' DEF\n#:contract M cost steps=const";
        let decls = contract_decls(source);
        let findings = decls["findings"].as_array().expect("findings array");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0]["severity"], "note");
        assert_eq!(decls["outcome"], "nil");
    }

    #[test]
    fn omitted_cost_axis_is_not_checked() {
        // Only `steps` is declared; MAP's numeric and collection axes are
        // also mismatched against a tighter class but must not be checked.
        let source = "{ [ 1 ] MAP } 'M' DEF\n#:contract M cost steps=const";
        let decls = contract_decls(source);
        let findings = decls["findings"].as_array().expect("findings array");
        assert_eq!(findings.len(), 1, "only the declared axis is checked");
        assert!(findings[0]["message"].as_str().unwrap().contains("steps"));
    }

    #[test]
    fn unknown_cost_class_is_a_parse_error() {
        let source = "{ 1 2 ADD } 'S' DEF\n#:contract S cost numeric=quadratic";
        let decls = contract_decls(source);
        assert_eq!(decls["violated"], true);
        let findings = decls["findings"].as_array().expect("findings array");
        assert!(findings.iter().any(|f| f["message"]
            .as_str()
            .unwrap()
            .contains("unknown cost class")));
    }

    #[test]
    fn unknown_cost_axis_is_a_parse_error() {
        let source = "{ 1 2 ADD } 'S' DEF\n#:contract S cost bogus=const";
        let decls = contract_decls(source);
        assert_eq!(decls["violated"], true);
        let findings = decls["findings"].as_array().expect("findings array");
        assert!(findings
            .iter()
            .any(|f| f["message"].as_str().unwrap().contains("unknown cost axis")));
    }

    #[test]
    fn legacy_fields_still_present() {
        let source = "{ 1 PRINT } 'F' DEF\n#:contract F ( 1 -- 0 ) pure";
        let decls = contract_decls(source);
        assert_eq!(decls["violated"], true);
        let findings = decls["findings"].as_array().expect("findings array");
        assert!(!findings.is_empty());
        for finding in findings {
            assert!(finding.get("severity").is_some());
            assert!(finding.get("message").is_some());
        }
    }
}
