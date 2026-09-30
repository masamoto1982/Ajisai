//! `ajisai contract <file>` — report each user word's **inferred** contract
//! (`crate::interpreter::word_contract`), execution-free. The reporting
//! companion to the `#:contract` declaration checker (`cli::contract_decl`): it
//! surfaces what the inference engine derives so a user can discover a contract
//! and codify it. Each report also carries a paste-ready `#:contract` directive
//! (`suggested`) for exactly the properties the checker verifies, closing the
//! loop report → declare → `check --contract`.
//!
//! Definitions and imports are registered without running any word body or
//! top-level code (shared with the checker via `build_definitions_interpreter`),
//! so this never executes the program.

use super::contract_decl::build_definitions_interpreter;
use crate::interpreter::word_contract::{ContractFlow, WordContract};
use crate::interpreter::word_cost::CostClass;

/// One user word's inferred contract, under the keys and in the vocabulary of
/// a contract Record (`CONTRACT`, `spec/words.json`): a report, the Record
/// `CONTRACT` answers for the same word, and a `#:contract` declaration all
/// spell one fact one way.
pub(crate) struct WordReport {
    pub name: String,
    /// `inputs` and `outputs`: a count, or `None` for `variable`.
    pub inputs: Option<u16>,
    pub outputs: Option<u16>,
    pub partiality: &'static str,
    pub purity: &'static str,
    pub determinism: &'static str,
    /// The inferred charged-cost class on each of the three axes
    /// (`"const"` … `"unbounded"`).
    pub cost_steps: &'static str,
    pub cost_numeric: &'static str,
    pub cost_collection: &'static str,
    pub effects: Vec<String>,
    pub confidence: &'static str,
    pub gaps: Vec<&'static str>,
    /// A `#:contract` directive line that codifies the checkable subset of
    /// this inferred contract.
    pub suggested: String,
}

fn counts(flow: &ContractFlow) -> (Option<u16>, Option<u16>) {
    match flow {
        ContractFlow::Fixed { consumes, produces } => (Some(*consumes), Some(*produces)),
        ContractFlow::Dynamic => (None, None),
    }
}

/// The `#:contract` directive that codifies the inferred contract's checkable
/// subset, in the same keys and values the report itself uses, so pasting it
/// back verifies exactly what was reported. A `variable` arity is omitted (the
/// checker cannot pin it).
fn suggested_directive(name: &str, contract: &WordContract) -> String {
    let mut parts = vec![format!("#:contract {name}")];
    if let (Some(inputs), Some(outputs)) = counts(&contract.flow) {
        parts.push(format!("inputs={inputs}"));
        parts.push(format!("outputs={outputs}"));
    }
    parts.push(format!("purity={}", contract.purity.as_spec_str()));
    parts.push(format!("partiality={}", contract.partiality.as_spec_str()));
    parts.push(format!(
        "determinism={}",
        contract.determinism.as_spec_str()
    ));
    // The exact-only discipline instead applies to `cost`, per axis rather
    // than word-wide: `steps`/`numeric`/`collection` each carry their own
    // witness (`docs/dev/cost-contract-design.md` §3), so each is gated on
    // its own rather than by the word's overall `confidence`. An axis stays
    // out entirely when its bound is not provably attained, since an unproven
    // bound would only ever check as a note — suggesting it would invite a
    // declaration weaker than the checker verifies. When *no* axis is exact,
    // the `cost` keyword itself must be left out too: `parse_cost_terms`
    // rejects a bare `cost` with zero `axis=class` terms.
    let mut cost_terms = Vec::new();
    if contract.cost.steps.1 {
        cost_terms.push(format!(
            "steps={}",
            CostClass::as_spec_str(contract.cost.steps.0)
        ));
    }
    if contract.cost.numeric.1 {
        cost_terms.push(format!(
            "numeric={}",
            CostClass::as_spec_str(contract.cost.numeric.0)
        ));
    }
    if contract.cost.collection.1 {
        cost_terms.push(format!(
            "collection={}",
            CostClass::as_spec_str(contract.cost.collection.0)
        ));
    }
    if !cost_terms.is_empty() {
        parts.push("cost".to_string());
        parts.extend(cost_terms);
    }
    parts.join(" ")
}

/// Infer and render every user word's contract, in source-definition order.
/// Execution-free.
pub(crate) fn report_contracts(source: &str) -> Vec<WordReport> {
    let (mut interp, names) = build_definitions_interpreter(source);
    let mut reports = Vec::new();
    for name in names {
        let Some(contract) = interp.infer_word_contract(&name) else {
            continue;
        };
        let (inputs, outputs) = counts(&contract.flow);
        reports.push(WordReport {
            name: name.clone(),
            inputs,
            outputs,
            partiality: contract.partiality.as_spec_str(),
            purity: contract.purity.as_spec_str(),
            determinism: contract.determinism.as_spec_str(),
            cost_steps: CostClass::as_spec_str(contract.cost.steps.0),
            cost_numeric: CostClass::as_spec_str(contract.cost.numeric.0),
            cost_collection: CostClass::as_spec_str(contract.cost.collection.0),
            effects: contract.effects.clone(),
            confidence: contract.confidence.as_spec_str(),
            gaps: contract.gaps.iter().map(|gap| gap.as_str()).collect(),
            suggested: suggested_directive(&name, &contract),
        });
    }
    reports
}

