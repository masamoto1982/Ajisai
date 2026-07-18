//! Native tests for the execution receipt (`run --json --receipt`, Phase 6):
//! observational transparency, provenance of executed content-identified words,
//! ordered host effects, absence events, protocol-derived result identity, and
//! the exclusion of internal optimization vocabulary.

use super::receipt::{build_receipt, RECEIPT_SCHEMA_VERSION};
use crate::interpreter::Interpreter;
use std::sync::Arc;

/// Run `code` with receipt recording enabled and return `(interpreter, receipt
/// JSON)`. Mirrors the CLI `cmd_run` receipt path.
fn run_with_receipt(code: &str) -> (Interpreter, serde_json::Value) {
    let mut interp = Interpreter::with_host(Arc::new(super::host::CliHostEnv));
    interp.set_receipt_recording(true);
    super::block_on(interp.execute(code)).expect("program executes");
    let trace = interp.drain_error_flow_trace();
    let receipt = build_receipt(&interp, code, &trace);
    (interp, receipt)
}

fn run_plain(code: &str) -> Interpreter {
    let mut interp = Interpreter::with_host(Arc::new(super::host::CliHostEnv));
    super::block_on(interp.execute(code)).expect("program executes");
    interp
}

#[test]
fn receipt_has_schema_version_and_identities() {
    let (_interp, receipt) = run_with_receipt("[ 2 ] [ 3 ] +");
    assert_eq!(receipt["schemaVersion"], RECEIPT_SCHEMA_VERSION);
    assert!(receipt["sourceIdentity"].as_str().unwrap().starts_with('#'));
    assert!(receipt["resultIdentity"].as_str().unwrap().starts_with('#'));
    assert_eq!(receipt["implementation"]["name"], "ajisai-core");
}

#[test]
fn recording_is_observationally_transparent() {
    // The stack a receipt run leaves must equal a plain run's stack: enabling
    // provenance recording changes nothing observable.
    let code = "{ [ 2 ] * } 'DBL' DEF [ 5 ] DBL";
    let (with_receipt, _) = run_with_receipt(code);
    let plain = run_plain(code);
    let a = super::report::stack_json(&with_receipt);
    let b = super::report::stack_json(&plain);
    assert_eq!(a, b, "recording must not change the result");
}

#[test]
fn result_identity_distinguishes_values_and_is_stable() {
    let (_i1, r10a) = run_with_receipt("{ [ 2 ] * } 'DBL' DEF [ 5 ] DBL");
    let (_i2, r10b) = run_with_receipt("{ [ 2 ] * } 'DBL' DEF [ 5 ] DBL");
    let (_i3, r12) = run_with_receipt("{ [ 2 ] * } 'DBL' DEF [ 6 ] DBL");
    // Same program → same identity; different result → different identity.
    assert_eq!(r10a["resultIdentity"], r10b["resultIdentity"]);
    assert_ne!(r10a["resultIdentity"], r12["resultIdentity"]);
}

#[test]
fn executed_words_records_content_identity_and_call_count() {
    // DBL is invoked three times via MAP; the receipt aggregates that into one
    // entry with the word's §8.6 content identity and a call count.
    let (interp, receipt) = run_with_receipt("{ [ 2 ] * } 'DBL' DEF [ 1 2 3 ] 'DBL' MAP");
    let words = receipt["executedWords"].as_array().unwrap();
    let dbl = words
        .iter()
        .find(|w| w["resolvedName"] == "EXAMPLE@DBL")
        .expect("DBL appears in executed words");
    assert_eq!(
        dbl["contentIdentity"],
        serde_json::json!(interp.word_identity("EXAMPLE@DBL").unwrap())
    );
    assert_eq!(dbl["callCount"], 3);
    // Core/module words carry no content identity and are excluded.
    assert!(words.iter().all(|w| w["resolvedName"] != "MAP"));
}

#[test]
fn observed_effects_preserve_kind_and_order() {
    let (_interp, receipt) = run_with_receipt("[ 1 ] PRINT [ 2 ] PRINT");
    let effects = receipt["observedEffects"].as_array().unwrap();
    assert_eq!(effects.len(), 2);
    assert_eq!(effects[0]["order"], 0);
    assert_eq!(effects[0]["kind"], "print");
    assert_eq!(effects[1]["order"], 1);
    assert_eq!(effects[0]["payload"], "[ 1/1 ]");
    assert_eq!(effects[1]["payload"], "[ 2/1 ]");
}

#[test]
fn required_and_granted_capabilities_are_recorded() {
    let (_interp, receipt) = run_with_receipt("[ 1 ] PRINT");
    let required = receipt["requiredCapabilities"].as_array().unwrap();
    assert!(
        required.iter().any(|c| c == "effect"),
        "PRINT requires effect"
    );
    // The CLI host grants a fixed set; the field lists them as protocol strings.
    let granted = receipt["grantedCapabilities"].as_array().unwrap();
    assert!(granted.iter().all(|c| c.is_string()));
}

#[test]
fn absence_event_preserves_reason_origin_recoverability() {
    let (_interp, receipt) = run_with_receipt("[ 5 ] [ 0 ] /");
    let events = receipt["absenceEvents"].as_array().unwrap();
    let event = events.last().expect("division by zero yields a NIL event");
    assert_eq!(event["reason"], "divisionByZero");
    assert_eq!(event["origin"], "executionFailure");
    assert_eq!(event["recoverability"], "recoverable");
}

#[test]
fn integrity_and_water_fields_present() {
    let (_interp, receipt) = run_with_receipt("[ 2 ] [ 3 ] +");
    let integrity = &receipt["integrity"];
    assert!(integrity["shadowValidationPerformed"].is_boolean());
    assert!(integrity["referenceAgreement"].is_boolean());
    assert_eq!(integrity["plainFallbacks"], 0);
    assert_eq!(integrity["integrityMismatches"], 0);
    let water = &receipt["water"];
    assert_eq!(water["stepLimit"], 100_000);
    assert!(water["stepsUsed"].as_u64().unwrap() >= 1);
}

#[test]
fn receipt_excludes_internal_optimization_vocabulary() {
    // The receipt is a stable public record; internal optimization identifiers
    // must never leak into it.
    let (_interp, receipt) = run_with_receipt("{ [ 2 ] * } 'DBL' DEF [ 1 2 3 ] 'DBL' MAP");
    let text = receipt.to_string().to_lowercase();
    for forbidden in [
        "simd",
        "quantiz",
        "shapeic",
        "shape_ic",
        "tier",
        "epoch",
        "compiledplan",
        "arc",
        "pointer",
    ] {
        assert!(
            !text.contains(forbidden),
            "receipt leaked internal term `{}`",
            forbidden
        );
    }
}
