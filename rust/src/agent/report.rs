//! Report assembly for a completed execution, and its JSON rendering for the
//! `ajisai` CLI (`--json`).
//!
//! Assembly is I/O-free, so the typed agent API and the terminal CLI observe
//! the same stack, NIL flow, diagnostics, output and runtime metrics.
//! Rendering serializes the *existing* diagnostic structures — `DebugDiagnosis`,
//! `AiDiagnosticPayload`, `ErrorFlowEvent`, `RuntimeMetrics`, and the shared
//! value protocol (`types::value_protocol`) — into the camelCase wire format
//! documented in `docs/dev/agent-cli-output-contract.md`. Field names follow
//! the same protocol-string convention as the WASM boundary
//! (`diagnosis_to_js` / `value_to_protocol`); no new diagnostic concepts are
//! introduced here.

use super::execution_receipt::build_receipt;
use super::observation_digest::{observation_digest, ObservationDigestInput};
use super::{error_report, user_word_identities};
use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::{AiDiagnosticPayload, DebugDiagnosis};
use crate::interpreter::error_flow_trace::{ErrorFlowEvent, ErrorFlowEventKind};
use crate::interpreter::trace_diagnosis::EventDiagnosis;
use crate::interpreter::upstream_nil_link::link_upstream_nil;
use crate::interpreter::{Interpreter, ResourceUsage, RuntimeMetrics};
use crate::semantic::AbsenceMetadata;
use crate::types::value_protocol::{exact_terms, value_to_protocol, ProtocolNode, ProtocolValue};
use crate::types::{Value, ValueData};
use serde_json::{json, Map, Value as Json};

/// Version of the top-level `--json` envelope. Bump only on a breaking
/// change (field removal or rename); purely additive fields keep the same
/// version. See `docs/dev/agent-cli-output-contract.md`.
///
/// 3: `aiDiagnostic` classifies only (`category`, `repair`, `word`,
/// `family`) — `kind` became `category`, `recoverability` gave way to the
/// registry's `repair`, and the copies of `nextChecks`/`candidates`/
/// `resourceLimit` went; an error's trace event no longer repeats the
/// top-level `diagnosis`.
pub(crate) const SCHEMA_VERSION: u64 = 3;

pub(crate) struct Report {
    pub status: &'static str,
    pub stack: Json,
    /// Human display strings for the stack, bottom to top — the same text
    /// the GUI's Stack projection renders. Not the text `PRINT` writes: a
    /// top-level String is displayed quoted (`'a'`) and printed raw (`a`),
    /// see `types::display::format_for_output`. Carried in the JSON envelope as
    /// `stackDisplay` so agents and the SKILL.md generator can show
    /// "code → expected stack" pairs without re-deriving display rules.
    pub stack_display: Vec<String>,
    pub output: Vec<String>,
    pub message: Option<String>,
    pub diagnosis: Option<DebugDiagnosis>,
    pub ai_diagnostic: Option<AiDiagnosticPayload>,
    pub error_flow_trace: Vec<ErrorFlowEvent>,
    pub runtime_metrics: RuntimeMetrics,
    /// What the run spent of the budgets that can refuse it. Read from the
    /// counters the ceilings read, so it cannot disagree with them.
    pub resource_usage: ResourceUsage,
    /// Per-word contract declarations checked against inference
    /// (`check --contract`, P2). `None` unless the user opted in; additive
    /// field. Prebuilt JSON so `report` stays decoupled from the declaration
    /// types.
    pub contract_decls: Option<Json>,
    /// Which stack slots an error report dropped the values of, and why
    /// (`agent::error_stack`). `None` whenever nothing was dropped, which is
    /// every success and every ordinary error.
    pub stack_elided: Option<Json>,
    /// Canonical `#`-prefixed 64-hex-char BLAKE3 digest of the whole
    /// observation (`status` / stack / output / user-dictionary identities /
    /// error category) — `agent::observation_digest`, Phase 1 of
    /// `docs/dev/competitive-advantage-work-order-2026-08.md`.
    pub observation_digest: String,
    /// The execution receipt (`agent::execution_receipt`, Phase 4 of
    /// `docs/dev/auditable-kernel-work-order-2026-09.md`): source digest,
    /// engine version, registry digest, limit profile, outcome status,
    /// `observation_digest` and `resourceUsage`, bundled and folded into one
    /// more digest — the material a third party needs to verify "this
    /// source, on this engine, under these limits, produced this outcome"
    /// rather than merely being told what the outcome was. `None` for
    /// `check`/`infer-contracts`, which never execute and so have nothing to
    /// receipt.
    pub receipt: Option<Json>,
    /// The outcome id (`spec/outcomes.json`) a `compute` run produced —
    /// `value`, `nil:<reason>` or `error:<category>`, from
    /// [`Report::outcome_id`]. `None` for `check`, which never runs, and for
    /// a run the id cannot name; the JSON then carries no `outcome` at all.
    pub outcome: Option<String>,
}

