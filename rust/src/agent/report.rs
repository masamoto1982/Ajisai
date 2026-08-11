//! JSON report assembly for the `ajisai` CLI (`--json`).
//!
//! Serializes the *existing* diagnostic structures — `DebugDiagnosis`,
//! `AiDiagnosticPayload`, `ErrorFlowEvent`, `RuntimeMetrics`, and the shared
//! value protocol (`types::value_protocol`) — into the camelCase wire format
//! documented in `docs/dev/agent-cli-output-contract.md`. Field names follow
//! the same protocol-string convention as the WASM boundary
//! (`diagnosis_to_js` / `value_to_protocol`); no new diagnostic concepts are
//! introduced here.

use crate::interpreter::debug_diagnosis::{AiDiagnosticPayload, DebugDiagnosis};
use crate::interpreter::error_flow_trace::ErrorFlowEvent;
use crate::interpreter::{Interpreter, RuntimeMetrics};
use crate::semantic::AbsenceMetadata;
use crate::types::value_protocol::{
    exact_display, exact_terms, interpretation_protocol_str, value_to_protocol, ProtocolNode,
    ProtocolValue,
};
use crate::types::{Interpretation, Value, ValueData};
use serde_json::{json, Map, Value as Json};

/// Version of the top-level `--json` envelope. Bump only on a breaking
/// change (field removal or rename); purely additive fields keep the same
/// version. See `docs/dev/agent-cli-output-contract.md`.
pub(crate) const SCHEMA_VERSION: u64 = 1;

pub(crate) struct Report {
    pub status: &'static str,
    pub stack: Json,
    /// Human display strings for the stack, bottom to top — the same text
    /// the GUI and PRINT render. Carried in the JSON envelope as
    /// `stackDisplay` so agents and the SKILL.md generator can show
    /// "code → expected stack" pairs without re-deriving display rules.
    pub stack_display: Vec<String>,
    pub output: Vec<String>,
    pub message: Option<String>,
    pub diagnosis: Option<DebugDiagnosis>,
    pub ai_diagnostic: Option<AiDiagnosticPayload>,
    pub error_flow_trace: Vec<ErrorFlowEvent>,
    pub runtime_metrics: RuntimeMetrics,
    /// Per-word contract declarations checked against inference
    /// (`check --contract`, P2). `None` unless the user opted in; additive
    /// field. Prebuilt JSON so `report` stays decoupled from the declaration
    /// types.
    pub contract_decls: Option<Json>,
}

impl Report {
    pub(crate) fn to_json(&self) -> Json {
        json!({
            "schemaVersion": SCHEMA_VERSION,
            "status": self.status,
            "stack": self.stack,
            "stackDisplay": self.stack_display,
            "output": self.output,
            "message": self.message,
            "diagnosis": self.diagnosis.as_ref().map(diagnosis_json),
            "errorFlowTrace": self
                .error_flow_trace
                .iter()
                .map(error_flow_event_json)
                .collect::<Vec<_>>(),
            "aiDiagnostic": self.ai_diagnostic.as_ref().map(ai_payload_json),
            "runtimeMetrics": runtime_metrics_json(&self.runtime_metrics),
            "contractDecls": self.contract_decls,
        })
    }
}

pub(crate) fn stack_json(interp: &Interpreter) -> Json {
    // The `Stack` owns aligned `(value, role)` slots, so iterate them directly.
    let nodes: Vec<Json> = interp
        .get_stack()
        .iter_slots()
        .map(|(value, role)| protocol_node_json(&value_to_protocol(value, Some(role))))
        .collect();
    Json::Array(nodes)
}

pub(crate) fn diagnosis_json(diagnosis: &DebugDiagnosis) -> Json {
    let mut where_obj = Map::new();
    where_obj.insert(
        "kind".into(),
        json!(diagnosis.where_.kind.as_protocol_str()),
    );
    if let Some(word) = &diagnosis.where_.word {
        where_obj.insert("word".into(), json!(word));
    }
    if let Some(dictionary) = &diagnosis.where_.dictionary {
        where_obj.insert("dictionary".into(), json!(dictionary));
    }
    json!({
        "when": diagnosis.when.as_protocol_str(),
        "why": diagnosis.why.as_protocol_str(),
        "summary": diagnosis.summary,
        "where": Json::Object(where_obj),
        "evidence": diagnosis.evidence,
        "nextChecks": diagnosis.next_checks.iter().map(check_json).collect::<Vec<_>>(),
        "agreedPrefix": diagnosis.agreed_prefix,
        "candidates": diagnosis.candidates,
        "resourceLimit": diagnosis.resource_limit.as_ref().map(resource_limit_json),
    })
}

