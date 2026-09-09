//! Tests for `crate::agent::execution_receipt` and the `Report::receipt`
//! field it feeds (docs/dev/auditable-kernel-work-order-2026-09.md Phase 4,
//! §4.4's acceptance criteria).

use super::api::{compute, ComputeOptions};
use super::block_on;
use crate::interpreter::RuntimeLimits;

#[test]
fn same_source_and_profile_receipts_identically() {
    let a = block_on(compute("1 2 ADD", ComputeOptions::default())).to_json();
    let b = block_on(compute("1 2 ADD", ComputeOptions::default())).to_json();
    assert_eq!(a["receipt"], b["receipt"]);
    assert!(a["receipt"]["digest"].is_string(), "receipt: {a}");
}

#[test]
fn a_one_character_source_change_changes_the_receipt() {
    let a = block_on(compute("1 2 ADD", ComputeOptions::default())).to_json();
    let b = block_on(compute("1 3 ADD", ComputeOptions::default())).to_json();
    assert_ne!(a["receipt"]["digest"], b["receipt"]["digest"]);
    assert_ne!(a["receipt"]["sourceDigest"], b["receipt"]["sourceDigest"]);
    // Everything else about the run is identical — isolates the source as
    // the one changed input.
    assert_eq!(a["receipt"]["engineVersion"], b["receipt"]["engineVersion"]);
    assert_eq!(
        a["receipt"]["registryDigest"],
        b["receipt"]["registryDigest"]
    );
    assert_eq!(a["receipt"]["limitProfile"], b["receipt"]["limitProfile"]);
}

#[test]
fn changing_only_the_limit_profile_changes_the_receipt() {
    let default_profile = block_on(compute("1 2 ADD", ComputeOptions::default())).to_json();
    let narrowed_profile = block_on(compute(
        "1 2 ADD",
        ComputeOptions {
            runtime_limits: Some(RuntimeLimits {
                max_materialized_elements: 10,
                ..RuntimeLimits::default()
            }),
            ..ComputeOptions::default()
        },
    ))
    .to_json();
    assert_eq!(
        default_profile["receipt"]["sourceDigest"],
        narrowed_profile["receipt"]["sourceDigest"]
    );
    assert_ne!(
        default_profile["receipt"]["limitProfile"],
        narrowed_profile["receipt"]["limitProfile"]
    );
    assert_ne!(
        default_profile["receipt"]["digest"],
        narrowed_profile["receipt"]["digest"]
    );
}

/// Pitfall B: `resourceUsage` must be exactly reproducible across repeated
/// runs of the same source under the same profile before it can enter a
/// receipt at all — if fastpath/SIMD path selection ever made it vary, no
/// receipt could be reproducible either. Checked directly (not only through
/// the receipt digest) so a future regression here is diagnosed as a
/// resource-meter bug, not chased as a receipt bug.
#[test]
fn resource_usage_is_reproducible_across_repeated_runs() {
    for source in [
        "1 2 ADD",
        "[ 1 2 3 4 5 ] SUM",
        "[ 1 2 3 ] [ 4 5 6 ] ADD",
        "2 SQRT 2 SQRT ADD",
        "1000000000000000000000 7 MUL",
    ] {
        let first = block_on(compute(source, ComputeOptions::default())).to_json();
        for _ in 0..4 {
            let again = block_on(compute(source, ComputeOptions::default())).to_json();
            assert_eq!(
                first["resourceUsage"], again["resourceUsage"],
                "{source:?} charged different resourceUsage across repeated runs"
            );
        }
    }
}

/// A run that raises a language ERROR is just as receiptable as one that
/// succeeds — the outcome status is part of what the receipt certifies, not
/// a precondition for having one.
#[test]
fn an_error_outcome_still_gets_a_receipt() {
    let response = block_on(compute("FROBNICATE", ComputeOptions::default())).to_json();
    assert_eq!(response["status"], "error");
    assert!(response["receipt"].is_object(), "receipt: {response}");
    assert_eq!(response["receipt"]["outcomeStatus"], "error");
}

/// Pitfall C: a Tier 2 (`PI`) result already forces `observationDigest` to
/// `null` (`observation_digest`'s own module doc); the receipt must inherit
/// that refusal rather than certify an observation it never actually hashed.
#[test]
fn a_tier_2_result_has_no_receipt() {
    let response = block_on(compute("PI", ComputeOptions::default())).to_json();
    assert_eq!(response["status"], "ok");
    assert!(
        response["observationDigest"].is_null(),
        "response: {response}"
    );
    assert!(response["receipt"].is_null(), "response: {response}");
}

/// `check`/`infer-contracts` never execute, so they have nothing to
/// receipt — `Report::receipt`'s own doc comment states this; pinned here so
/// a future change does not silently start attaching one.
#[test]
fn check_never_carries_a_receipt() {
    let response = super::api::check("1 2 ADD", false).to_json();
    assert_eq!(response["status"], "ok");
    assert!(response["receipt"].is_null(), "response: {response}");
}