impl Report {
    /// The outcome id this report names, in the vocabulary `outcomes`
    /// predicts in, so a prediction and a run compare by membership.
    ///
    /// `status` separates a value from an error but folds a reasoned absence
    /// into `ok`, so LANG.FAILURE's three results were reconstructable only by
    /// reading the top stack node's `semantics.absence.reason`. An error is its
    /// category (`aiDiagnostic.category`, else `diagnosis.why`), a NIL on top
    /// is its reason, anything else — an empty stack included — is a value. A
    /// report this cannot classify (a NIL with no reason, an error naming no
    /// category) has no id rather than a guessed one.
    pub(crate) fn outcome_id(&self) -> Option<String> {
        match self.status {
            "error" => {
                let category = match self
                    .ai_diagnostic
                    .as_ref()
                    .and_then(|ai| ai.category.as_deref())
                {
                    Some(category) => Some(category),
                    None => self.diagnosis.as_ref().map(|d| d.why.as_protocol_str()),
                };
                category
                    .filter(|category| !category.is_empty())
                    .map(|category| format!("error:{category}"))
            }
            "ok" => match self.stack.as_array().and_then(|stack| stack.last()) {
                Some(top) if top["type"] == "nil" => top["semantics"]["absence"]["reason"]
                    .as_str()
                    .filter(|reason| !reason.is_empty())
                    .map(|reason| format!("nil:{reason}")),
                _ => Some("value".to_string()),
            },
            _ => None,
        }
    }

    pub(crate) fn to_json(&self) -> Json {
        self.document(self.stack.clone())
    }

    /// [`Report::to_json`], moving the stack's JSON into the document instead
    /// of copying it: the stack is most of a large report, and `json!` would
    /// rebuild it node by node through `serde_json::to_value`.
    pub(crate) fn into_json(mut self) -> Json {
        let stack = std::mem::take(&mut self.stack);
        self.document(stack)
    }

    fn document(&self, stack: Json) -> Json {
        let mut doc = json!({
            "schemaVersion": SCHEMA_VERSION,
            "status": self.status,
            "stackDisplay": self.stack_display,
            "output": self.output,
            "message": self.message,
            "diagnosis": self.diagnosis.as_ref().map(diagnosis_json),
            "aiDiagnostic": self.ai_diagnostic.as_ref().map(ai_payload_json),
            "runtimeMetrics": runtime_metrics_json(&self.runtime_metrics),
            "resourceUsage": resource_usage_json(&self.resource_usage),
            "contractDecls": self.contract_decls,
            "stackElided": self.stack_elided,
            "observationDigest": self.observation_digest,
            "receipt": self.receipt,
        });
        doc["stack"] = stack;
        let (trace, trace_elided) =
            super::error_stack::bounded_error_flow_trace_json(&self.error_flow_trace);
        doc["errorFlowTrace"] = trace;
        if let Some(elided) = trace_elided {
            doc["errorFlowTraceElided"] = elided;
        }
        if let Some(outcome) = &self.outcome {
            doc["outcome"] = json!(outcome);
        }
        doc
    }
}

