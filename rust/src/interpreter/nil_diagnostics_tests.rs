//! Tests for `nil_diagnostics`: the diagnostic absence accessors
//! (LANG.VALUES.NIL / LANG.OBSERVATION.DIAGNOSIS) `NIL?` and `NIL-REASON`
//! here, and, as nested modules, the produced-NIL trace bookkeeping and the
//! upstream-NIL link a downstream failure gets.
//!
//! Coverage follows the §15 discipline: success paths, the non-NIL path, the
//! reason-present vs reason-absent split, protocol-string (not Rust `Debug`)
//! output, the U firewall, operand consumption, and MC/DC over the two governing
//! decisions (`is_operational_nil` and reason `Some`/`None`).

use crate::error::NilReason;
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::Interpreter;
use crate::test_support::run;

/// The top-of-stack value, as a protocol string, or `None` when it is NIL.
fn top_text(interp: &Interpreter) -> Option<String> {
    let top = interp.get_stack().last().expect("stack must be non-empty");
    if top.is_nil() {
        return None;
    }
    Some(value_as_string(top).expect("top must be a Text value"))
}

fn top_is_nil(interp: &Interpreter) -> bool {
    interp
        .get_stack()
        .last()
        .map(|v| v.is_nil())
        .unwrap_or(false)
}

fn top_is_true(interp: &Interpreter) -> bool {
    interp
        .get_stack()
        .last()
        .and_then(|v| v.as_truth())
        .unwrap_or(false)
}
// ── NIL? ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn nil_check_is_true_for_operational_nil_and_consumes_it() {
    let interp = run("1 0 DIV NIL?").await;
    let stack = interp.get_stack();
    assert_eq!(stack.len(), 1, "NIL? consumes the inspected value");
    assert!(top_is_true(&interp), "NIL? on an operational NIL is TRUE");
}

#[tokio::test]
async fn nil_check_is_false_for_present_value() {
    let interp = run("5 NIL?").await;
    let stack = interp.get_stack();
    assert_eq!(stack.len(), 1, "NIL? consumes the inspected value");
    assert_eq!(
        stack[0].as_truth(),
        Some(false),
        "NIL? on a present value is FALSE"
    );
}

/// The logical Unknown (U) is a NIL read in truth position
/// (LANG.VALUES.TRUTH), so it is an absence and `NIL?` answers TRUE — the
/// same answer a chosen fallback acts on, and the same story the host protocol
/// tells about it (`type: "nil"`, a published `absence.reason`).
///
/// `TRUE NIL AND` is the strong-Kleene UNKNOWN row (neither operand absorbs
/// the other), so it produces a genuine U — `AND`/`NOT` are what makes
/// U reachable from source at all.
#[tokio::test]
async fn nil_check_is_true_for_logical_unknown() {
    let interp = run("TRUE NIL AND NIL?").await;
    assert_eq!(
        interp.get_stack()[0].as_truth(),
        Some(true),
        "NIL? on the logical Unknown must be TRUE: U is an absence"
    );
}

/// A reason survives being read in truth position. `AND` used to swallow it:
/// `1 0 DIV TRUE AND NIL-REASON` answered that it had no reason while the protocol
/// still published `absence.reason = divisionByZero` for that value, so the
/// language contradicted its own boundary and LANG.VALUES.NIL ("the reason
/// is the entire observable content of a NIL").
#[tokio::test]
async fn nil_reason_survives_a_kleene_word() {
    let interp = run("1 0 DIV TRUE AND NIL-REASON").await;
    assert_eq!(
        top_text(&interp).as_deref(),
        Some("divisionByZero"),
        "an UNKNOWN must keep the reason it arrived with"
    );
}

