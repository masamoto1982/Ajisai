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

use super::api::ComputeOptions;
use super::contract_decl::build_definitions_interpreter;
use super::execution_receipt::limit_profile_json;
use super::resolve_words;
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
pub(crate) fn predict_outcomes(source: &str, options: &ComputeOptions) -> OutcomeReport {
    // The interpreter a prediction reasons about: the same ceilings a
    // `compute` under `options` would run under, so `limitProfile` names what
    // the prediction actually assumed.
    let probe = options.interpreter();
    // `tokenize` already ran the structural phase, so an unbalanced bracket
    // is refused here with every other source error.
    let Ok(tokens) = crate::tokenizer::tokenize(source) else {
        return exact("error:malformedSource", &probe);
    };
    // A source past `sourceBytes` is refused before anything runs, whether
    // or not it holds a token (one long comment does not), so the walk below
    // — which adds its structural ceilings only for a non-empty token
    // stream — would answer `value` for it. Only the `#:contract` check,
    // which `compute` runs first, can end it otherwise.
    if probe
        .runtime_limits()
        .check_source_bytes(source.len())
        .is_err()
    {
        let mut outcomes = vec!["error:resourceLimitExceeded".to_string()];
        if super::contract_violation::declares_contracts(source) {
            outcomes.insert(0, "error:contractViolation".to_string());
        }
        return OutcomeReport {
            exact: outcomes.len() == 1,
            outcomes,
            limit_profile: limit_profile_json(probe.runtime_limits(), probe.max_execution_steps()),
        };
    }
    let (mut interp, _names, unsettled) = build_definitions_interpreter(source);
    options.apply(&mut interp);
    let mut prediction = interp.predict_program_outcomes(&tokens, &|name| unsettled.contains(name));
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
    // A `#:contract` directive is checked before anything runs, and a
    // declaration inference disproves refuses the run (`api::compute`), so a
    // program that carries one can end that way and no other program can.
    // Decided from the directive's presence alone, exactly as `compute`
    // decides whether to check.
    if super::contract_violation::declares_contracts(source)
        && !prediction
            .outcomes
            .iter()
            .any(|outcome| outcome == "error:contractViolation")
    {
        prediction
            .outcomes
            .push("error:contractViolation".to_string());
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

#[cfg(test)]
mod tests {
    use crate::agent::api::{predict_outcomes, ComputeOptions};

    /// Each Word calls the one before it twice, so walking every call site
    /// anew visits `W0` 2^40 times; the walk visits each Word once. Run on a
    /// thread so a regression fails here instead of hanging the suite.
    #[test]
    fn prediction_is_not_exponential_in_the_call_graph() {
        let mut source = "[ 1 ] 'W0' DEF\n".to_string();
        for i in 1..=40 {
            source.push_str(&format!("[ W{0} W{0} ] 'W{1}' DEF\n", i - 1, i));
        }
        source.push_str("W40");
        let (done, finished) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let report = predict_outcomes(&source, ComputeOptions::agent(None));
            let _ = done.send(report.to_json()["outcomes"].clone());
        });
        let outcomes = finished
            .recv_timeout(std::time::Duration::from_secs(30))
            .expect("prediction must finish");
        assert!(outcomes.as_array().unwrap().iter().any(|id| id == "value"));
    }

    /// One comment line past `sourceBytes` holds no token, and the run
    /// refuses it all the same.
    #[test]
    fn a_source_past_the_byte_limit_predicts_the_refusal_even_without_tokens() {
        let source = format!("# {}", "x".repeat(70_000));
        let response = predict_outcomes(&source, ComputeOptions::agent(None)).to_json();
        assert_eq!(
            response["outcomes"],
            serde_json::json!(["error:resourceLimitExceeded"])
        );
        assert_eq!(response["exact"], true);
    }

    #[test]
    fn malformed_source_predicts_exactly_that() {
        let response = predict_outcomes("[ 1 2", ComputeOptions::default()).to_json();
        assert_eq!(response["status"], "ok");
        assert_eq!(response["exact"], true);
        assert_eq!(
            response["outcomes"],
            serde_json::json!(["error:malformedSource"])
        );
    }

    /// An unknown word is reachable, not inevitable: word resolution happens
    /// during execution, so anything that fails earlier decides the outcome
    /// instead. `error:unknownWord` therefore joins the set rather than
    /// replacing it, and the prediction is not exact.
    #[test]
    fn an_unknown_word_joins_the_set_without_claiming_to_be_the_whole_answer() {
        let response = predict_outcomes("FROBNICATE", ComputeOptions::default()).to_json();
        let outcomes = response["outcomes"].as_array().unwrap();
        assert!(outcomes.iter().any(|v| v == "error:unknownWord"));

        // These two really answer `stackUnderflow` and `nonNumeric` — measured,
        // and the reason claiming `unknownWord` exactly was an under-approximation.
        for (source, actual) in [
            ("ADD FROBNICATE", "error:stackUnderflow"),
            ("'a' 1 ADD FROBNICATE", "error:nonNumeric"),
        ] {
            let response = predict_outcomes(source, ComputeOptions::default()).to_json();
            assert_eq!(response["exact"], false, "{source}");
            let outcomes = response["outcomes"].as_array().unwrap();
            assert!(
                outcomes.iter().any(|v| v == actual),
                "{source}: predicted {outcomes:?}, which omits the outcome it really produces ({actual})"
            );
            assert!(
                outcomes.iter().any(|v| v == "error:unknownWord"),
                "{source}"
            );
        }
    }

    #[test]
    fn a_program_that_calls_nothing_still_carries_structural_ceilings() {
        // Even pure literals with no Word call at all can in principle hit a
        // structural ceiling (a numeric literal too long for the profile, for
        // instance — see `word_outcome_vocabulary::structural_ceiling_ids`'s
        // doc), so this is never exact; only the truly empty program is.
        let response = predict_outcomes("1 2 3", ComputeOptions::default()).to_json();
        assert_eq!(response["exact"], false);
        let outcomes = response["outcomes"].as_array().unwrap();
        assert!(outcomes.iter().any(|v| v == "value"));
        assert!(outcomes.iter().any(|v| v == "error:resourceLimitExceeded"));
    }

    #[test]
    fn the_empty_program_predicts_exactly_value() {
        let response = predict_outcomes("", ComputeOptions::default()).to_json();
        assert_eq!(response["exact"], true);
        assert_eq!(response["outcomes"], serde_json::json!(["value"]));
    }

    #[test]
    fn every_response_names_its_limit_profile() {
        let response = predict_outcomes("1 2 ADD", ComputeOptions::default()).to_json();
        assert!(response["limitProfile"]["executionSteps"].is_u64());
        assert!(response["limitProfile"]["materializedElements"].is_u64());
    }

    #[test]
    fn a_nontrivial_program_over_approximates_and_says_so() {
        let response = predict_outcomes("1 2 ADD", ComputeOptions::default()).to_json();
        let outcomes: Vec<String> = response["outcomes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert!(outcomes.contains(&"value".to_string()));
        assert!(outcomes.contains(&"error:nonNumeric".to_string()));
        assert_eq!(response["exact"], false);
    }
}