/// A count, or `"variable"` — the spelling a contract Record uses.
fn arity_json(count: Option<u16>) -> serde_json::Value {
    count.map_or_else(|| serde_json::json!("variable"), |n| serde_json::json!(n))
}

/// JSON array for the `--json` envelope.
pub(crate) fn reports_json(reports: &[WordReport]) -> serde_json::Value {
    serde_json::Value::Array(
        reports
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.name,
                    "inputs": arity_json(r.inputs),
                    "outputs": arity_json(r.outputs),
                    "partiality": r.partiality,
                    "purity": r.purity,
                    "determinism": r.determinism,
                    "cost": {
                        "steps": r.cost_steps,
                        "numeric": r.cost_numeric,
                        "collection": r.cost_collection,
                    },
                    "effects": r.effects,
                    "confidence": r.confidence,
                    "gaps": r.gaps,
                    "suggested": r.suggested,
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    //! Tests for Phase 1 of `docs/dev/cost-discoverability-work-order-2026-08.md`:
    //! surfacing the inferred `cost` bound (`interpreter::word_cost`) through
    //! `ajisai contract`'s JSON and `suggested` directive, under an exact-only
    //! discipline applied per axis rather than word-wide (§1.4 pitfall B/C/D),
    //! and the standing guarantee that `suggested` is paste-ready: it may carry
    //! no term the declaration checker cannot parse.

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
        let reports = report_contracts("[ RANGE ] 'MK' DEF");
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
        let line = suggested("[ JOIN ] 'J' DEF", "J");
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
        // fail `contract_gap::parse_cost_terms` and break `suggested`'s only
        // purpose: pasting into source and passing `check --contract`.
        let line = suggested("[ [ 1 ] MAP ] 'M' DEF", "M");
        assert!(
            !line.contains("cost"),
            "cost keyword must be omitted with no exact axis: {line}"
        );
    }

    #[test]
    fn suggested_round_trips_through_the_checker() {
        // The one test that matters most (that work order §1.5 Step 1.4): every
        // `suggested` line this Phase produces, pasted back into its own source, must
        // pass `check --contract` cleanly.
        //
        // The assertions below are deliberately anchored on `findings`,
        // `violated` and the *number* of parsed declarations rather than on
        // `outcome`/`gapSummary`. A directive the parser rejects never
        // reaches `decl_outcomes` at all (`ContractDeclCheck::decl_outcomes`:
        // "a malformed directive counts toward `violated`/`findings` but not
        // this list"), so `outcome` folds to `"value"` and both `gapSummary`
        // counters stay 0 while `check` is in fact exiting 1 on a hard
        // error. Asserting only those three would certify a `suggested` line
        // the checker cannot even parse — which is exactly the round trip
        // this test exists to rule out.
        let source =
            "[ ADD ] 'A' DEF\n[ JOIN ] 'J' DEF\n[ [ 1 ] MAP ] 'M' DEF\n[ 0 10 RANGE ] 'K' DEF";
        let reports = report_contracts(source);
        assert_eq!(reports.len(), 4, "expected one report per defined word");
        let mut annotated = source.to_string();
        for r in &reports {
            annotated.push('\n');
            annotated.push_str(&r.suggested);
        }
        let response = crate::agent::api::check(&annotated, true);
        let json = response.to_json();
        let decls = &json["contractDecls"];
        // Every suggested line parsed *and* verified.
        assert_eq!(
            decls["findings"].as_array().expect("findings array").len(),
            0,
            "suggested lines produced findings: {decls}"
        );
        assert_ne!(decls["outcome"], "error", "decls was: {decls}");
        assert_eq!(
            decls["declarations"]
                .as_array()
                .expect("declarations array")
                .len(),
            reports.len(),
            "a suggested line failed to parse, so it never reached the check: {decls}"
        );
        assert_eq!(decls["outcome"], "value", "decls was: {decls}");
        assert_eq!(decls["gapSummary"]["violated"], 0);
        assert_eq!(decls["gapSummary"]["cannotVerify"], 0);
        // The user-facing signal: pasting the suggestions back in still exits 0.
        assert_eq!(response.exit_code(), 0, "decls was: {decls}");
    }

    #[test]
    fn suggested_carries_no_term_the_checker_cannot_parse() {
        // Regression for the space term: `space:linear` was emitted here
        // while `contract_decl.rs` has no `space:` production, so the whole
        // directive was rejected as malformed and `check --contract` exited
        // 1 on its own suggestion. `[ ADD ]` is space-exact (`space:linear`
        // in the report), which is precisely the case that used to emit it.
        let line = suggested("[ ADD ] 'A' DEF", "A");
        assert!(
            !line.contains("space"),
            "suggested must carry only checkable terms: {line}"
        );
    }

    #[test]
    fn axis_order_is_stable() {
        // `ADD` is exact on all three axes (`word_cost_tests.rs`'s
        // `input_driven_arithmetic_is_exactly_linear_in_numeric_work`), so the
        // full `cost` term is always present — locking the required
        // steps → numeric → collection order in one literal string.
        let source = "[ ADD ] 'A' DEF";
        let line = suggested(source, "A");
        assert!(
            line.contains("cost steps=const numeric=linear collection=const"),
            "line was: {line}"
        );
        // And the same source reported twice renders identically, so a diff
        // of two `ajisai contract` runs stays empty.
        assert_eq!(line, suggested(source, "A"));
    }
}