// ── NIL-REASON ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn nil_reason_reports_division_by_zero_protocol_string() {
    let interp = run("1 0 DIV NIL-REASON").await;
    let stack = interp.get_stack();
    assert_eq!(stack.len(), 1, "NIL-REASON consumes the inspected value");
    assert_eq!(
        top_text(&interp).as_deref(),
        Some("divisionByZero"),
        "NIL-REASON must be the lowerCamelCase protocol string, not a Debug name"
    );
}

/// The output must be the protocol string, never the Rust `Debug` rendering of
/// the `NilReason` enum (`DivisionByZero`).
#[tokio::test]
async fn nil_reason_is_protocol_string_not_debug_name() {
    let interp = run("1 0 DIV NIL-REASON").await;
    let text = top_text(&interp).expect("reason must be Text");
    assert_eq!(text, "divisionByZero");
    assert_ne!(text, format!("{:?}", NilReason::DivisionByZero));
}

#[tokio::test]
async fn nil_reason_reports_index_out_of_bounds() {
    let interp = run("[ 1 2 3 ] 9 GET NIL-REASON").await;
    assert_eq!(top_text(&interp).as_deref(), Some("indexOutOfBounds"));
}

/// A written NIL reads back as `literal`. Every NIL carries a reason now, so
/// the reason-`None` branch of `NIL-REASON` is no longer reachable through a
/// literal; the branch itself is covered by `nil_reason_is_nil_for_present_value`
/// below, where the subject is not an operational NIL at all.
#[tokio::test]
async fn nil_reason_of_a_written_nil_is_literal() {
    let interp = run("NIL NIL-REASON").await;
    assert_eq!(top_text(&interp).as_deref(), Some("literal"));
}

/// The non-NIL path: `NIL-REASON` on a present value yields NIL, not an error.
#[tokio::test]
async fn nil_reason_is_nil_for_present_value() {
    let interp = run("5 NIL-REASON").await;
    assert!(top_is_nil(&interp));
    assert_eq!(interp.get_stack().len(), 1, "the 5 is consumed");
}

/// `NIL-REASON` on the result of an exact-arithmetic comparison must yield
/// NIL, never a reason — same as `nil_reason_is_nil_for_present_value`
/// above. Exact comparisons always decide (LANG.VALUES.EXACT), so `2 SQRT 1
/// ADD` compared against itself resolves to a definite `TRUE`.
#[tokio::test]
async fn nil_reason_is_nil_for_a_decidable_exact_comparison() {
    let interp = run("2 SQRT 1 ADD 2 SQRT 1 ADD SUB 0 EQ NIL-REASON").await;
    assert!(
        top_is_nil(&interp),
        "NIL-REASON on a non-NIL value must be NIL, never a reason string"
    );
}
// ── Domain miss (LANG.FAILURE.PROJECT: "SQRT of a negative rational is a well-formed
//    domain miss") ────────────────────────────────────────────────────────────
/// Division by zero keeps its own reason. The domain-miss variant is a new
/// classification, not a rename of an existing one.
#[tokio::test]
async fn division_by_zero_is_untouched_by_the_domain_miss_split() {
    let interp = run("1 0 DIV NIL-REASON").await;
    assert_eq!(top_text(&interp).as_deref(), Some("divisionByZero"));
}

#[cfg(test)]
mod upstream_nil_link_tests {
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
    /// 1,000,000, so `0 100001 RANGE` projects here and simply succeeds under
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
    /// level only as "got NIL" from the Word that refused it. A NIL flows through
    /// every `data` operand, so the Word that refuses it is one whose operand is
    /// a block — here `EXEC`.
    #[tokio::test]
    async fn a_space_ceiling_reaches_the_top_level_diagnosis() {
        let report = report("0 100001 RANGE EXEC").await;
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
        let report = report("1 0 DIV EXEC").await;
        let detail = report["diagnosis"]["nextChecks"][0]["detail"]["en"]
            .as_str()
            .expect("english detail");
        assert!(
            detail.contains("DIV") && detail.contains("divisionByZero"),
            "{detail}"
        );
    }

