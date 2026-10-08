//! Typed, source-only boundary for agent hosts.
//!
//! Host adapters should consume this API instead of reproducing interpreter
//! execution and report assembly. It performs no filesystem or terminal I/O.

use super::report::{completed_run_report, Report};
use super::{
    contract_decl, contract_report, contract_violation, error_report, outcome_report,
    print_payloads, resolve_words,
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

/// The execution-step budget of the agent profile: the one the MCP server
/// applies (`tools/mcp-server/index.js`, `LIMITS.executionSteps`, which
/// threads it explicitly on every call) and the one `ajisai agent compute`
/// runs under unless `--step-limit` says otherwise. `docs/dev/mcp-host-profiles.md`
/// compares it with the playground's derived budget, which is 120 times
/// larger: a block iteration is one step per element, so under this profile
/// MAP / FILTER / FOLD walk tens of thousands of elements and a vector
/// operation (`V V ADD`) handles the rest.
pub const LOCAL_AGENT_EXECUTION_STEPS: usize = 100_000;

/// Byte budget for a *successful* agent-profile result's `stack` and
/// `stackDisplay`, together, past which the largest slots are elided
/// (`agent::error_stack`) rather than sent.
///
/// A success used to be sent whole or not at all: a 7,000-element vector left
/// on the stack became the host's `responseTooLarge`, and the caller learned
/// that its answer was too big and nothing else — not what was on the stack,
/// not how big, not that the small values beside it were fine. Elided, the
/// same result arrives with every affordable slot in full and the oversized
/// one replaced by a record of what it was, so the caller can see that it
/// left an intermediate value behind and fix that.
///
/// Sized against the MCP host's 1 MiB `responseBytes`, which bounds the
/// response as sent: the envelope twice (structured, and mirrored into a text
/// block where every quote is escaped), measured at 2.2x the envelope. A
/// stack of this many bytes leaves that response under the ceiling with
/// about 6% to spare, and still admits a 5,000-element vector of small
/// integers in full (433 KB). The estimate the budget is compared against is
/// `error_stack::node_wire_bytes`, calibrated on the real rendering.
pub const AGENT_STACK_BUDGET_BYTES: usize = 440 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComputeOptions {
    pub step_limit: Option<usize>,
    pub runtime_limits: Option<RuntimeLimits>,
    /// Byte budget for a successful result's stack, past which the largest
    /// slots are elided. `None` sends every success whole, whatever its size
    /// — the trusted profile, for a host with no response ceiling.
    pub stack_budget_bytes: Option<usize>,
}

impl ComputeOptions {
    /// The agent profile: [`LOCAL_AGENT_RUNTIME_LIMITS`] and
    /// [`LOCAL_AGENT_EXECUTION_STEPS`], with `step_limit` overriding the
    /// step budget when given, and [`AGENT_STACK_BUDGET_BYTES`] bounding
    /// what a success sends. The profile used to leave the step budget at
    /// the interpreter's derived default when none was given, so `ajisai
    /// agent compute` ran 12,180,000 steps where the MCP server, which names
    /// the same profile, ran 100,000.
    pub const fn agent(step_limit: Option<usize>) -> Self {
        ComputeOptions {
            step_limit: Some(match step_limit {
                Some(limit) => limit,
                None => LOCAL_AGENT_EXECUTION_STEPS,
            }),
            runtime_limits: Some(LOCAL_AGENT_RUNTIME_LIMITS),
            stack_budget_bytes: Some(AGENT_STACK_BUDGET_BYTES),
        }
    }

    /// The agent profile's ceilings as `ajisai://limits` and a receipt name
    /// them, for a host that wants to show them beside its own.
    pub fn agent_limit_profile() -> serde_json::Value {
        let options = Self::agent(None);
        crate::interpreter::limit_profile::to_json(
            &LOCAL_AGENT_RUNTIME_LIMITS,
            options.step_limit.unwrap_or(LOCAL_AGENT_EXECUTION_STEPS),
        )
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

    /// [`AgentResponse::to_json`], consuming the response so the stack's JSON
    /// is moved into the envelope rather than copied.
    pub fn into_json(self) -> serde_json::Value {
        self.report.into_json()
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

    // The pre-execution check (LANG.CONTRACT.CHECK) runs here as well as in
    // `check`: a program that declares a contract for its own Word is
    // checked against it *before anything runs*, and a declaration inference
    // disproves stops the run — the program is wrong about itself, and
    // executing it would answer a value as if it were not. The result is the
    // same report `check` gives, with `contractDecls` carrying every finding.
    // Programs without a directive are untouched, which is every program
    // that does not opt in.
    let mut interp = options.interpreter();
    let decls = contract_violation::declared_contract_check(source);
    if let Some(check) = &decls {
        if check.violated {
            return AgentResponse::computed(contract_violation::violation_report(
                &interp,
                check,
                Some(source),
            ));
        }
    }
    let result = interp.execute(source).await;
    let trace = interp.drain_error_flow_trace();
    let output = print_payloads(&interp);
    let mut report = completed_run_report(
        &interp,
        result,
        trace,
        output,
        source,
        options.stack_budget_bytes,
    );
    // A verified (or unverifiable) declaration is reported too, so the caller
    // sees the check happened and what it decided; a source with no
    // directive carries no `contractDecls` at all.
    report.contract_decls = decls.as_ref().map(|check| check.to_json());
    AgentResponse::computed(report)
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
    if let Some(check) = contract_decls.as_ref().filter(|result| result.violated) {
        // The same shape every other error has — `message`, `diagnosis`,
        // `aiDiagnostic.category: contractViolation` — so a reader following
        // the documented order ("on error, read diagnosis.why") finds the
        // violation where every other failure is, not only in
        // `contractDecls.findings`.
        return AgentResponse {
            report: contract_violation::violation_report(&interp, check, None),
        };
    }
    let status = "ok";
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
