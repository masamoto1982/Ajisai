use super::{entries, to_json, write_digest_bytes, LIMIT_PROFILE_CEILINGS};
use crate::interpreter::RuntimeLimits;

fn sample() -> (RuntimeLimits, usize) {
    (
        RuntimeLimits {
            max_materialized_elements: 2,
            max_source_bytes: 3,
            max_numeric_literal_digits: 4,
            max_numeric_work: 5,
            max_collection_work: 6,
            max_bigint_bits: 7,
            max_algebraic_terms: 8,
        },
        1,
    )
}

/// The three published forms are one list now, so this only has to pin the
/// list. `entries` is exhaustive over `RuntimeLimits`, so a new ceiling
/// cannot reach any consumer without passing through here.
#[test]
fn every_ceiling_is_named_exactly_once() {
    let (limits, steps) = sample();
    let names: Vec<&str> = entries(&limits, steps).iter().map(|(n, _)| *n).collect();
    assert_eq!(names.len(), LIMIT_PROFILE_CEILINGS);
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "duplicate name in {names:?}");
}

#[test]
fn the_json_form_carries_exactly_the_entries() {
    let (limits, steps) = sample();
    let json = to_json(&limits, steps);
    let object = json.as_object().expect("limit profile is an object");
    assert_eq!(object.len(), LIMIT_PROFILE_CEILINGS);
    for (name, value) in entries(&limits, steps) {
        assert_eq!(object[name], serde_json::json!(value), "{name}");
    }
}

/// The receipt's byte grammar is a published format: these exact bytes are
/// what every receipt digest is computed over. A change here is a
/// `RECEIPT_SCHEMA_TAG` change, and this test is what makes that deliberate
/// rather than accidental — including the field *order*, which a set-based
/// assertion would not catch.
#[test]
fn the_digest_bytes_are_the_entry_values_in_order() {
    let (limits, steps) = sample();
    let mut bytes = Vec::new();
    write_digest_bytes(&mut bytes, &limits, steps);
    assert_eq!(bytes.len(), LIMIT_PROFILE_CEILINGS * 8);

    let mut expected = Vec::new();
    for value in [1u64, 2, 3, 4, 5, 6, 7, 8] {
        expected.extend_from_slice(&value.to_be_bytes());
    }
    assert_eq!(
        bytes, expected,
        "receipt byte grammar changed; bump RECEIPT_SCHEMA_TAG if intended"
    );
}

/// Two runs that differ only in one ceiling must not digest alike — the
/// property the byte grammar exists for, and the one a forgotten field
/// silently breaks.
#[test]
fn changing_any_single_ceiling_changes_the_bytes() {
    let (base, steps) = sample();
    let mut baseline = Vec::new();
    write_digest_bytes(&mut baseline, &base, steps);

    let mut variants = vec![(base, steps + 1000)];
    for mutate in [
        |mut l: RuntimeLimits| {
            l.max_materialized_elements += 1000;
            l
        },
        |mut l: RuntimeLimits| {
            l.max_source_bytes += 1000;
            l
        },
        |mut l: RuntimeLimits| {
            l.max_numeric_literal_digits += 1000;
            l
        },
        |mut l: RuntimeLimits| {
            l.max_numeric_work += 1000;
            l
        },
        |mut l: RuntimeLimits| {
            l.max_collection_work += 1000;
            l
        },
        |mut l: RuntimeLimits| {
            l.max_bigint_bits += 1000;
            l
        },
        |mut l: RuntimeLimits| {
            l.max_algebraic_terms += 1000;
            l
        },
    ] {
        variants.push((mutate(base), steps));
    }
    assert_eq!(variants.len(), LIMIT_PROFILE_CEILINGS);

    for (limits, step_limit) in variants {
        let mut bytes = Vec::new();
        write_digest_bytes(&mut bytes, &limits, step_limit);
        assert_ne!(
            bytes, baseline,
            "a ceiling change left the receipt bytes identical: {limits:?} steps={step_limit}"
        );
    }
}
