//! Test suite for `crate::interpreter::upstream_nil_link`.
//!
//! Driven end-to-end through the agent API rather than by hand-built traces:
//! what is being pinned is that the *top-level* diagnosis a consumer reads
//! names the cause, and only a real run produces the trace that has to carry
//! it.

use crate::agent::api::{compute, ComputeOptions, LOCAL_AGENT_RUNTIME_LIMITS};
use crate::interpreter::upstream_nil_link::UPSTREAM_NIL_CHECK;

/// The agent host profile, which is the one this link exists for: its
/// materialization ceiling is 100,000, where the interpreter default is
/// 1,000,000, so `[ 0 100001 ] RANGE` projects here and simply succeeds under
/// the default. Running these against the default profile would have made the
/// resource case pass by never failing at all.
async fn report(source: &str) -> serde_json::Value {
    compute(
        source,
        ComputeOptions {
            runtime_limits: Some(LOCAL_AGENT_RUNTIME_LIMITS),
            ..ComputeOptions::default()
        },
    )
    .await
    .to_json()
}

fn check_codes(report: &serde_json::Value) -> Vec<String> {
    report["diagnosis"]["nextChecks"]
        .as_array()
        .map(|checks| {
            checks
                .iter()
                .filter_map(|c| c["code"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn evidence(report: &serde_json::Value) -> Vec<String> {
    report["diagnosis"]["evidence"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|e| e.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The reported case: a resource ceiling two Words upstream reached the top
/// level only as "LENGTH got a Nil".
#[tokio::test]
async fn a_space_ceiling_reaches_the_top_level_diagnosis() {
    let report = report("[ 0 100001 ] RANGE LENGTH").await;
    assert_eq!(report["status"], "error");

    let codes = check_codes(&report);
    assert_eq!(
        codes.first().map(String::as_str),
        Some(UPSTREAM_NIL_CHECK),
        "the cause belongs before the checks about the reporting Word: {codes:?}"
    );

    let detail = report["diagnosis"]["nextChecks"][0]["detail"]["en"]
        .as_str()
        .expect("english detail");
    assert!(
        detail.contains("RANGE") && detail.contains("spaceExhausted"),
        "the check must name the producer and the reason: {detail}"
    );

    let evidence = evidence(&report);
    assert!(
        evidence.contains(&"upstreamNilProducer=RANGE".to_string())
            && evidence.contains(&"upstreamNilReason=spaceExhausted".to_string()),
        "a machine matches on evidence, so the cause belongs there too: {evidence:?}"
    );
}

/// The same link for an absence with a different origin, so the behaviour is
/// the NIL-flow rule and not a special case for the resource ceiling.
#[tokio::test]
async fn a_division_by_zero_reaches_the_top_level_diagnosis_too() {
    let report = report("1 0 / LENGTH").await;
    let detail = report["diagnosis"]["nextChecks"][0]["detail"]["en"]
        .as_str()
        .expect("english detail");
    assert!(
        detail.contains("DIV") && detail.contains("divisionByZero"),
        "{detail}"
    );
}

/// The negative half, and the one that decides whether the link is worth
/// having: a plain type error keeps exactly the diagnosis it had. A link that
/// attached itself to every failure would be a plausible wrong cause on the
/// failures that already state their own.
#[tokio::test]
async fn a_genuine_type_error_is_left_alone() {
    let report = report("5 LENGTH").await;
    assert_eq!(report["status"], "error");
    assert!(
        !check_codes(&report).contains(&UPSTREAM_NIL_CHECK.to_string()),
        "no NIL was produced, so there is no upstream cause to name"
    );
    assert!(
        !evidence(&report).iter().any(|e| e.starts_with("upstreamNil")),
        "no upstream evidence either"
    );
}

/// A NIL the program wrote down is not a fault. `NIL LENGTH` is a type error
/// whose cause is the source line in front of the reader, so sending them
/// "upstream" would be sending them nowhere.
#[tokio::test]
async fn a_written_nil_is_not_reported_as_an_upstream_cause() {
    let report = report("NIL LENGTH").await;
    assert_eq!(report["status"], "error");
    assert!(
        !check_codes(&report).contains(&UPSTREAM_NIL_CHECK.to_string()),
        "a literal NIL is not an upstream failure"
    );
}

/// A recovered absence never reaches a failing Word, so nothing is linked and
/// the run simply succeeds — the check does not perturb the NIL-flow path it
/// reads from.
#[tokio::test]
async fn a_recovered_absence_leaves_no_trace_of_the_link() {
    let report = report("1 0 / OR-NIL 42").await;
    assert_eq!(report["status"], "ok");
    assert!(report["diagnosis"].is_null());
}
