//! Typed, source-only boundary for agent hosts.
//!
//! Host adapters should consume this API instead of reproducing interpreter
//! execution and report assembly. It performs no filesystem or terminal I/O.

use super::{
    check_structure, contract_decl, contract_report, error_report, print_payloads, report::Report,
    resolve_words, run_render, Opts,
};
use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::{DebugDiagnosis, ErrorPhase};
use crate::interpreter::{Interpreter, RuntimeLimits};

/// Tighter internal-cost profile for untrusted, agent-generated programs.
pub const LOCAL_AGENT_RUNTIME_LIMITS: RuntimeLimits = RuntimeLimits {
    max_materialized_elements: 100_000,
    max_source_bytes: 64 * 1024,
    max_numeric_literal_digits: 4_096,
    max_numeric_work: 10_000_000,
    // Twice the numeric budget, which is what makes the two bound the same
    // amount of *time* rather than the same number of units: the numeric
    // meter's slowest unbounded path charges 14,465 units/ms and the collection
    // meter's charges 30,800, so 10M and 20M both buy about 0.7 s. Their sum
    // leaves `wallTimeMs` 5,000 a 3.7x margin. Derived in
    // `docs/dev/collection-word-billing-2026-08-13.md` §6.
    max_collection_work: 20_000_000,
    max_bigint_bits: 262_144,
    // Not a round number: 512 terms is 3.0% of `responseBytes` in `exactTerms`
    // (4,096 was 26.5% — a quarter of the whole response for one value), and
    // sixteen doublings past the point where the continued fraction stops being
    // readable at all. It is also *live*: the doubling that crosses it charges
    // 2,113,536 units, a fifth of `max_numeric_work`, so this ceiling names
    // itself instead of being pre-empted. At 4,096 it could not — the doubling
    // that would first exceed it costs 16,799,744 against a 10,000,000 budget,
    // so `numericWork` always answered first and this limit was a claim rather
    // than a control. See `profile_liveness_tests`.
    max_algebraic_terms: 512,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComputeOptions {
    pub step_limit: Option<usize>,
    pub runtime_limits: Option<RuntimeLimits>,
}

pub struct AgentResponse {
    report: Report,
}

pub struct ContractResponse {
    contracts: serde_json::Value,
}

impl ContractResponse {
    /// Common agent envelope. The native schema-1 CLI keeps emitting the bare
    /// `contracts` array for compatibility; new hosts should use this shape.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "schemaVersion": super::report::SCHEMA_VERSION,
            "status": "ok",
            "contracts": self.contracts,
        })
    }

    pub(crate) fn contracts(&self) -> &serde_json::Value {
        &self.contracts
    }
}

