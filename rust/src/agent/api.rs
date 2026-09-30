//! Typed, source-only boundary for agent hosts.
//!
//! Host adapters should consume this API instead of reproducing interpreter
//! execution and report assembly. It performs no filesystem or terminal I/O.

use super::report::{completed_run_report, Report};
use super::{
    contract_decl, contract_report, error_report, outcome_report, print_payloads, resolve_words,
};
use crate::error::ErrorCategory;
use crate::interpreter::debug_diagnosis::{DebugDiagnosis, ErrorPhase};
use crate::interpreter::runtime_limits::DEFAULT_MAX_NESTING_DEPTH;
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
    max_nesting_depth: DEFAULT_MAX_NESTING_DEPTH,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComputeOptions {
    pub step_limit: Option<usize>,
    pub runtime_limits: Option<RuntimeLimits>,
}

impl ComputeOptions {
    /// The agent profile: [`LOCAL_AGENT_RUNTIME_LIMITS`], with `step_limit`
    /// overriding the execution step budget when given.
    pub const fn agent(step_limit: Option<usize>) -> Self {
        ComputeOptions {
            step_limit,
            runtime_limits: Some(LOCAL_AGENT_RUNTIME_LIMITS),
        }
    }

    /// Put `interp` under these ceilings; an absent one keeps the
    /// interpreter's own default.
    pub(crate) fn apply(&self, interp: &mut Interpreter) {
        if let Some(limits) = self.runtime_limits {
            interp.set_runtime_limits(limits);
        }
        if let Some(limit) = self.step_limit {
            interp.set_max_execution_steps(limit);
        }
    }

    /// A fresh interpreter under these ceilings.
    pub(crate) fn interpreter(&self) -> Interpreter {
        let mut interp = Interpreter::new();
        self.apply(&mut interp);
        interp
    }
}

pub struct AgentResponse {
    report: Report,
}

pub struct ContractResponse {
    /// The inferred contracts, or the source-form error that stopped
    /// inference before it began — the same report `check` gives.
    result: Result<serde_json::Value, Box<AgentResponse>>,
}

impl ContractResponse {
    /// The agent envelope around the inferred contracts, or the error report.
    pub fn to_json(&self) -> serde_json::Value {
        match &self.result {
            Ok(contracts) => serde_json::json!({
                "schemaVersion": super::report::SCHEMA_VERSION,
                "status": "ok",
                "contracts": contracts,
            }),
            Err(report) => report.to_json(),
        }
    }

    pub fn exit_code(&self) -> i32 {
        match &self.result {
            Ok(_) => 0,
            Err(report) => report.exit_code(),
        }
    }
}

impl AgentResponse {
    /// A `compute` report, carrying the outcome id it names.
    fn computed(mut report: Report) -> Self {
        report.outcome = report.outcome_id();
        AgentResponse { report }
    }

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
        // Applied even though tokenization never gets far enough to spend any
        // of it: the receipt names the profile the caller asked for, not the
        // interpreter's built-in default, and the two can differ.
        let interp = options.interpreter();
        return AgentResponse::computed(error_report(
            &interp,
            &diagnosis,
            Some(&ErrorCategory::MalformedSource),
            message,
            Vec::new(),
            Vec::new(),
            Some(source),
        ));
    }

    let mut interp = options.interpreter();
    let result = interp.execute(source).await;
    let trace = interp.drain_error_flow_trace();
    let output = print_payloads(&interp);
    AgentResponse::computed(completed_run_report(&interp, result, trace, output, source))
}

/// The source-form gate every execution-free operation shares: a source that
/// does not tokenize, or whose vector delimiters do not balance, is
/// `malformedSource` before anything else can be said about it. `Err` carries
/// the finished error report.
///
/// Both refusals come from the one call: the tokenizer's structural phase
/// (`spec/grammar.json`, structuralValidation) runs on every `tokenize`
/// result, so an unbalanced bracket is a tokenize failure here exactly as it
/// is to `compute` — one phase, one message, for every tool.
///
/// `check` had this gate and `infer_contracts` did not, so `[ 1 2` was a
/// `malformedSource` error to `compute`, `check` and `outcomes` and an `ok`
/// with no contracts to inference — four tools, two answers about one source.
fn tokens_of_well_formed(
    interp: &Interpreter,
    source: &str,
) -> Result<Vec<crate::types::Token>, Box<AgentResponse>> {
    let message = match crate::tokenizer::tokenize(source) {
        Ok(tokens) => return Ok(tokens),
        Err(message) => message,
    };
    // The same category `run` reports for an unbalanced bracket.
    let category = ErrorCategory::MalformedSource;
    let diagnosis = DebugDiagnosis::from_error_category(
        ErrorPhase::Tokenize,
        None,
        Some(&category),
        None,
        0,
        0,
        Some(message.clone()),
    );
    Err(Box::new(AgentResponse {
        report: error_report(
            interp,
            &diagnosis,
            Some(&category),
            message,
            Vec::new(),
            Vec::new(),
            None,
        ),
    }))
}