    /// The link names the Word that produced the NIL, not the last Word it passed
    /// through on the way: with `ADD` between the projection and the refusal, the
    /// cause is still `DIV`.
    #[tokio::test]
    async fn the_link_names_the_producer_not_the_last_word_the_nil_passed() {
        let report = report("1 0 DIV 2 ADD EXEC").await;
        assert_eq!(report["status"], "error");
        let evidence = evidence(&report);
        assert!(
            evidence.contains(&"upstreamNilProducer=DIV".to_string()),
            "{evidence:?}"
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
            !evidence(&report)
                .iter()
                .any(|e| e.starts_with("upstreamNil")),
            "no upstream evidence either"
        );
    }

    /// A NIL the program wrote down is not a fault. `NIL EXEC` is a type error
    /// whose cause is the source line in front of the reader, so sending them
    /// "upstream" would be sending them nowhere.
    #[tokio::test]
    async fn a_written_nil_is_not_reported_as_an_upstream_cause() {
        let report = report("NIL EXEC").await;
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
        let report = report("1 0 DIV 'S' BIND 42 S S NIL? SELECT").await;
        assert_eq!(report["status"], "ok");
        assert!(report["diagnosis"].is_null());
    }
}

/// What the per-Word bookkeeping in `nil_trace` must answer, and must answer
/// without unpacking the data it is asked about.
///
/// One question runs after *every* Word: "did this Word produce a reasoned
/// absence?" (the error-flow trace). Two things decide it. *Whether* the Word
/// produced one is the mint count (`crate::semantic::minted_absence_count`):
/// a NIL that arrived in an operand and left in the result was carried, not
/// produced (LANG.FAILURE.PASSTHROUGH). *Which* reason it produced is read off
/// the result — and for a dense tensor, off its absence map rather than by
/// rebuilding every lane as a boxed `Value`, which on a 4096-lane tensor was
/// 67% of all instructions executed and scaled with the data rather than the
/// failures. These are ratio-free, wall-clock-free gates on the parts that
/// could silently change.
#[cfg(test)]
mod nil_trace_tests {
    use crate::error::NilReason;
    use crate::interpreter::error_flow_trace::ErrorFlowEventKind;
    use crate::interpreter::nil_diagnostics::projected_nil_reason;
    use crate::interpreter::Interpreter;
    use crate::semantic::Recoverability;
    use crate::types::{Value, ValueData};

    /// Every reason the trace recorded for a `NilProduced` event raised by
    /// `word`, in the order the run recorded them.
    async fn traced_reasons(source: &str, word: &str) -> Vec<Option<NilReason>> {
        let mut interp = Interpreter::new();
        interp
            .execute(source)
            .await
            .unwrap_or_else(|e| panic!("`{source}` must compute, got: {e:?}"));
        interp
            .drain_error_flow_trace()
            .iter()
            .filter(|event| {
                event.kind == ErrorFlowEventKind::NilProduced && event.word.as_deref() == Some(word)
            })
            .map(|event| event.absence.as_ref().and_then(|a| a.reason))
            .collect()
    }

    async fn top_is_dense_tensor(source: &str) -> bool {
        let mut interp = Interpreter::new();
        interp.execute(source).await.expect("must compute");
        matches!(
            interp.get_stack().as_slice().last().map(|v| &v.data),
            Some(ValueData::Tensor { .. })
        )
    }

    fn number(n: i64) -> Value {
        Value::from_number(crate::types::fraction::Fraction::new(n.into(), 1.into()))
    }

    /// `MAP`ping a failing block over a numeric vector lands a *dense tensor*
    /// whose lanes are absent for a reason. The gate is the representation as
    /// much as the reason: if this stops being a `Tensor`, the reads below stop
    /// covering the dense absence map and silently pass on the `Vector` walk
    /// instead.
    #[tokio::test]
    async fn a_lifted_failure_lands_in_a_dense_tensor() {
        assert!(
            top_is_dense_tensor("[ 1 2 3 4 5 6 7 8 ] [ 9 0 DIV ] MAP").await,
            "MAP over a numeric vector must produce a dense Tensor for the \
             dense-absence gates below to mean anything"
        );
    }

