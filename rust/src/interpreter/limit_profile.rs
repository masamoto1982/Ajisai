//! The one place the resource-ceiling set is enumerated.
//!
//! A run's ceilings are published three ways, by three consumers that had no
//! reason to know about each other:
//!
//! - `wasm_interpreter_bindings::host_profile` — JSON for the playground's
//!   "resource limits" label, read by `src/entry/entry-common.ts`.
//! - `agent::execution_receipt::limit_profile_json` — JSON in the execution
//!   receipt and in the `outcomes` tool's response.
//! - `agent::execution_receipt::write_limit_profile` — the receipt's hashed
//!   byte grammar.
//!
//! Each used to enumerate the eight ceilings itself. They agreed, and nothing
//! made them: the first even claims in its own doc comment to publish "the
//! same names every other Ajisai host publishes them by", with no check
//! anywhere that this is so.
//!
//! The byte-grammar copy is the one that made this worth fixing. A ceiling
//! omitted there does not fail a test — it silently weakens the receipt: two
//! runs under *different* ceilings would digest identically, and a receipt
//! exists precisely to answer "under which limit profile was this judged".
//! The JSON copies degrade loudly by comparison; that one degrades into a
//! quiet false attestation.
//!
//! So `entries` below destructures `RuntimeLimits` **exhaustively**. Adding a
//! ceiling to that struct stops this file compiling until the new ceiling is
//! named and placed, which propagates to all three consumers at once. That is
//! a compile-time guarantee rather than a test, which is the strongest form
//! this particular cross-check can take.

use super::RuntimeLimits;

/// The number of ceilings in a limit profile: `RuntimeLimits`' own fields plus
/// the execution-step budget, which lives beside rather than inside it (see
/// `RuntimeLimits`' doc for why).
pub(crate) const LIMIT_PROFILE_CEILINGS: usize = 8;

/// Every ceiling of one run as `(published name, value)`, in the order the
/// receipt's byte grammar writes them.
///
/// The order is part of `RECEIPT_SCHEMA_TAG`'s grammar: reordering these
/// entries changes every receipt digest, so it is a schema-tag change, not a
/// refactor. The names are the wire names every host publishes.
pub(crate) fn entries(
    limits: &RuntimeLimits,
    step_limit: usize,
) -> [(&'static str, u64); LIMIT_PROFILE_CEILINGS] {
    // Exhaustive on purpose — see this module's doc. Do not replace with
    // field access, and do not add `..`.
    let RuntimeLimits {
        max_materialized_elements,
        max_source_bytes,
        max_numeric_literal_digits,
        max_numeric_work,
        max_collection_work,
        max_bigint_bits,
        max_algebraic_terms,
    } = *limits;
    [
        ("executionSteps", step_limit as u64),
        ("materializedElements", max_materialized_elements as u64),
        ("sourceBytes", max_source_bytes as u64),
        ("numericLiteralDigits", max_numeric_literal_digits as u64),
        ("numericWork", max_numeric_work),
        ("collectionWork", max_collection_work),
        ("bigintBits", max_bigint_bits),
        ("algebraicTerms", max_algebraic_terms as u64),
    ]
}

/// The ceilings as a JSON object, under the wire names of [`entries`].
pub(crate) fn to_json(limits: &RuntimeLimits, step_limit: usize) -> serde_json::Value {
    serde_json::Value::Object(
        entries(limits, step_limit)
            .into_iter()
            .map(|(name, value)| (name.to_string(), serde_json::json!(value)))
            .collect(),
    )
}

/// Append the ceilings to a digest input as big-endian `u64`s, in [`entries`]'
/// order. Length-prefixing is unnecessary here and deliberately absent: the
/// count is fixed by `LIMIT_PROFILE_CEILINGS`, so the field is
/// self-delimiting as long as that constant and this writer move together —
/// which the exhaustive destructure is what enforces.
pub(crate) fn write_digest_bytes(bytes: &mut Vec<u8>, limits: &RuntimeLimits, step_limit: usize) {
    for (_, value) in entries(limits, step_limit) {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
}

#[cfg(test)]
#[path = "limit_profile_tests.rs"]
mod limit_profile_tests;