/// Validate source without executing it and return the standard report shape.
pub fn check(source: &str, verify_contracts: bool) -> AgentResponse {
    let interp = Interpreter::new();
    let tokens = match tokens_of_well_formed(&interp, source) {
        Ok(tokens) => tokens,
        Err(report) => return *report,
    };
    let resolved = resolve_words(&interp, &tokens);
    let unknown = &resolved.unknown;
    if let Some(first) = unknown.first() {
        let mut message = format!("Unknown words: {}", unknown.join(", "));
        if !resolved.bound_elsewhere.is_empty() {
            message.push_str(&format!(
                ". {} is bound in another frame: a binding is reachable in the frame that made it \
                 and in the blocks written there, never inside a Word it calls — pass the value \
                 as an operand instead",
                resolved.bound_elsewhere.join(", ")
            ));
        }
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
                None,
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
            // `check` never executes, so there is nothing to receipt —
            // see `Report::receipt`'s doc comment.
            receipt: None,
            outcome: None,
        },
    }
}

/// Infer user-Word contracts without executing definitions or top-level code.
pub fn infer_contracts(source: &str) -> ContractResponse {
    if let Err(report) = tokens_of_well_formed(&Interpreter::new(), source) {
        return ContractResponse {
            result: Err(report),
        };
    }
    let reports = contract_report::report_contracts(source);
    ContractResponse {
        result: Ok(contract_report::reports_json(&reports)),
    }
}

pub struct OutcomesResponse {
    report: outcome_report::OutcomeReport,
}

impl OutcomesResponse {
    pub fn to_json(&self) -> serde_json::Value {
        self.report.to_json()
    }
}

/// Predict the finite set of outcome ids `source` could produce without
/// executing it (`docs/dev/auditable-kernel-work-order-2026-09.md` Phase 5).
/// Always succeeds — an unresolvable program still has an exact, single
/// predicted outcome (`error:malformedSource` or `error:unknownWord`); see `outcome_report::predict_outcomes`.
/// Predict `source`'s outcome set under `options`' ceilings, without
/// executing it. The reported `limitProfile` is the profile the prediction
/// assumed, so it must be the one the caller would compute under.
pub fn predict_outcomes(source: &str, options: ComputeOptions) -> OutcomesResponse {
    OutcomesResponse {
        report: outcome_report::predict_outcomes(source, &options),
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
            "0 11 RANGE",
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

    /// `outcome` names a run in `outcomes`' own vocabulary — a value, a NIL
    /// by its reason, an error by its category — so a prediction and a run
    /// compare by membership. `check` never runs, so it names none.
    #[tokio::test]
    async fn compute_names_the_outcome_id_it_produced() {
        for (source, expected) in [
            ("1 2 ADD", "value"),
            ("", "value"),
            ("1 0 DIV", "nil:divisionByZero"),
            ("FROBNICATE", "error:unknownWord"),
            ("[ 1 2", "error:malformedSource"),
        ] {
            let json = compute(source, ComputeOptions::default()).await.to_json();
            assert_eq!(json["outcome"], expected, "{source}");
        }
        assert!(check("1 2 ADD", true).to_json().get("outcome").is_none());
    }

    #[test]
    fn check_is_execution_free_and_structured() {
        let response = check("[ [ 1 ] ADD ] 'INC' DEF 'must-not-print' PRINT", true);
        let json = response.to_json();
        assert_eq!(response.exit_code(), 0);
        assert_eq!(json["status"], "ok");
        assert_eq!(json["output"], serde_json::json!([]));
    }

    #[test]
    fn infer_contracts_returns_a_common_agent_envelope() {
        let response = infer_contracts("[ [ 1 ] ADD ] 'INC' DEF").to_json();
        assert_eq!(response["status"], "ok");
        assert_eq!(response["contracts"][0]["name"], "INC");
    }

    /// Source that does not read is `malformedSource` to every execution-free
    /// operation, exactly as `check` reports it — not an empty success.
    #[test]
    fn infer_contracts_reports_malformed_source_as_check_does() {
        for source in ["[ 1 2", "1 2 ]", "'unterminated"] {
            let inferred = infer_contracts(source);
            let checked = check(source, false);
            let inferred_json = inferred.to_json();
            assert_eq!(inferred_json["status"], "error", "{source}");
            assert_eq!(
                inferred_json["aiDiagnostic"]["category"], "malformedSource",
                "{source}"
            );
            assert_eq!(inferred_json, checked.to_json(), "{source}");
            assert_eq!(inferred.exit_code(), checked.exit_code(), "{source}");
        }
    }

    /// A body naming a Word nothing defines raises `unknownWord` when it
    /// runs, so its contract is `partial`, never `total`.
    #[test]
    fn an_unresolved_word_makes_a_contract_partial() {
        let response = infer_contracts("[ FOO ] 'W' DEF").to_json();
        let contract = &response["contracts"][0];
        assert_eq!(contract["name"], "W");
        assert_eq!(contract["partiality"], "partial");
        assert_eq!(contract["gaps"], serde_json::json!(["gap.unresolvedWord"]));
        // A body whose every name resolves keeps the registry's derivation.
        let resolved = infer_contracts("[ 1 ADD ] 'W' DEF").to_json();
        assert_eq!(resolved["contracts"][0]["partiality"], "total");
    }
}