    /// The reason survives the read: a reasoned absent lane densified into a
    /// tensor is read back from the absence map, the same answer the
    /// materialized lane walk gave.
    #[test]
    fn a_dense_tensors_absence_reason_is_read_from_its_map() {
        let lanes = vec![
            number(1),
            Value::nil_with_reason(NilReason::DivisionByZero, Recoverability::Recoverable),
            number(3),
        ];
        let dense = Value::from_vector_promoted(lanes.clone());
        assert!(
            matches!(dense.data, ValueData::Tensor { .. }),
            "numeric lanes with an absent one must densify"
        );
        assert_eq!(
            projected_nil_reason(&dense),
            Some(NilReason::DivisionByZero)
        );
        // Same failure, same reason, whichever representation carries it.
        let boxed = Value::from_vector(lanes);
        assert_eq!(projected_nil_reason(&boxed), projected_nil_reason(&dense));
    }

    /// The Word that produced a lane's absence is the one that ran the
    /// projection, not the Word whose result carries it: `MAP` lands a dense
    /// tensor whose lanes `DIV` made absent, so `DIV` is the producer — once per
    /// lane it answered — and `MAP` is the frame it happened in. `MAP` used to
    /// record the tensor as its own production, and a reader was sent to the
    /// wrong Word.
    #[tokio::test]
    async fn the_word_that_projected_the_lane_is_the_producer() {
        let source = "[ 1 2 3 4 5 6 7 8 ] [ 4 -1 MUL SQRT ] MAP";
        assert!(top_is_dense_tensor(source).await);
        assert_eq!(traced_reasons(source, "MAP").await, Vec::new());
        assert_eq!(
            traced_reasons(source, "SQRT").await,
            vec![Some(NilReason::DomainMiss); 8]
        );
    }

    /// A Word lifted over its operand projects per lane and is the producer
    /// itself: the reason is read off the result it built.
    #[tokio::test]
    async fn a_lifted_word_that_projects_is_the_producer() {
        assert_eq!(
            traced_reasons("[ 1 4 -1 9 ] SQRT", "SQRT").await,
            vec![Some(NilReason::DomainMiss)]
        );
    }

    /// `Literal` is the absence a Word *received*, not one it produced, so it
    /// is not an event — and a `NIL` written in source densifies into a tensor
    /// lane carrying exactly that reason. The dense read has to skip it for the
    /// same reason the lane walk did.
    #[tokio::test]
    async fn a_dense_literal_absence_is_not_traced_as_produced() {
        let mut interp = Interpreter::new();
        interp
            .execute("[ 1 NIL 3 4 5 6 7 8 ] 1 ADD")
            .await
            .expect("must compute");
        let produced: Vec<_> = interp
            .drain_error_flow_trace()
            .iter()
            .filter(|event| event.kind == ErrorFlowEventKind::NilProduced)
            .map(|event| event.word.clone())
            .collect();
        assert!(
            produced.is_empty(),
            "a literal NIL is propagated, not produced: {produced:?}"
        );
    }

    /// A reasoned absent lane carried through a lifted Word is not produced
    /// again by it: `ADD` over the tensor `MAP` left passes every absent lane
    /// through (LANG.FAILURE.PASSTHROUGH per lane) and mints nothing.
    #[tokio::test]
    async fn a_dense_reasoned_absence_passed_through_is_not_traced_again() {
        assert_eq!(
            traced_reasons("[ 1 2 3 4 5 6 7 8 ] [ 9 0 DIV ] MAP 1 ADD", "ADD").await,
            Vec::new()
        );
    }
}
