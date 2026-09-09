//! `ajisai agent outcomes <file>` — predict the finite set of outcome ids a
//! program could produce, execution-free (Phase 5,
//! `docs/dev/auditable-kernel-work-order-2026-09.md`).
//!
//! Two failures are exact by construction, decided the same way `check`
//! decides them, before prediction's own (necessarily coarser) walk ever
//! runs: a source that does not tokenize, and one whose vector delimiters
//! are unbalanced. Both are settled before a single Word runs, so nothing
//! else can happen first and each really is the one possible outcome.
//!
//! An unknown word is *not* in that class, though it looks like it. Word
//! resolution happens during execution, not before it, so an unresolvable
//! name decides the outcome only if execution reaches it — `ADD FROBNICATE`
//! answers `stackUnderflow`, and `'a' 1 ADD FROBNICATE` answers
//! `nonNumeric`. Claiming `error:unknownWord` exactly, as this module first
//! did, was an under-approximation (pitfall A) wearing the strongest label
//! the tool has. It is folded into the ordinary walk instead: reachable, so
//! it joins the set, without displacing whatever could fail before it.

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
    let (mut interp, _names) = build_definitions_interpreter(source);
    let mut prediction = interp.predict_program_outcomes(&tokens);
    // A name nothing defines raises `unknownWord` when execution reaches it.
    // The walk already covers the reaching part (every Word that could fail
    // first is in the set); this adds the arrival itself. `resolve_words` is
    // the same best-effort resolution `check` reports, so this fires on
    // exactly the names `check` would name.
    if !resolve_words(&probe, &tokens).unknown.is_empty()
        && !prediction
            .outcomes
            .iter()
            .any(|outcome| outcome == "error:unknownWord")
    {
        prediction.outcomes.push("error:unknownWord".to_string());
        prediction.outcomes.sort();
    }
    let exact = prediction.outcomes.len() == 1;
    OutcomeReport {
        outcomes: prediction.outcomes,
        exact,
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
