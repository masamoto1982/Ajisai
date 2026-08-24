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
use crate::interpreter::word_contract::{
    ContractConfidence, ContractDeterminism, ContractFlow, ContractPurity, NilBehavior,
    OrderSensitivity,
};
use crate::interpreter::word_cost::CostClass;
use crate::interpreter::word_space::SpaceClass;

/// One user word's inferred contract, rendered into stable labels.
pub(crate) struct WordReport {
    pub name: String,
    /// `"( c -- p )"` for a fixed arity, or `"dynamic"`.
    pub arity: String,
    pub purity: &'static str,
    pub determinism: &'static str,
    pub nil: &'static str,
    pub order: &'static str,
    /// The inferred space-growth class (`space:const` … `space:unbounded`).
    pub space: &'static str,
    /// The inferred charged-cost class on each of the three axes
    /// (`"const"` … `"unbounded"`, no `cost:`/`space:`-style prefix — these
    /// sit under a named axis in both JSON and terminal output, and the bare
    /// word matches the declaration vocabulary (`cost steps=const`)).
    pub cost_steps: &'static str,
    pub cost_numeric: &'static str,
    pub cost_collection: &'static str,
    pub effects: Vec<String>,
    pub confidence: &'static str,
    /// A `#:contract` directive line that codifies the checkable subset of
    /// this inferred contract (arity + purity + nil-freedom + `cost`). Never
    /// the space class, which the declaration grammar cannot parse.
    pub suggested: String,
}

fn purity_label(p: ContractPurity) -> &'static str {
    match p {
        ContractPurity::Pure => "pure",
        ContractPurity::Observable => "observable",
        ContractPurity::Effectful => "effectful",
    }
}

fn nil_label(n: NilBehavior) -> &'static str {
    match n {
        NilBehavior::NeverCreates => "nil-free",
        NilBehavior::Propagates => "nil-propagating",
        NilBehavior::MayCreate => "may-create-nil",
        NilBehavior::RejectsNil => "rejects-nil",
        NilBehavior::ConsumesNil => "consumes-nil",
    }
}

fn space_label(class: SpaceClass) -> &'static str {
    match class {
        SpaceClass::Const => "space:const",
        SpaceClass::Linear => "space:linear",
        SpaceClass::Superlinear => "space:superlinear",
        SpaceClass::Unbounded => "space:unbounded",
    }
}

fn arity_label(flow: &ContractFlow) -> String {
    match flow {
        ContractFlow::Fixed { consumes, produces } => format!("( {consumes} -- {produces} )"),
        ContractFlow::Dynamic => "dynamic".to_string(),
    }
}

/// The `#:contract` directive that codifies the inferred contract's checkable
/// subset. A dynamic arity is omitted (the checker cannot pin it); NIL behavior
/// maps to `nil-free` when the word never manufactures absence, else `may-nil`.
fn suggested_directive(
    name: &str,
    contract: &crate::interpreter::word_contract::WordContract,
) -> String {
    let mut parts = vec![format!("#:contract {name}")];
    if let ContractFlow::Fixed { consumes, produces } = &contract.flow {
        parts.push(format!("( {consumes} -- {produces} )"));
    }
    parts.push(purity_label(contract.purity).to_string());
    let nil = match contract.nil_behavior {
        NilBehavior::MayCreate => Some("may-nil"),
        NilBehavior::NeverCreates | NilBehavior::Propagates => Some("nil-free"),
        // Rejects/Consumes are not expressible as a nil-free/may-nil flag.
        NilBehavior::RejectsNil | NilBehavior::ConsumesNil => None,
    };
    if let Some(nil) = nil {
        parts.push(nil.to_string());
    }
    // No space term: the space class is *reported* (see `WordReport::space`)
    // but is not part of the checkable subset. `contract_decl.rs` has no
    // `space:` production — the declarable properties are arity, purity and
    // NIL behavior (`spec/language-semantics.md`, LANG.CONTRACT.CHECK) plus
    // `cost` — so emitting `space:linear` here made the whole directive a
    // malformed one, and a malformed directive is a hard `error` that fails
    // `check --contract` outright. `suggested` exists only to be pasted back
    // and pass, so it must carry nothing the checker cannot parse.
    //
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
        reports.push(WordReport {
            name: name.clone(),
            arity: arity_label(&contract.flow),
            purity: purity_label(contract.purity),
            determinism: match contract.determinism {
                ContractDeterminism::Deterministic => "deterministic",
                ContractDeterminism::NonDeterministic => "non-deterministic",
            },
            nil: nil_label(contract.nil_behavior),
            order: match contract.order_sensitivity {
                OrderSensitivity::OrderIndependent => "order-independent",
                OrderSensitivity::OrderSensitive => "order-sensitive",
            },
            space: space_label(contract.space),
            cost_steps: CostClass::as_spec_str(contract.cost.steps.0),
            cost_numeric: CostClass::as_spec_str(contract.cost.numeric.0),
            cost_collection: CostClass::as_spec_str(contract.cost.collection.0),
            effects: contract.effects.clone(),
            confidence: match contract.confidence {
                ContractConfidence::Complete => "complete",
                ContractConfidence::Conservative => "conservative",
            },
            suggested: suggested_directive(&name, &contract),
        });
    }
    reports
}

/// JSON array for the `--json` envelope.
pub(crate) fn reports_json(reports: &[WordReport]) -> serde_json::Value {
    serde_json::Value::Array(
        reports
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.name,
                    "arity": r.arity,
                    "purity": r.purity,
                    "determinism": r.determinism,
                    "nil": r.nil,
                    "order": r.order,
                    "space": r.space,
                    "cost": {
                        "steps": r.cost_steps,
                        "numeric": r.cost_numeric,
                        "collection": r.cost_collection,
                    },
                    "effects": r.effects,
                    "confidence": r.confidence,
                    "suggested": r.suggested,
                })
            })
            .collect(),
    )
}