fn check_json(check: &crate::interpreter::debug_diagnosis::DebugCheck) -> Json {
    json!({
        "code": check.code,
        "title": { "en": check.title.en, "ja": check.title.ja },
        "detail": { "en": check.detail.en, "ja": check.detail.ja },
    })
}

fn resource_limit_json(facts: &crate::interpreter::debug_diagnosis::ResourceLimitFacts) -> Json {
    json!({
        "resource": facts.resource,
        "limit": facts.limit,
        "observed": facts.observed,
    })
}

pub(crate) fn ai_payload_json(payload: &AiDiagnosticPayload) -> Json {
    json!({
        "kind": payload.kind,
        "recoverability": payload.recoverability,
        "semanticArea": payload.semantic_area,
        "word": payload.word,
        "semanticRole": payload.semantic_role,
        "algebraicFamily": payload.algebraic_family,
        "absenceReason": payload.nil_reason,
        "truthValue": payload.truth_value,
        "effect": payload.effect,
        "nextChecks": payload.next_checks.iter().map(check_json).collect::<Vec<_>>(),
        "candidates": payload.candidates,
        "resourceLimit": payload.resource_limit.as_ref().map(resource_limit_json),
    })
}

fn absence_json(absence: &AbsenceMetadata) -> Json {
    let mut obj = Map::new();
    if let Some(reason) = &absence.reason {
        obj.insert("reason".into(), json!(reason.as_protocol_str()));
    }
    obj.insert("origin".into(), json!(absence.origin.as_protocol_str()));
    obj.insert(
        "recoverability".into(),
        json!(absence.recoverability.as_protocol_str()),
    );
    if let Some(diagnosis) = &absence.diagnosis {
        obj.insert("diagnosis".into(), diagnosis_json(diagnosis));
    }
    Json::Object(obj)
}

pub(crate) fn error_flow_event_json(event: &ErrorFlowEvent) -> Json {
    let mut obj = Map::new();
    obj.insert("kind".into(), json!(event.kind.as_protocol_str()));
    if let Some(word) = &event.word {
        obj.insert("word".into(), json!(word));
    }
    if let Some(absence) = &event.absence {
        obj.insert("absence".into(), absence_json(absence));
    }
    obj.insert("stackLenBefore".into(), json!(event.stack_len_before));
    obj.insert("stackLenAfter".into(), json!(event.stack_len_after));
    obj.insert("message".into(), json!(event.message));
    if let Some(diagnosis) = &event.diagnosis {
        obj.insert("diagnosis".into(), diagnosis_json(diagnosis));
    }
    Json::Object(obj)
}

pub(crate) fn runtime_metrics_json(metrics: &RuntimeMetrics) -> Json {
    // Diagnostics only: these counters describe work the runtime was observed
    // to do. Reading them changes no result, and no Word reads them.
    json!({
        "compiledPlanBuildCount": metrics.compiled_plan_build_count,
        "compiledPlanCacheHitCount": metrics.compiled_plan_cache_hit_count,
        "compiledPlanCacheMissCount": metrics.compiled_plan_cache_miss_count,
        "condDispatchFastCount": metrics.cond_dispatch_fast_count,
        "condClauseCompiledCount": metrics.cond_clause_compiled_count,
        "scalarFastpathCount": metrics.scalar_fastpath_count,
        "resolveCacheHitCount": metrics.resolve_cache_hit_count,
        "resolveCacheMissCount": metrics.resolve_cache_miss_count,
        "resolveCacheInvalidationCount": metrics.resolve_cache_invalidation_count,
        "tailCallJumpCount": metrics.tail_call_jump_count,
        "executionSteps": metrics.execution_steps,
    })
}

