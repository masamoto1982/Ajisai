//! JSON report assembly for the `ajisai` CLI (`--json`).
//!
//! Serializes the *existing* diagnostic structures — `DebugDiagnosis`,
//! `AiDiagnosticPayload`, `ErrorFlowEvent`, `RuntimeMetrics`, and the shared
//! value protocol (`types::value_protocol`) — into the camelCase wire format
//! documented in `docs/dev/agent-cli-output-contract.md`. Field names follow
//! the same protocol-string convention as the WASM boundary
//! (`diagnosis_to_js` / `value_to_protocol`); no new diagnostic concepts are
//! introduced here.

use super::explain::{Explanation, Lang};
use super::plan_check::PlanCheck;
use crate::interpreter::debug_diagnosis::{AiDiagnosticPayload, DebugDiagnosis};
use crate::interpreter::error_flow_trace::ErrorFlowEvent;
use crate::interpreter::{Interpreter, RuntimeMetrics};
use crate::semantic::AbsenceMetadata;
use crate::types::value_protocol::{
    interpretation_protocol_str, value_to_protocol, ProtocolNode, ProtocolValue,
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
    /// Plain-language projection of the diagnosis (`--explain`). `None` unless
    /// the user opted in; additive field, see the CLI output contract.
    pub explanation: Option<Explanation>,
    /// Light contract / flow-mass check (`check --contract`). `None` unless the
    /// user opted in; additive field, see the CLI output contract.
    pub plan_check: Option<PlanCheck>,
    /// Language for rendering `plan_check` findings.
    pub lang: Lang,
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
            "explanation": self.explanation.as_ref().map(explanation_json),
            "planCheck": self.plan_check.as_ref().map(|check| plan_check_json(check, self.lang)),
        })
    }
}

/// JSON rendering of the light contract check (`super::plan_check`). Structured
/// mass numbers and NIL-flow word lists, plus the plain-language `findings`.
fn plan_check_json(check: &PlanCheck, lang: Lang) -> Json {
    let findings: Vec<Json> = check
        .findings(lang)
        .into_iter()
        .map(|finding| {
            json!({
                "severity": finding.severity.as_str(),
                "message": finding.message,
            })
        })
        .collect();
    json!({
        "overConsumes": check.over_consumes,
        "minDepth": check.min_depth,
        "netMass": check.net_mass,
        "massKnown": check.mass_known,
        "mayBubble": check.may_bubble,
        "hasFallback": check.has_fallback,
        "rejectsNil": check.rejects_nil,
        "findings": findings,
    })
}

/// JSON rendering of the plain-language projection (`super::explain`). The
/// L0 tier is `headline` + `nextStep`; `details` is the L2 repair checklist.
fn explanation_json(explanation: &Explanation) -> Json {
    json!({
        "lang": explanation.lang.as_str(),
        "headline": explanation.headline,
        "nextStep": explanation.next_step,
        "details": explanation.details,
    })
}

pub(crate) fn stack_json(interp: &Interpreter) -> Json {
    let hints = interp.collect_stack_hints();
    let nodes: Vec<Json> = interp
        .get_stack()
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let hint = hints.get(i).copied().unwrap_or(Interpretation::Unassigned);
            protocol_node_json(&value_to_protocol(value, Some(hint)))
        })
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
    if let Some(module) = &diagnosis.where_.module {
        where_obj.insert("module".into(), json!(module));
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
    })
}

fn check_json(check: &crate::interpreter::debug_diagnosis::DebugCheck) -> Json {
    json!({ "label": check.label, "detail": check.detail })
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
    // The VTU observation counters (docs/dev/virtual-tensor-unit-design.md)
    // plus the aggregate energyProxyScore (docs/quality/energy-proxy-score.md).
    // Counter names and the score describe observed structural work; they are
    // a proxy and never assert an energy outcome in joules.
    let proxy = crate::interpreter::energy_proxy::energy_proxy_report(metrics);
    json!({
        "vtu": {
            "tensorFlattenCount": metrics.vtu_tensor_flatten_count,
            "tensorFlattenedElements": metrics.vtu_tensor_flattened_elements,
            "tensorRebuildCount": metrics.vtu_tensor_rebuild_count,
            "tensorRebuiltElements": metrics.vtu_tensor_rebuilt_elements,
            "broadcastCount": metrics.vtu_broadcast_count,
            "unaryFlatCount": metrics.vtu_unary_flat_count,
            "allocatedElements": metrics.vtu_allocated_elements,
            "sameShapeElementwiseCount": metrics.vtu_same_shape_elementwise_count,
            "projectedBroadcastCount": metrics.vtu_projected_broadcast_count,
            "simdKernelUseCount": metrics.vtu_simd_kernel_use_count,
            "sparseCandidateCount": metrics.vtu_sparse_candidate_count,
            "sparseCandidateElements": metrics.vtu_sparse_candidate_elements,
            "sparseCandidateNonzeroElements": metrics.vtu_sparse_candidate_nonzero_elements,
            "sparseSkippableZeroElements": metrics.vtu_sparse_skippable_zero_elements,
            "candidateBlockCount": metrics.vtu_candidate_block_count,
            "rejectedBlockCount": metrics.vtu_rejected_block_count,
            "fusionCandidateCount": metrics.vtu_fusion_candidate_count,
            "bulkKernelUseCount": metrics.vtu_bulk_kernel_use_count,
            // Aggregate structural-cost proxy. Not joules; comparable only
            // within one proxyVersion. See docs/quality/energy-proxy-score.md.
            "energyProxyScore": proxy.score,
            "proxyVersion": proxy.proxy_version,
            "suggestions": proxy.suggestions,
        },
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
        ProtocolValue::Handle(id) => json!(id),
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
    Json::Object(obj)
}
