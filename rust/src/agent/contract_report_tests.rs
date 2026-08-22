//! Tests for Phase 1 of `docs/dev/cost-discoverability-work-order-2026-08.md`:
//! surfacing the inferred `cost` bound (`interpreter::word_cost`) through
//! `ajisai contract`'s JSON and `suggested` directive. Mirrors the exact-only
//! discipline `contract_report.rs` already applies to `space` — here per axis
//! rather than word-wide (§1.4 pitfall B/C/D).

#[cfg(test)]
mod contract_report_tests {
    use crate::agent::contract_report::{report_contracts, reports_json};

    fn suggested(source: &str, name: &str) -> String {
        report_contracts(source)
            .into_iter()
            .find(|r| r.name == name)
            .expect("word report")
            .suggested
    }

    #[test]
    fn cost_axes_are_reported_per_axis() {
        // A bare `RANGE` materializes a length set by its operand's *value*,
        // provably unbounded over input size (`word_cost_tests.rs`'s
        // `value_driven_materializer_is_unbounded_over_input_size`).
        let reports = report_contracts("{ RANGE } 'MK' DEF");
        let json = reports_json(&reports);
        let mk = json
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == "MK")
            .expect("MK report");
        assert_eq!(mk["cost"]["collection"], "unbounded");
    }

    #[test]
    fn suggested_includes_only_exact_axes() {
        // `JOIN` charges no numericWork (const, but only exact by the
        // `Const`-is-always-exact rule, pitfall D) and touches no other
        // collection beyond its own operand at a merely plausible
        // `superlinear` bound (never measured to attain it, so inexact).
        // `steps`/`numeric` belong in `suggested`; `collection` must not.
        let line = suggested("{ JOIN } 'J' DEF", "J");
        assert!(line.contains("steps=const"), "line was: {line}");
        assert!(line.contains("numeric=const"), "line was: {line}");
        assert!(
            !line.contains("collection="),
            "inexact axis leaked into suggested: {line}"
        );
    }

    #[test]
    fn suggested_omits_the_cost_keyword_when_no_axis_is_exact() {
        // A higher-order word is unbounded on every axis but never a proven
        // witness (`word_cost_tests.rs`'s
        // `higher_order_word_is_unbounded_on_every_axis_but_not_exact`).
        // Emitting a bare `cost` keyword with zero `axis=class` terms would
        // fail `contract_cost::parse_cost_terms` and break `suggested`'s only
        // purpose: pasting into source and passing `check --contract`.
        let line = suggested("{ [ 1 ] MAP } 'M' DEF", "M");
        assert!(
            !line.contains("cost"),
            "cost keyword must be omitted with no exact axis: {line}"
        );
    }

    #[test]
    fn suggested_round_trips_through_the_checker() {
        // The one test that matters most (§1.5 Step 1.4): every `suggested`
        // line this Phase produces, pasted back into its own source, must
        // pass `check --contract` cleanly — no violation, nothing left
        // unverifiable.
        let source = "{ ADD } 'A' DEF\n{ JOIN } 'J' DEF\n{ [ 1 ] MAP } 'M' DEF";
        let reports = report_contracts(source);
        let mut annotated = source.to_string();
        for r in &reports {
            annotated.push('\n');
            annotated.push_str(&r.suggested);
        }
        let response = crate::agent::api::check(&annotated, true);
        let json = response.to_json();
        let decls = &json["contractDecls"];
        assert_eq!(decls["outcome"], "value", "decls was: {decls}");
        assert_eq!(decls["gapSummary"]["violated"], 0);
        assert_eq!(decls["gapSummary"]["cannotVerify"], 0);
    }

    #[test]
    fn axis_order_is_stable() {
        // `ADD` is exact on all three axes (`word_cost_tests.rs`'s
        // `input_driven_arithmetic_is_exactly_linear_in_numeric_work`), so the
        // full `cost` term is always present — locking the required
        // steps → numeric → collection order in one literal string.
        let line = suggested("{ ADD } 'A' DEF", "A");
        assert!(
            line.contains("cost steps=const numeric=linear collection=const"),
            "line was: {line}"
        );
    }
}
