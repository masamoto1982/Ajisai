//! Does an absent lane's reason survive every boundary it crosses?
//!
//! LANG.VALUES.NIL makes the reason the entire observable content of an
//! absence, so a boundary that carries a NIL without its reason carries a
//! different value. `Value` records one on the value itself; a dense tensor
//! records one per lane, beside the lane. This file is the second half's
//! coverage, and it is one file rather than three because the property is one
//! property: promotion into dense storage, materialization back out, the
//! persistence codec, and the value arena each used to drop it, and each is a
//! place a future change could drop it again.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::arena::{arena_to_value, value_to_arena};
use super::fraction::Fraction;
use super::value_persist::{decode_stack, encode_stack};
use super::{DenseTensor, Interpretation, Value, ValueData};
use crate::error::NilReason;
use crate::semantic::Recoverability;

fn div_by_zero() -> Value {
    Value::nil_with_reason(NilReason::DivisionByZero, Recoverability::Recoverable)
}

/// **A vector holding a NIL is stored densely, and the NIL keeps its
/// reason.**
///
/// `try_dense_value` used to refuse a NIL child, so this vector kept its
/// nested form. The refusal was right while dense storage could say only
/// *that* a lane was absent: densifying `NIL(divisionByZero)` and reading
/// it back gave a NIL claiming the program had written it. With the reason
/// stored beside the lane the refusal is no longer needed.
#[test]
fn a_densified_nil_keeps_its_reason() {
    let value = Value::from_vector_promoted(vec![Value::from_int(1), div_by_zero()]);
    assert!(
        matches!(value.data, ValueData::Tensor { .. }),
        "a vector of numeric lanes promotes, NIL included: {:?}",
        value.data
    );
    let lane = value.child(1).expect("lane 1 is a child");
    assert!(lane.is_nil());
    assert_eq!(lane.nil_reason().copied(), Some(NilReason::DivisionByZero));
}

/// A lane made absent by a `Fraction` alone is absent for a reason the
/// tensor was never told, and says so — rather than claiming `literal`,
/// which asserts the program wrote it.
#[test]
fn a_lane_absent_without_a_recorded_reason_reads_back_reasonless() {
    let tensor = DenseTensor::from_fractions(vec![Fraction::from(1), Fraction::nil()], vec![2])
        .expect("small fractions admit dense representation");
    assert_eq!(tensor.absence_at(1).and_then(|a| a.reason), None);
    let lane = Value::from_dense_lane(&tensor, 1);
    assert!(lane.is_nil());
    assert_eq!(lane.nil_reason(), None);
}

/// A present lane never reports a reason, whatever the map was built with
/// — the denominator sentinel stays the only authority on presence.
#[test]
fn a_reason_recorded_for_a_present_lane_is_dropped() {
    let absences = BTreeMap::from([(
        0,
        div_by_zero()
            .absence_metadata()
            .cloned()
            .expect("a reasoned NIL carries metadata"),
    )]);
    let tensor = DenseTensor::from_fractions_with_absences(
        vec![Fraction::from(1), Fraction::from(2)],
        vec![2],
        absences,
    )
    .expect("small fractions admit dense representation");
    assert!(tensor.is_valid(0));
    assert!(tensor.absence_at(0).is_none());
    assert_eq!(tensor.absences().count(), 0);
}

/// **A sub-tensor's absences are re-indexed with its lanes.** Taking row 0
/// of a 2x2 must carry lane 1's reason to the child's lane 1, not leave it
/// keyed by the parent's lane index — where the child would never find it.
#[test]
fn a_sub_tensor_rebases_the_absences_it_inherits() {
    let value = Value::from_vector_promoted(vec![
        Value::from_vector_promoted(vec![Value::from_int(1), div_by_zero()]),
        Value::from_vector_promoted(vec![Value::from_int(3), Value::from_int(4)]),
    ]);
    assert!(matches!(value.data, ValueData::Tensor { .. }));
    let row = value.child(0).expect("row 0 is a child");
    let lane = row.child(1).expect("lane 1 of row 0 is a child");
    assert_eq!(lane.nil_reason().copied(), Some(NilReason::DivisionByZero));
}