impl AgentResponse {
    pub fn exit_code(&self) -> i32 {
        if self.report.status == "ok" {
            0
        } else {
            1
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        self.report.to_json()
    }

    pub(crate) fn report(&self) -> &Report {
        &self.report
    }
}

/// Execute one source document and return the same structured observation the
/// CLI emits, without creating a file or writing stdout/stderr.
pub async fn compute(source: &str, options: ComputeOptions) -> AgentResponse {
    let opts = Opts {
        json: true,
        contract: false,
        step_limit: options.step_limit,
    };
    if let Err(message) = crate::tokenizer::tokenize(source) {
        let diagnosis = DebugDiagnosis::from_error_category(
            ErrorPhase::Tokenize,
            None,
            Some(&ErrorCategory::MalformedSource),
            None,
            0,
            0,
            Some(message.clone()),
        );
        let interp = Interpreter::new();
        return AgentResponse {
            report: error_report(
                &interp,
                &diagnosis,
                None,
                message,
                Vec::new(),
                Vec::new(),
                &opts,
            ),
        };
    }

    let mut interp = Interpreter::new();
    if let Some(limits) = options.runtime_limits {
        interp.set_runtime_limits(limits);
    }
    if let Some(limit) = options.step_limit {
        interp.set_max_execution_steps(limit);
    }
    let result = interp.execute(source).await;
    let trace = interp.drain_error_flow_trace();
    let output = print_payloads(&interp);
    AgentResponse {
        report: run_render::completed_run_report(&interp, result, trace, output, &opts),
    }
}

/// Validate source without executing it and return the standard report shape.
pub fn check(source: &str, verify_contracts: bool) -> AgentResponse {
    let opts = Opts {
        json: true,
        contract: verify_contracts,
        step_limit: None,
    };
    let interp = Interpreter::new();
    let tokens = match crate::tokenizer::tokenize(source) {
        Ok(tokens) => tokens,
        Err(message) => {
            let diagnosis = DebugDiagnosis::from_error_category(
                ErrorPhase::Tokenize,
                None,
                Some(&ErrorCategory::MalformedSource),
                None,
                0,
                0,
                Some(message.clone()),
            );
            return AgentResponse {
                report: error_report(
                    &interp,
                    &diagnosis,
                    None,
                    message,
                    Vec::new(),
                    Vec::new(),
                    &opts,
                ),
            };
        }
    };
    if let Err(message) = check_structure(&tokens) {
        let category = ErrorCategory::StructureError;
        let diagnosis = DebugDiagnosis::from_error_category(
            ErrorPhase::ParseStructure,
            None,
            Some(&category),
            None,
            0,
            0,
            Some(message.clone()),
        );
        return AgentResponse {
            report: error_report(
                &interp,
                &diagnosis,
                Some(&category),
                message,
                Vec::new(),
                Vec::new(),
                &opts,
            ),
        };
    }
    let resolved = resolve_words(&interp, &tokens);
    let unknown = &resolved.unknown;
    if let Some(first) = unknown.first() {
        let message = format!("Unknown words: {}", unknown.join(", "));
        let category = ErrorCategory::UnknownWord;
        let mut diagnosis = DebugDiagnosis::from_error_category(
            ErrorPhase::ResolveWord,
            Some(first),
            Some(&category),
            None,
            0,
            0,
            Some(format!("Unknown word: {first}")),
        );
        diagnosis
            .evidence
            .push(format!("unknownWords={}", unknown.join(",")));
        diagnosis.with_user_vocabulary(resolved.locally_defined.iter().map(String::as_str));
        return AgentResponse {
            report: error_report(
                &interp,
                &diagnosis,
                Some(&category),
                message,
                Vec::new(),
                Vec::new(),
                &opts,
            ),
        };
    }

    let contract_decls = verify_contracts.then(|| contract_decl::check_contract_decls(source));
    let contract_failed = contract_decls
        .as_ref()
        .is_some_and(|result| result.violated);
    let status = if contract_failed { "error" } else { "ok" };
    // `check` never executes, so the observation is the degenerate one: no
    // stack, no output, no dictionary — but it still folds to a stable digest
    // that a caller can compare across two identical `check` calls.
    let digest = super::observation_digest::observation_digest(
        super::observation_digest::ObservationDigestInput {
            status,
            stack: &[],
            output: &[],
            user_words: &[],
            error_category: None,
        },
    );
    AgentResponse {
        report: Report {
            status,
            stack: serde_json::Value::Array(Vec::new()),
            stack_display: Vec::new(),
            output: Vec::new(),
            message: None,
            diagnosis: None,
            ai_diagnostic: None,
            error_flow_trace: Vec::new(),
            runtime_metrics: crate::interpreter::RuntimeMetrics::default(),
            resource_usage: crate::interpreter::ResourceUsage::default(),
            contract_decls: contract_decls.as_ref().map(|result| result.to_json()),
            stack_elided: None,
            observation_digest: digest,
        },
    }
}

/// Infer user-Word contracts without executing definitions or top-level code.
pub fn infer_contracts(source: &str) -> ContractResponse {
    let reports = contract_report::report_contracts(source);
    ContractResponse {
        contracts: contract_report::reports_json(&reports),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn compute_is_source_only_and_returns_the_cli_envelope() {
        let response = compute("[ 2 ] SQRT", ComputeOptions::default()).await;
        let json = response.to_json();
        assert_eq!(response.exit_code(), 0);
        assert_eq!(json["status"], "ok");
        assert_eq!(
            json["stack"][0]["value"][0]["semantics"]["exactTerms"][0]["radicand"],
            "2"
        );
    }

    #[tokio::test]
    async fn compute_preserves_structured_language_errors() {
        let response = compute("FROBNICATE", ComputeOptions::default()).await;
        let json = response.to_json();
        assert_eq!(response.exit_code(), 1);
        assert_eq!(json["status"], "error");
        assert_eq!(json["diagnosis"]["why"], "typoOrUnknownName");
    }

    #[tokio::test]
    async fn compute_applies_injected_internal_cost_limits() {
        let response = compute(
            "[ 0 11 ] RANGE",
            ComputeOptions {
                runtime_limits: Some(RuntimeLimits {
                    max_materialized_elements: 10,
                    ..RuntimeLimits::default()
                }),
                ..ComputeOptions::default()
            },
        )
        .await;
        let json = response.to_json();
        assert_eq!(json["status"], "ok");
        assert_eq!(
            json["stack"][0]["semantics"]["absence"]["reason"],
            "spaceExhausted"
        );
    }

    #[test]
    fn check_is_execution_free_and_structured() {
        let response = check("[ [ 1 ] + ] 'INC' DEF 'must-not-print' PRINT", true);
        let json = response.to_json();
        assert_eq!(response.exit_code(), 0);
        assert_eq!(json["status"], "ok");
        assert_eq!(json["output"], serde_json::json!([]));
    }

    #[test]
    fn infer_contracts_returns_a_common_agent_envelope() {
        let response = infer_contracts("[ [ 1 ] + ] 'INC' DEF").to_json();
        assert_eq!(response["status"], "ok");
        assert_eq!(response["contracts"][0]["name"], "INC");
    }
}