/// JSON rendering of a `ProtocolNode` — the same shape `protocol_to_js`
/// produces for the GUI: `{ type, value, displayHint, semantics? }`.
fn protocol_node_json(node: &ProtocolNode) -> Json {
    let mut obj = Map::new();
    obj.insert(
        "displayHint".into(),
        json!(interpretation_protocol_str(node.display_hint)),
    );
    if let Some(source) = &node.semantics {
        obj.insert(
            "semantics".into(),
            semantics_json(source, node.display_hint),
        );
    }
    obj.insert("type".into(), json!(node.type_str));
    let value = match &node.value {
        ProtocolValue::Null => Json::Null,
        ProtocolValue::Bool(b) => json!(b),
        ProtocolValue::Text(s) => json!(s),
        ProtocolValue::Number {
            numerator,
            denominator,
        } => json!({ "numerator": numerator, "denominator": denominator }),
        ProtocolValue::Children(kids) => Json::Array(kids.iter().map(protocol_node_json).collect()),
    };
    obj.insert("value".into(), value);
    Json::Object(obj)
}

/// JSON rendering of the per-value `semantics` block — mirrors
/// `value_semantics_to_js` at the WASM boundary.
fn semantics_json(value: &Value, effective: Interpretation) -> Json {
    let mut obj = Map::new();
    obj.insert(
        "semanticKind".into(),
        json!(value.semantic_kind().as_protocol_str()),
    );
    obj.insert("shape".into(), json!(value.shape_kind().as_protocol_str()));
    let truth = value.truth_value_for_role(effective);
    if let Some(truth) = truth {
        obj.insert("truthValue".into(), json!(truth));
    }
    let mut capabilities: Vec<&'static str> = Vec::new();
    let mut has_truth_valued = false;
    for capability in value.capabilities() {
        if capability == crate::semantic::Capability::TruthValued {
            has_truth_valued = true;
        }
        capabilities.push(capability.as_protocol_str());
    }
    // A value rendered under the TruthValue role advertises `truthValued`
    // even when the role lives in the semantic plane (comparison/logic
    // booleans), not on the value's own hint.
    if truth.is_some() && !has_truth_valued {
        capabilities.push(crate::semantic::Capability::TruthValued.as_protocol_str());
    }
    obj.insert("capabilities".into(), json!(capabilities));
    obj.insert("origin".into(), json!(value.origin().as_protocol_str()));
    if let Some(absence) = value.normalized_absence_metadata() {
        obj.insert("absence".into(), absence_json(&absence));
    }
    if matches!(value.data, ValueData::ExactScalar(_))
        && effective != Interpretation::ContinuedFraction
    {
        obj.insert("approximate".into(), json!(true));
    }
    // The same normal form in two shapes: the terms a consumer computes with,
    // and one short string a reader can take in. Emitted together because they
    // are derived together — see `value_protocol::exact_display`.
    if let Some(display) = exact_display(value) {
        obj.insert("exactDisplay".into(), json!(display));
    }
    if let Some(terms) = exact_terms(value) {
        obj.insert(
            "exactTerms".into(),
            Json::Array(
                terms
                    .into_iter()
                    .map(|term| {
                        json!({
                            "numerator": term.numerator,
                            "denominator": term.denominator,
                            "radicand": term.radicand,
                        })
                    })
                    .collect(),
            ),
        );
    }
    Json::Object(obj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::exact::ExactReal;
    use crate::types::fraction::Fraction;

    #[test]
    fn cli_keeps_algebraic_normal_form_beside_approximation() {
        let sqrt_two = ExactReal::from_sqrt_rational(Fraction::new(2.into(), 1.into()))
            .expect("sqrt(2) is in the supported algebraic domain");
        let value = Value::from_exact_real(sqrt_two);
        let semantics = semantics_json(&value, Interpretation::RawNumber);

        assert_eq!(semantics["approximate"], true);
        assert_eq!(semantics["exactTerms"][0]["numerator"], "1");
        assert_eq!(semantics["exactTerms"][0]["denominator"], "1");
        assert_eq!(semantics["exactTerms"][0]["radicand"], "2");
        // The short rendering of those same terms. Without it the only two
        // things a reader meets before them are a truncated continued
        // fraction and a rational approximation.
        assert_eq!(semantics["exactDisplay"], "sqrt(2)");
    }
}