/// **The two representations are one value.** A dense tensor and the
/// nested vector holding the same lanes compare equal and hash alike —
/// and stop doing both when the reasons differ, exactly as the same two
/// NILs do as scalars.
#[test]
fn a_nil_lane_reconciles_across_the_two_representations() {
    fn hash_of(value: &Value) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    let dense = Value::from_vector_promoted(vec![Value::from_int(1), div_by_zero()]);
    let nested = Value {
        data: ValueData::Vector(Arc::new(vec![Value::from_int(1), div_by_zero()])),
        hint: Interpretation::Unassigned,
        absence: None,
    };
    assert_eq!(dense, nested);
    assert_eq!(hash_of(&dense), hash_of(&nested));

    let written = Value {
        data: ValueData::Vector(Arc::new(vec![Value::from_int(1), Value::nil()])),
        hint: Interpretation::Unassigned,
        absence: None,
    };
    assert_ne!(
        dense, written,
        "a computed absence is not a written one, in either representation"
    );
    assert_ne!(hash_of(&dense), hash_of(&written));
}

/// **An absent lane keeps its reason across the arena boundary.**
///
/// The arena stored a tensor as `Vec<Fraction>`, which records that a lane
/// is absent and nothing about why — the same gap `NodeKind::Nil(reason)`
/// had already closed for a whole-value absence, left open one level down.
#[test]
fn tensor_roundtrip_through_arena_preserves_a_lanes_reason() {
    let value = Value::from_vector_promoted(vec![
        Value::from_int(1),
        Value::nil_with_reason(NilReason::DivisionByZero, Recoverability::Recoverable),
    ]);
    assert!(
        matches!(&value.data, ValueData::Tensor { data, .. } if !data.is_valid(1)),
        "the fixture must be a tensor with an absent lane"
    );

    let (arena, root) = value_to_arena(&value);
    let restored = arena_to_value(&arena, root);
    let ValueData::Tensor { data, .. } = &restored.data else {
        panic!("expected a tensor back, got {:?}", restored.data);
    };
    assert_eq!(data.lane_reason(1), Some(NilReason::DivisionByZero));
    assert_eq!(restored, value);
}

/// **A tensor's absent lane keeps its reason across the persistence
/// boundary.**
///
/// The codec carried a whole-value NIL's reason in its `r` field from the
/// start, and carried a tensor's lanes as bare numerator/denominator columns
/// — which record that a lane is absent and nothing about why. So a saved
/// session reloaded `[ 1 2 ] [ 1 0 ] /` as a vector whose second lane had
/// stopped being a division by zero. Under LANG.VALUES.NIL the reason is the
/// whole observable content of an absence, so that reload returned a
/// different value, which is the one thing this codec promises not to do.
#[test]
fn a_tensors_absent_lane_keeps_its_reason_across_the_boundary() {
    let value = Value::from_vector_promoted(vec![
        Value::from_int(1),
        Value::nil_with_reason(NilReason::DivisionByZero, Recoverability::Recoverable),
    ]);
    assert!(
        matches!(&value.data, ValueData::Tensor { data, .. } if !data.is_valid(1)),
        "the fixture must actually be a tensor with an absent lane: {:?}",
        value.data
    );

    let encoded = encode_stack(std::iter::once((&value, Interpretation::Unassigned)))
        .expect("a tensor encodes");
    let decoded = decode_stack(&encoded).expect("it decodes");
    let restored = &decoded[0].0;

    let ValueData::Tensor { data, .. } = &restored.data else {
        panic!("expected a tensor back, got {:?}", restored.data);
    };
    assert_eq!(data.lane_reason(1), Some(NilReason::DivisionByZero));
    assert_eq!(
        restored, &value,
        "reason and all, the reloaded value is the value that was saved"
    );
}

// ── promotion appends lanes into one buffer, and must index absences the
//    same way it did when each leaf carried its own ─────────────────────────
//
// `try_collect_dense` used to build a `DenseCollect` per leaf and splice each
// one in, offsetting that leaf's absences by the buffer length at the moment of
// the splice. It now appends straight into a shared buffer and records each
// absence at the index it lands on. Those are the same arithmetic only if the
// offset is taken before the lanes are appended and nested runs write absolute
// indices — which is the whole of what a misplaced reason would look like.

