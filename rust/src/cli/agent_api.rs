//! Typed, source-only boundary for agent hosts.
//!
//! Host adapters should consume this API instead of reproducing interpreter
//! execution and report assembly. It performs no filesystem or terminal I/O.

use super::{error_report, print_payloads, report::Report, run_render, Opts};
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
}
