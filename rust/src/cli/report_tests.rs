//! Native tests for the CLI `--json` report layer: envelope shape,
//! diagnosis serialization (camelCase protocol strings, `nextChecks`
//! present), missing-capability environment diagnosis, and the static
//! `check` passes. Contract: `docs/dev/agent-cli-output-contract.md`.

use super::report::{self, Report};
use super::{check_structure, missing_capability_diagnosis, resolve_words};
use crate::interpreter::{Interpreter, RuntimeMetrics};
use std::sync::Arc;

fn run_program(code: &str) -> (Interpreter, Result<(), crate::error::AjisaiError>) {
    let mut interp = Interpreter::with_host(Arc::new(super::host::CliHostEnv));
    let result = super::block_on(interp.execute(code));
    (interp, result)
}

#[test]
fn ok_report_envelope_has_contract_fields() {
    let (interp, result) = run_program("[ 1 ] [ 2 ] +");
    assert!(result.is_ok());
    let report = Report {
        status: "ok",
        stack: report::stack_json(&interp),
        stack_display: super::stack_display(&interp),
        output: Vec::new(),
        message: None,
        diagnosis: None,
        ai_diagnostic: None,
        error_flow_trace: Vec::new(),
        runtime_metrics: interp.runtime_metrics(),
        explanation: None,
        plan_check: None,
        receipt: None,
        lang: super::Lang::Ja,
    };
    let doc = report.to_json();
    assert_eq!(doc["schemaVersion"], report::SCHEMA_VERSION);
    assert_eq!(doc["status"], "ok");
    assert!(doc["stack"].is_array());
    assert_eq!(doc["stack"].as_array().unwrap().len(), 1);
    assert_eq!(doc["stackDisplay"][0], "[ 3/1 ]");
    assert!(doc["diagnosis"].is_null());
    assert!(doc["aiDiagnostic"].is_null());
    assert!(doc["runtimeMetrics"]["vtu"].is_object());
    // 18 VTU observation counters plus the aggregate energyProxyScore /
    // proxyVersion / suggestions (docs/quality/energy-proxy-score.md).
    assert_eq!(doc["runtimeMetrics"]["vtu"].as_object().unwrap().len(), 21);
    assert!(doc["runtimeMetrics"]["vtu"]["energyProxyScore"].is_number());
    assert_eq!(doc["runtimeMetrics"]["vtu"]["proxyVersion"], 1);
    assert!(doc["runtimeMetrics"]["vtu"]["suggestions"].is_array());
    // Cost-model observability surface (SPECIFICATION.html §4.8): the scalar
    // fast-path count and the COMPARE-WITHIN budget group. This program uses
    // no comparison, so every comparison counter is zero.
    assert!(doc["runtimeMetrics"]["scalarFastpathCount"].is_number());
    let comparison = doc["runtimeMetrics"]["comparison"]
        .as_object()
        .expect("comparison metrics group present");
    assert_eq!(comparison.len(), 4);
    assert_eq!(doc["runtimeMetrics"]["comparison"]["compareWithinCount"], 0);
    assert_eq!(
        doc["runtimeMetrics"]["comparison"]["compareWithinBudgetTermsConsumed"],
        0
    );
}

#[test]
fn error_report_carries_diagnosis_with_next_checks() {
    let (interp, result) = run_program("[ 1 2 ] FROBNICATE");
    let err = result.expect_err("unknown word must fail");
    let trace = interp.peek_error_flow_trace().to_vec();
    let diagnosis = trace
        .iter()
        .rev()
        .find_map(|event| event.diagnosis.clone())
        .expect("word error must carry a diagnosis");
    let category = crate::error::ErrorCategory::from_error(&err);
    let report = super::error_report(
        &interp,
        &diagnosis,
        Some(&category),
        err.to_string(),
        Vec::new(),
        trace,
        &super::Opts {
            json: true,
            explain: false,
            contract: false,
            receipt: false,
            lang: super::Lang::Ja,
            step_limit: None,
        },
    );
    let doc = report.to_json();
    assert_eq!(doc["status"], "error");
    assert!(
        doc["explanation"].is_null(),
        "explanation must be null without --explain"
    );
    assert_eq!(doc["diagnosis"]["why"], "typoOrUnknownName");
    assert_eq!(doc["diagnosis"]["when"], "resolveWord");
    assert!(
        !doc["diagnosis"]["nextChecks"]
            .as_array()
            .unwrap()
            .is_empty(),
        "diagnosis must include nextChecks"
    );
    assert_eq!(doc["aiDiagnostic"]["kind"], "unknownWord");
    assert_eq!(doc["aiDiagnostic"]["recoverability"], "fixProgram");
    assert!(!doc["errorFlowTrace"].as_array().unwrap().is_empty());
}

