//! Execution receipt (Phase 4,
//! `docs/dev/auditable-kernel-work-order-2026-09.md`): extends the
//! observation digest (Phase 1,
//! `docs/dev/competitive-advantage-work-order-2026-08.md`) from "what
//! happened" to "this source, on this engine, under these limits, provably
//! produced this outcome" — material a third party can check without taking
//! the host's word for it.
//!
//! The observation digest already solves the hard representation problems
//! (`observation_digest`'s own module doc: algebraic normal forms, Vector/
//! Tensor equivalence, NIL reasons, `stackDisplay` vs value). A receipt
//! is strictly a superset: it bundles that digest alongside everything else
//! a verifier needs — what source, which engine, which vocabulary and
//! outcome-space registry, which resource ceilings, and what the run
//! actually spent — into one more BLAKE3 digest, under its own schema tag
//! (`RECEIPT_SCHEMA_TAG`, distinct from `DIGEST_SCHEMA_TAG` — a receipt is a
//! higher-level concept than the digest it carries, not a replacement for
//! it).

use serde_json::{json, Value as Json};

use crate::interpreter::word_identity::content_digest;
use crate::interpreter::{limit_profile, ResourceUsage, RuntimeLimits};

/// Version tag for the receipt's own byte grammar. Bump it if the grammar
/// changes — a receipt is not a compatible value across a tag change, the
/// same discipline `observation_digest::DIGEST_SCHEMA_TAG` documents.
const RECEIPT_SCHEMA_TAG: &[u8] = b"AJISAI-RECEIPT-2";

/// The exact bytes of the vocabulary and outcome-space registry this binary
/// was built from, embedded at compile time. `spec/words.json` and
/// `spec/outcomes.json` are the canonical sources
/// (`scripts/check-outcome-registry.mjs` keeps the Rust enums they generate
/// in sync with them), so hashing their own bytes — not a projection of
/// them — is what lets a verifier compare against the exact spec/ files a
/// given release shipped.
const WORDS_JSON: &str = include_str!("../../../spec/words.json");
const OUTCOMES_JSON: &str = include_str!("../../../spec/outcomes.json");

/// The engine version every receipt names — identical to `ajisai version`'s
/// own answer, so a verifier can check one against the other without a
/// second lookup path.
pub(crate) fn engine_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// BLAKE3 digest of `spec/words.json` + `spec/outcomes.json`'s literal bytes.
/// Changes exactly when the language's declared vocabulary or outcome space
/// changes, independent of implementation-only edits that leave both files'
/// bytes untouched — the two are the whole "which vocabulary, which outcome
/// space" fact a verifier needs, and this is a digest of exactly those bytes
/// and nothing derived from them.
pub(crate) fn registry_digest() -> String {
    let mut bytes = Vec::with_capacity(WORDS_JSON.len() + OUTCOMES_JSON.len());
    bytes.extend_from_slice(WORDS_JSON.as_bytes());
    bytes.extend_from_slice(OUTCOMES_JSON.as_bytes());
    content_digest(&bytes)
}

fn write_str(bytes: &mut Vec<u8>, s: &str) {
    bytes.extend_from_slice(&(s.len() as u64).to_be_bytes());
    bytes.extend_from_slice(s.as_bytes());
}

/// The limit profile as a JSON object, shared with `outcome_report`'s
/// `outcomes` tool: a static prediction is only meaningful relative to a
/// named profile (`docs/dev/auditable-kernel-work-order-2026-09.md` §5.2
/// pitfall C), and this is the same shape a receipt already reports it in.
/// The ceiling set itself is enumerated once, in
/// `interpreter::limit_profile` — see that module for why.
pub(crate) fn limit_profile_json(limits: &RuntimeLimits, step_limit: usize) -> Json {
    limit_profile::to_json(limits, step_limit)
}

fn write_limit_profile(bytes: &mut Vec<u8>, limits: &RuntimeLimits, step_limit: usize) {
    limit_profile::write_digest_bytes(bytes, limits, step_limit);
}

/// Assemble the execution receipt for one run.
///
/// `observation_digest` is the caller's own already-computed digest for this
/// run (`Report::observation_digest`) — never recomputed here, so the two
/// can never silently disagree about the same observation.
pub(crate) fn build_receipt(
    source: &str,
    limits: &RuntimeLimits,
    step_limit: usize,
    status: &str,
    resource_usage: &ResourceUsage,
    observation_digest: &str,
) -> Json {
    let registry_digest = registry_digest();
    let engine_version = engine_version();

    let mut bytes = Vec::new();
    bytes.extend_from_slice(RECEIPT_SCHEMA_TAG);
    write_str(&mut bytes, source);
    write_str(&mut bytes, engine_version);
    write_str(&mut bytes, &registry_digest);
    write_limit_profile(&mut bytes, limits, step_limit);
    write_str(&mut bytes, status);
    write_str(&mut bytes, observation_digest);
    bytes.extend_from_slice(&resource_usage.execution_steps.to_be_bytes());
    bytes.extend_from_slice(&resource_usage.numeric_work.to_be_bytes());
    bytes.extend_from_slice(&resource_usage.collection_work.to_be_bytes());
    let digest = content_digest(&bytes);

    json!({
        "sourceDigest": content_digest(source.as_bytes()),
        "engineVersion": engine_version,
        "registryDigest": registry_digest,
        "limitProfile": limit_profile_json(limits, step_limit),
        "outcomeStatus": status,
        "observationDigest": observation_digest,
        "resourceUsage": {
            "executionSteps": resource_usage.execution_steps,
            "numericWork": resource_usage.numeric_work,
            "collectionWork": resource_usage.collection_work,
        },
        "digest": digest,
    })
}

#[cfg(test)]
mod tests {
    //! Tests for `crate::agent::execution_receipt` and the `Report::receipt`
    //! field it feeds (docs/dev/auditable-kernel-work-order-2026-09.md Phase 4,
    //! §4.4's acceptance criteria).

    use crate::agent::api::{compute, ComputeOptions};
    use crate::agent::block_on;
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
            "[ 1 2 3 4 5 ] 0 [ ADD ] FOLD",
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

    /// `check`/`infer-contracts` never execute, so they have nothing to
    /// receipt — `Report::receipt`'s own doc comment states this; pinned here so
    /// a future change does not silently start attaching one.
    #[test]
    fn check_never_carries_a_receipt() {
        let response = crate::agent::api::check("1 2 ADD", false).to_json();
        assert_eq!(response["status"], "ok");
        assert!(response["receipt"].is_null(), "response: {response}");
    }
}