#[test]
fn promotion_puts_every_reason_on_the_lane_it_belongs_to() {
    let lanes = vec![
        Value::from_fraction(Fraction::from(1)),
        div_by_zero(),
        Value::from_fraction(Fraction::from(3)),
        Value::nil_with_reason(NilReason::IndexOutOfBounds, Recoverability::Recoverable),
        Value::from_fraction(Fraction::from(5)),
    ];
    let promoted = Value::from_vector_promoted(lanes);

    let ValueData::Tensor { data, shape } = &promoted.data else {
        panic!("a flat run of numeric lanes promotes: {promoted:?}");
    };
    assert_eq!(**shape, vec![5]);
    assert_eq!(
        data.absences()
            .map(|(index, metadata)| (index, metadata.reason))
            .collect::<Vec<_>>(),
        vec![
            (1, Some(NilReason::DivisionByZero)),
            (3, Some(NilReason::IndexOutOfBounds)),
        ],
        "each reason sits on its own lane"
    );
    for (index, rendered) in [(0, "1/1"), (2, "3/1"), (4, "5/1")] {
        assert_eq!(
            promoted.child(index).map(|c| format!("{c}")),
            Some(rendered.to_string()),
            "present lane {index}"
        );
    }
}

#[test]
fn promotion_rebases_a_nested_runs_absences_onto_the_flat_buffer() {
    // Row 1 lane 1 is absent, so the flat index is 1*3 + 1 = 4; row 2 lane 2 is
    // 2*3 + 2 = 8. An offset taken after appending, or a nested run writing
    // indices from its own zero, puts both somewhere else.
    let row = |values: [Option<i64>; 3]| {
        Value::from_vector(
            values
                .into_iter()
                .map(|v| match v {
                    Some(n) => Value::from_fraction(Fraction::from(n)),
                    None => div_by_zero(),
                })
                .collect(),
        )
    };
    let promoted = Value::from_vector_promoted(vec![
        row([Some(1), Some(2), Some(3)]),
        row([Some(4), None, Some(6)]),
        row([Some(7), Some(8), None]),
    ]);

    let ValueData::Tensor { data, shape } = &promoted.data else {
        panic!("a rectangular run of numeric rows promotes: {promoted:?}");
    };
    assert_eq!(**shape, vec![3, 3]);
    assert_eq!(
        data.absences().map(|(index, _)| index).collect::<Vec<_>>(),
        vec![4, 8],
        "nested absences land at their flat index"
    );
    assert!(
        data.absences()
            .all(|(_, metadata)| metadata.reason == Some(NilReason::DivisionByZero)),
        "and keep the reason they were written with"
    );
}

#[test]
fn promotion_refuses_what_it_refused_before_and_leaves_it_nested() {
    // A non-numeric leaf, and a ragged shape. Either must leave the value in
    // its nested form rather than half-promoting the lanes appended before the
    // refusal was reached — the buffer is scratch, and a refusal discards it.
    let with_text = Value::from_vector_promoted(vec![
        Value::from_fraction(Fraction::from(1)),
        Value::from_string("two"),
        Value::from_fraction(Fraction::from(3)),
    ]);
    assert!(
        matches!(with_text.data, ValueData::Vector(_)),
        "a Text leaf blocks promotion: {with_text:?}"
    );
    assert_eq!(with_text.len(), 3, "and every element survives");

    let ragged = Value::from_vector_promoted(vec![
        Value::from_vector(vec![Value::from_fraction(Fraction::from(1))]),
        Value::from_vector(vec![
            Value::from_fraction(Fraction::from(2)),
            Value::from_fraction(Fraction::from(3)),
        ]),
    ]);
    assert!(
        matches!(ragged.data, ValueData::Vector(_)),
        "disagreeing shapes block promotion: {ragged:?}"
    );
    assert_eq!(ragged.len(), 2);
}

#[test]
fn promotion_carries_an_existing_tensors_lanes_and_reasons_in() {
    // A Tensor leaf is the case whose lanes arrive in bulk, so its absences are
    // the ones most easily left at their own indices instead of the buffer's.
    let inner =
        Value::from_vector_promoted(vec![Value::from_fraction(Fraction::from(1)), div_by_zero()]);
    assert!(
        matches!(inner.data, ValueData::Tensor { .. }),
        "inner promotes"
    );

    let promoted = Value::from_vector_promoted(vec![inner.clone(), inner]);
    let ValueData::Tensor { data, shape } = &promoted.data else {
        panic!("two tensors of one shape promote: {promoted:?}");
    };
    assert_eq!(**shape, vec![2, 2]);
    assert_eq!(
        data.absences().map(|(index, _)| index).collect::<Vec<_>>(),
        vec![1, 3],
        "the second copy's hole is offset by the first copy's length"
    );
}
