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
//! Tensor equivalence, `hint` vs meaning, `stackDisplay` vs value). A receipt
//! is strictly a superset: it bundles that digest alongside everything else
//! a verifier needs — what source, which engine, which vocabulary and
//! outcome-space registry, which resource ceilings, and what the run
//! actually spent — into one more BLAKE3 digest, under its own schema tag
//! (`RECEIPT_SCHEMA_TAG`, distinct from `DIGEST_SCHEMA_TAG` — a receipt is a
//! higher-level concept than the digest it carries, not a replacement for
//! it).

use serde_json::{json, Value as Json};

use crate::interpreter::word_identity::content_digest;
use crate::interpreter::{ResourceUsage, RuntimeLimits};

/// Version tag for the receipt's own byte grammar. Bump it if the grammar
/// changes — a receipt is not a compatible value across a tag change, the
/// same discipline `observation_digest::DIGEST_SCHEMA_TAG` documents.
const RECEIPT_SCHEMA_TAG: &[u8] = b"AJISAI-RECEIPT-1";

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

fn limit_profile_json(limits: &RuntimeLimits, step_limit: usize) -> Json {
    json!({
        "executionSteps": step_limit,
        "materializedElements": limits.max_materialized_elements,
        "sourceBytes": limits.max_source_bytes,
        "numericLiteralDigits": limits.max_numeric_literal_digits,
        "numericWork": limits.max_numeric_work,
        "collectionWork": limits.max_collection_work,
        "bigintBits": limits.max_bigint_bits,
        "algebraicTerms": limits.max_algebraic_terms,
    })
}

fn write_limit_profile(bytes: &mut Vec<u8>, limits: &RuntimeLimits, step_limit: usize) {
    bytes.extend_from_slice(&(step_limit as u64).to_be_bytes());
    bytes.extend_from_slice(&(limits.max_materialized_elements as u64).to_be_bytes());
    bytes.extend_from_slice(&(limits.max_source_bytes as u64).to_be_bytes());
    bytes.extend_from_slice(&(limits.max_numeric_literal_digits as u64).to_be_bytes());
    bytes.extend_from_slice(&limits.max_numeric_work.to_be_bytes());
    bytes.extend_from_slice(&limits.max_collection_work.to_be_bytes());
    bytes.extend_from_slice(&limits.max_bigint_bits.to_be_bytes());
    bytes.extend_from_slice(&(limits.max_algebraic_terms as u64).to_be_bytes());
}

/// Assemble the execution receipt for one run, or `None` when the
/// observation itself could not be digested — a Tier 2 `ExactReal::Computable`
/// scalar was present somewhere in the stack, the same condition
/// `observation_digest` refuses to fabricate a value for (pitfall C: a
/// receipt built over an approximated observation would certify the wrong
/// thing, which is worse than certifying nothing).
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
    observation_digest: Option<&str>,
) -> Option<Json> {
    let observation_digest = observation_digest?;
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

    Some(json!({
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
    }))
}