pub(crate) fn stack_json(interp: &Interpreter) -> Json {
    let nodes: Vec<Json> = interp
        .get_stack()
        .iter()
        .map(|value| protocol_node_json(&value_to_protocol(value)))
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
    json!({
        "when": diagnosis.when.as_protocol_str(),
        "why": diagnosis.why.as_protocol_str(),
        "summary": diagnosis.summary,
        "where": Json::Object(where_obj),
        "evidence": diagnosis.evidence,
        "nextChecks": diagnosis.next_checks.iter().map(check_json).collect::<Vec<_>>(),
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
    let mut out = json!({
        "resource": facts.resource,
        "limit": facts.limit,
        "observed": facts.observed,
    });
    // Emitted only where it exists, never as a null. A ceiling whose `observed`
    // is a real size says everything it has to say without it, and a key that
    // is present-but-empty invites a reader to treat "no progress recorded" as
    // "no progress made".
    if let Some(progress) = facts.progress {
        out["progress"] = json!({
            "completed": progress.completed,
            "total": progress.total,
            "unit": progress.unit,
        });
    }
    out
}

pub(crate) fn ai_payload_json(payload: &AiDiagnosticPayload) -> Json {
    let mut obj = Map::new();
    obj.insert("category".into(), json!(payload.category));
    // As in spec/outcomes.json: present only as `program`; absent means the
    // operand is what is wrong.
    if let Some(repair) = payload.repair {
        obj.insert("repair".into(), json!(repair));
    }
    obj.insert("word".into(), json!(payload.word));
    obj.insert("family".into(), json!(payload.family));
    Json::Object(obj)
}

fn absence_json(absence: &AbsenceMetadata) -> Json {
    let mut obj = Map::new();
    if let Some(reason) = &absence.reason {
        obj.insert("reason".into(), json!(reason.as_protocol_str()));
    }
    if let Some(detail) = &absence.detail {
        obj.insert("detail".into(), json!(detail.as_str()));
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
    // A NIL has no diagnosis anywhere else, so its event carries one. An
    // ERROR's is the report's top-level `diagnosis` — built from this very
    // event (`failed_run_diagnosis`) — and sending it here as well
    // doubled every error report for no information.
    if let (ErrorFlowEventKind::NilProduced, Some(diagnosis)) = (&event.kind, &event.diagnosis) {
        obj.insert("diagnosis".into(), diagnosis_json(diagnosis));
    }
    Json::Object(obj)
}

pub(crate) fn runtime_metrics_json(metrics: &RuntimeMetrics) -> Json {
    // Diagnostics only: these counters describe *how* the runtime went about
    // its work — which cache answered, which fast path fired. Reading them
    // changes no result, and no Word reads them.
    json!({
        "compiledPlanBuildCount": metrics.compiled_plan_build_count,
        "compiledPlanCacheHitCount": metrics.compiled_plan_cache_hit_count,
        "compiledPlanCacheMissCount": metrics.compiled_plan_cache_miss_count,
        "scalarFastpathCount": metrics.scalar_fastpath_count,
        "resolveCacheHitCount": metrics.resolve_cache_hit_count,
        "resolveCacheMissCount": metrics.resolve_cache_miss_count,
        "resolveCacheInvalidationCount": metrics.resolve_cache_invalidation_count,
        "tailCallJumpCount": metrics.tail_call_jump_count,
    })
}

/// What the run spent of the budgets that could have refused it, in the keys
/// the host declares those budgets under.
///
/// Separate from `runtimeMetrics` on purpose. Every key here names a
/// `mcp.limits` key and carries the same number the ceiling compared against,
/// so an agent can subtract one from the other and know what it has left; no
/// key here is an internal routing counter, and no counter there is a budget.
pub(crate) fn resource_usage_json(usage: &ResourceUsage) -> Json {
    json!({
        "executionSteps": usage.execution_steps,
        "numericWork": usage.numeric_work,
        "collectionWork": usage.collection_work,
    })
}

/// JSON rendering of a `ProtocolNode` — the same shape `protocol_to_js`
/// produces for the GUI: `{ type, value, semantics? }`.
pub(crate) fn protocol_node_json(node: &ProtocolNode) -> Json {
    let mut obj = Map::new();
    obj.insert("semantics".into(), semantics_json(&node.semantics));
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
        ProtocolValue::Record { keys, values } => json!({
            "keys": keys.iter().map(protocol_node_json).collect::<Vec<_>>(),
            "values": values.iter().map(protocol_node_json).collect::<Vec<_>>(),
        }),
    };
    obj.insert("value".into(), value);
    Json::Object(obj)
}

/// JSON rendering of the per-value `semantics` block — the one rendering:
/// the WASM boundary converts this same value (`value_semantics_to_js`)
/// rather than building its own.
pub(crate) fn semantics_json(value: &Value) -> Json {
    let mut obj = Map::new();
    if let Some(truth) = value.truth_value() {
        obj.insert("truthValue".into(), json!(truth));
    }
    if let Some(absence) = value.normalized_absence_metadata() {
        obj.insert("absence".into(), absence_json(&absence));
    }
    if matches!(value.data, ValueData::ExactScalar(_)) {
        obj.insert("approximate".into(), json!(true));
    }
    // The terms a consumer computes with. The rendering a reader takes in is
    // the stack display, which writes these same terms (`sqrt(2)`).
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

/// Assemble the report for a completed execution without performing host I/O.
/// The typed agent API and the terminal renderer share this boundary.
///
/// `source` is the exact program text that was run — carried through only to
/// name it in the execution receipt (`Report::receipt`); nothing here
/// re-parses or re-executes it.
///
/// `stack_budget` is the byte budget a successful result's stack may take
/// before its largest slots are elided (`agent::error_stack`); `None` sends
/// the stack whole.
pub(crate) fn completed_run_report(
    interp: &Interpreter,
    result: crate::error::Result<()>,
    trace: Vec<ErrorFlowEvent>,
    output: Vec<String>,
    source: &str,
    stack_budget: Option<usize>,
) -> Report {
    match result {
        Ok(()) => {
            let digest = observation_digest(ObservationDigestInput {
                status: "ok",
                stack: interp.get_stack(),
                output: &output,
                user_words: &user_word_identities(interp),
                error_category: None,
            });
            let resource_usage = interp.resource_usage();
            let receipt = build_receipt(
                source,
                interp.runtime_limits(),
                interp.max_execution_steps(),
                "ok",
                &resource_usage,
                &digest,
            );
            // The answer is the stack, so a success is sent whole wherever
            // the host can take it; under a budget, the slots that do not
            // fit are replaced by a record of what they were rather than
            // the whole result being refused (`agent::error_stack`).
            let residue = match stack_budget {
                Some(budget) => super::error_stack::elided_value_stack(interp, budget),
                None => super::error_stack::whole_stack(interp),
            };
            Report {
                status: "ok",
                stack: residue.stack,
                stack_display: residue.stack_display,
                output,
                message: None,
                diagnosis: None,
                ai_diagnostic: None,
                error_flow_trace: trace,
                runtime_metrics: interp.runtime_metrics(),
                resource_usage,
                contract_decls: None,
                stack_elided: residue.elided,
                observation_digest: digest,
                receipt: Some(receipt),
                outcome: None,
            }
        }
        Err(err) => {
            let message = err.to_string();
            let diagnosis = failed_run_diagnosis(interp, &err, &trace);
            let category = ErrorCategory::from_error(&err);
            error_report(
                interp,
                &diagnosis,
                category.as_ref(),
                message,
                output,
                trace,
                Some(source),
            )
        }
    }
}

/// The top-level diagnosis of a run that ended in `err`: the last one the
/// trace carries, else one built from the error itself. Shared with the WASM
/// boundary, so the GUI's `aiDiagnostic` names the failure the CLI's does.
pub(crate) fn failed_run_diagnosis(
    interp: &Interpreter,
    err: &crate::error::AjisaiError,
    trace: &[ErrorFlowEvent],
) -> DebugDiagnosis {
    let stack_len = interp.get_stack().len();
    let mut diagnosis = trace
        .iter()
        .rev()
        .find_map(|event| event.diagnosis.as_ref().map(EventDiagnosis::to_diagnosis))
        .unwrap_or_else(|| DebugDiagnosis::from_error(err, None, stack_len, stack_len));
    // A NIL that flowed downstream fails at the Word that *received* it, so
    // the top-level diagnosis names that Word and not the cause. Give the top
    // level a link back to the producing node rather than leaving the cause
    // reachable only by walking `errorFlowTrace`.
    link_upstream_nil(&mut diagnosis, trace);
    diagnosis
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
        let semantics = semantics_json(&value);

        assert_eq!(semantics["approximate"], true);
        assert_eq!(semantics["exactTerms"][0]["numerator"], "1");
        assert_eq!(semantics["exactTerms"][0]["denominator"], "1");
        assert_eq!(semantics["exactTerms"][0]["radicand"], "2");
        // The rendering is the stack display's, not a second field here.
        assert!(semantics.get("exactDisplay").is_none());
    }
}
