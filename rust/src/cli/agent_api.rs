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
use crate::interpreter::Interpreter;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComputeOptions {
    pub step_limit: Option<usize>,
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
    let unknown = resolve_words(&interp, &tokens);
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
    AgentResponse {
        report: Report {
            status: if contract_failed { "error" } else { "ok" },
            stack: serde_json::Value::Array(Vec::new()),
            stack_display: Vec::new(),
            output: Vec::new(),
            message: None,
            diagnosis: None,
            ai_diagnostic: None,
            error_flow_trace: Vec::new(),
            runtime_metrics: crate::interpreter::RuntimeMetrics::default(),
            contract_decls: contract_decls.as_ref().map(|result| result.to_json()),
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

    #[test]
    fn check_is_execution_free_and_structured() {
        let response = check("{ [ 1 ] + } 'INC' DEF 'must-not-print' PRINT", true);
        let json = response.to_json();
        assert_eq!(response.exit_code(), 0);
        assert_eq!(json["status"], "ok");
        assert_eq!(json["output"], serde_json::json!([]));
    }

    #[test]
    fn infer_contracts_returns_a_common_agent_envelope() {
        let response = infer_contracts("{ [ 1 ] + } 'INC' DEF").to_json();
        assert_eq!(response["status"], "ok");
        assert_eq!(response["contracts"][0]["name"], "INC");
    }
}
