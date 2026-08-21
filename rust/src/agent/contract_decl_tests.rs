//! Step 3.5 of `docs/dev/competitive-advantage-work-order-2026-08.md`: gap
//! identifiers for `check --contract`'s "cannot verify" result.

#[cfg(test)]
mod contract_decl_tests {
    use crate::agent::api::check;
    use serde_json::Value as Json;

    fn contract_decls(source: &str) -> Json {
        check(source, true).to_json()["contractDecls"].clone()
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
}
