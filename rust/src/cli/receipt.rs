//! Execution receipt assembly for `ajisai run --json --receipt` (Phase 6).
//!
//! A receipt records what a run's result was based on — the source it ran, the
//! content-identified words it executed, the host capabilities it required and
//! was granted, the observable host effects it emitted in order, the water it
//! spent, and whether the compiled path agreed with the reference path — plus a
//! stable identity of the result computed from the shared value protocol, not a
//! display string.
//!
//! The receipt is additive: it appears only when `--receipt` is passed, and
//! producing it never changes the run's result (recording is observational).
//! It exposes only stable, public facts. Internal optimization details — SIMD
//! lane widths, shape-IC state, quantized-block internals, tier representations,
//! pointer identity, Rust `Debug` names, unstable cache keys — are never
//! included, and the receipt is a provenance record, not a proof of correctness
//! or tamper-evidence.

use serde_json::{json, Value as Json};

use crate::interpreter::content_digest;
use crate::interpreter::error_flow_trace::ErrorFlowEvent;
use crate::interpreter::Interpreter;

/// Version of the `receipt` object shape. Bump only on a breaking change to the
/// receipt fields; additive fields keep the same version.
pub(crate) const RECEIPT_SCHEMA_VERSION: u64 = 1;

/// Build the `receipt` JSON object for a completed run. `source` is the program
/// text (for source identity); `trace` is the drained error-flow trace (for
/// absence events).
pub(crate) fn build_receipt(interp: &Interpreter, source: &str, trace: &[ErrorFlowEvent]) -> Json {
    let metrics = interp.runtime_metrics();
    json!({
        "schemaVersion": RECEIPT_SCHEMA_VERSION,
        "sourceIdentity": content_digest(source.as_bytes()),
        "implementation": {
            "name": "ajisai-core",
            "version": env!("CARGO_PKG_VERSION"),
        },
        // No machine-readable version is declared by the specification yet; the
        // field is present so a future declared version is additive, not a shape
        // change. Never fabricated.
        "specification": { "declaredVersion": Json::Null },
        "executedWords": executed_words_json(interp),
        "requiredCapabilities": required_capabilities_json(interp),
        "grantedCapabilities": granted_capabilities_json(interp),
        "observedEffects": observed_effects_json(interp),
        "water": {
            "stepLimit": interp.max_execution_steps(),
            "stepsUsed": interp.execution_step_count(),
            "comparisonRefinements": metrics.compare_within_budget_terms_consumed,
        },
        "integrity": {
            "shadowValidationPerformed": metrics.shadow_validation_started_count > 0,
            "referenceAgreement": metrics.shadow_validation_integrity_mismatch_count == 0,
            "plainFallbacks": metrics.shadow_validation_fallback_count,
            "integrityMismatches": metrics.shadow_validation_integrity_mismatch_count,
        },
        "absenceEvents": absence_events_json(trace),
        "resultIdentity": result_identity(interp),
    })
}

/// Executed content-identified words, ordered by first execution. Only words
/// that carry a §8.6 content identity (user words) appear: core and module
/// words are part of the implementation vocabulary (captured by
/// `implementation`), not user provenance.
fn executed_words_json(interp: &Interpreter) -> Json {
    let mut rows: Vec<(u64, Json)> = Vec::new();
    for (name, record) in interp.receipt_recorder().executed_words() {
        let Some(identity) = interp.word_identity(name) else {
            continue;
        };
        rows.push((
            record.first_seen_order,
            json!({
                "resolvedName": name,
                "contentIdentity": identity,
                "firstSeenOrder": record.first_seen_order,
                "callCount": record.call_count,
            }),
        ));
    }
    rows.sort_by_key(|(order, _)| *order);
    Json::Array(rows.into_iter().map(|(_, row)| row).collect())
}

fn required_capabilities_json(interp: &Interpreter) -> Json {
    Json::Array(
        interp
            .receipt_recorder()
            .required_capabilities()
            .iter()
            .map(|cap| json!(cap.as_protocol_str()))
            .collect(),
    )
}

fn granted_capabilities_json(interp: &Interpreter) -> Json {
    Json::Array(
        interp
            .granted_host_capabilities()
            .iter()
            .map(|cap| json!(cap.as_protocol_str()))
            .collect(),
    )
}

/// Observable host effects, in emission order. `kind` is the stable
/// language-independent tag; `payload` is the effect's own text.
fn observed_effects_json(interp: &Interpreter) -> Json {
    Json::Array(
        interp
            .host_effects()
            .iter()
            .enumerate()
            .map(|(order, effect)| {
                json!({
                    "order": order,
                    "kind": effect.kind(),
                    "payload": effect.payload(),
                })
            })
            .collect(),
    )
}

/// Absence (NIL) events observed during the run, in order, with their reason,
/// origin, and recoverability — never collapsed to a generic failure.
fn absence_events_json(trace: &[ErrorFlowEvent]) -> Json {
    let mut events: Vec<Json> = Vec::new();
    for event in trace {
        let Some(absence) = &event.absence else {
            continue;
        };
        let mut obj = serde_json::Map::new();
        obj.insert("kind".into(), json!(event.kind.as_protocol_str()));
        if let Some(word) = &event.word {
            obj.insert("word".into(), json!(word));
        }
        if let Some(reason) = &absence.reason {
            obj.insert("reason".into(), json!(reason.as_protocol_str()));
        }
        obj.insert("origin".into(), json!(absence.origin.as_protocol_str()));
        obj.insert(
            "recoverability".into(),
            json!(absence.recoverability.as_protocol_str()),
        );
        events.push(Json::Object(obj));
    }
    Json::Array(events)
}

/// Identity of the final stack, computed from the canonical bytes of the shared
/// value protocol (kind, exact numerator/denominator, interpretation, absence
/// reason/origin/recoverability, logical-Unknown diagnosis, and Vector/Tensor/
/// Record structure) — never from a display string.
fn result_identity(interp: &Interpreter) -> String {
    let stack = super::report::stack_json(interp);
    content_digest(canonical_json(&stack).as_bytes())
}

/// Deterministic serialization of a JSON value: object keys are emitted in
/// sorted order so the byte sequence — and thus the derived identity — does not
/// depend on map iteration order.
fn canonical_json(value: &Json) -> String {
    match value {
        Json::Null => "null".to_string(),
        Json::Bool(b) => b.to_string(),
        Json::Number(n) => n.to_string(),
        Json::String(s) => encode_json_string(s),
        Json::Array(items) => {
            let parts: Vec<String> = items.iter().map(canonical_json).collect();
            format!("[{}]", parts.join(","))
        }
        Json::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .iter()
                .map(|key| format!("{}:{}", encode_json_string(key), canonical_json(&map[*key])))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
    }
}

fn encode_json_string(s: &str) -> String {
    // serde_json::to_string on a &str is total and produces the canonical
    // JSON string escaping.
    serde_json::to_string(s).expect("string encoding is infallible")
}