#[test]
fn division_by_zero_bubbles_to_nil_with_traced_diagnosis() {
    // SPEC Bubble Rule: x/0 projects to NIL, the run itself succeeds, and
    // the NIL production is observable in the error-flow trace with a full
    // diagnosis (`nextChecks` included).
    let (mut interp, result) = run_program("[ 1 ] [ 0 ] DIV");
    assert!(result.is_ok(), "division by zero must bubble, not fail");
    let trace = interp.drain_error_flow_trace();
    let event = trace.first().expect("nilProduced event must be traced");
    let doc = report::error_flow_event_json(event);
    assert_eq!(doc["kind"], "nilProduced");
    assert_eq!(doc["absence"]["reason"], "divisionByZero");
    assert!(!doc["diagnosis"]["nextChecks"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn print_output_is_collected_not_inline() {
    let (interp, result) = run_program("[ 1 2 3 ] PRINT");
    assert!(result.is_ok());
    let output = super::print_payloads(&interp);
    assert_eq!(output, vec!["[ 1/1 2/1 3/1 ]".to_string()]);
}

#[test]
fn missing_audio_capability_yields_environment_diagnosis() {
    let (interp, result) = run_program("'MUSIC' IMPORT [ [ 60 ] [ 4 ] ] MUSIC@PLAY");
    let err = result.expect_err("audio words must fail without an audio host");
    let message = err.to_string();
    let diagnosis = missing_capability_diagnosis(&interp, &message)
        .expect("missing-capability diagnostic effect must be surfaced");
    assert_eq!(
        diagnosis.why,
        crate::interpreter::debug_diagnosis::CauseClass::Environment
    );
    let doc = report::diagnosis_json(&diagnosis);
    assert_eq!(doc["why"], "environment");
    assert_eq!(doc["where"]["kind"], "hostEnvironment");
    assert!(!doc["nextChecks"].as_array().unwrap().is_empty());
    let ai = diagnosis.ai_payload(None, None, None, None);
    assert_eq!(ai.recoverability, "fixHost");
}

#[test]
fn check_structure_catches_unbalanced_brackets() {
    // The tokenizer already rejects most unbalanced sources; the structural
    // scan is the backstop for token streams that lex but do not nest.
    use crate::types::Token;
    let unclosed = vec![Token::VectorStart, Token::Number("1".into())];
    assert!(check_structure(&unclosed).is_err());
    let stray_close = vec![Token::VectorStart, Token::VectorEnd, Token::VectorEnd];
    assert!(check_structure(&stray_close).is_err());
    let tokens = crate::tokenizer::tokenize("[ [ 1 ] [ 2 ] ]").unwrap();
    assert!(check_structure(&tokens).is_ok());
}

#[test]
fn resolve_accepts_builtins_aliases_defs_and_imports() {
    let interp = Interpreter::new();
    let tokens = crate::tokenizer::tokenize(
        "{ [ 1 ] [ 2 ] + } 'MY-WORD' DEF\nMY-WORD PRINT\n'ALGO' IMPORT [ 3 1 2 ] SORT",
    )
    .unwrap();
    assert_eq!(resolve_words(&interp, &tokens), Vec::<String>::new());
}

#[test]
fn resolve_flags_unknown_words() {
    let interp = Interpreter::new();
    let tokens = crate::tokenizer::tokenize("[ 1 ] [ 2 ] FROBNICATE").unwrap();
    assert_eq!(resolve_words(&interp, &tokens), vec!["FROBNICATE"]);
}

#[test]
fn check_report_uses_default_metrics() {
    let report = Report {
        status: "ok",
        stack: serde_json::Value::Array(Vec::new()),
        stack_display: Vec::new(),
        output: Vec::new(),
        message: None,
        diagnosis: None,
        ai_diagnostic: None,
        error_flow_trace: Vec::new(),
        runtime_metrics: RuntimeMetrics::default(),
        explanation: None,
        plan_check: None,
        receipt: None,
        lang: super::Lang::Ja,
    };
    let doc = report.to_json();
    assert_eq!(doc["runtimeMetrics"]["vtu"]["tensorFlattenCount"], 0);
}
