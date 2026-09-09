//! `ajisai agent outcomes <file>` — predict the finite set of outcome ids a
//! program could produce, execution-free (Phase 5,
//! `docs/dev/auditable-kernel-work-order-2026-09.md`).
//!
//! Three failures are exact by construction, decided the same way `check`
//! decides them, before prediction's own (necessarily coarser) walk ever
//! runs: a source that does not tokenize, one whose vector delimiters are
//! unbalanced, and one naming a word nothing defines. Each has exactly one
//! possible outcome — there is nothing to predict.

use super::contract_decl::build_definitions_interpreter;
use super::execution_receipt::limit_profile_json;
use super::{check_structure, resolve_words};
use crate::interpreter::Interpreter;

pub(crate) struct OutcomeReport {
    pub outcomes: Vec<String>,
    pub exact: bool,
    pub limit_profile: serde_json::Value,
}

fn exact(outcome: &str, interp: &Interpreter) -> OutcomeReport {
    OutcomeReport {
        outcomes: vec![outcome.to_string()],
        exact: true,
        limit_profile: limit_profile_json(interp.runtime_limits(), interp.max_execution_steps()),
    }
}

/// Predict `source`'s outcome set without executing it.
pub(crate) fn predict_outcomes(source: &str) -> OutcomeReport {
    let probe = Interpreter::new();
    let Ok(tokens) = crate::tokenizer::tokenize(source) else {
        return exact("error:malformedSource", &probe);
    };
    if check_structure(&tokens).is_err() {
        return exact("error:structureError", &probe);
    }
    let resolved = resolve_words(&probe, &tokens);
    if !resolved.unknown.is_empty() {
        return exact("error:unknownWord", &probe);
    }

    let (mut interp, _names) = build_definitions_interpreter(source);
    let prediction = interp.predict_program_outcomes(&tokens);
    OutcomeReport {
        outcomes: prediction.outcomes,
        exact: prediction.exact,
        limit_profile: limit_profile_json(interp.runtime_limits(), interp.max_execution_steps()),
    }
}

impl OutcomeReport {
    pub(crate) fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "schemaVersion": super::report::SCHEMA_VERSION,
            "status": "ok",
            "outcomes": self.outcomes,
            "exact": self.exact,
            "limitProfile": self.limit_profile,
        })
    }
}
