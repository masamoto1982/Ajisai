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
