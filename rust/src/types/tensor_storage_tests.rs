//! Test suite for `crate::types::tensor_storage` — the dense and sparse
//! numeric tensor stores.
//!
//! Lifted out of an inline `#[cfg(test)] mod` so the module it tests stays
//! inside the 500-line budget (docs/dev/specification-implementation-rules.md),
//! and so it sits where every other test file in `types/` already does.

use super::fraction::Fraction;
use super::tensor_storage::{DenseTensor, SparseTensor};

fn dense_from_i64(values: &[i64], shape: Vec<usize>) -> DenseTensor {
    DenseTensor::from_fractions(values.iter().copied().map(Fraction::from).collect(), shape)
        .expect("small dense tensor should build")
}

#[test]
fn dense_tensor_sparse_density_counts_zero_and_nonzero_lanes() {
    let all_zero = dense_from_i64(&vec![0; 64], vec![64]);
    assert_eq!(all_zero.zero_count(), 64);
    assert_eq!(all_zero.nonzero_count(), 0);
    assert_eq!(all_zero.density(), 0.0);
    assert!(all_zero.is_sparse_candidate());

    let all_nonzero = dense_from_i64(&vec![1; 64], vec![64]);
    assert_eq!(all_nonzero.zero_count(), 0);
    assert_eq!(all_nonzero.nonzero_count(), 64);
    assert_eq!(all_nonzero.density(), 1.0);
    assert!(!all_nonzero.is_sparse_candidate());

    let mixed = dense_from_i64(&[0, 7, 0, -3], vec![4]);
    assert_eq!(mixed.zero_count(), 2);
    assert_eq!(mixed.nonzero_count(), 2);
    assert_eq!(mixed.density(), 0.5);
    assert!(!mixed.is_sparse_candidate());
}

#[test]
fn dense_tensor_sparse_density_does_not_count_absent_lanes_as_zero() {
    // An absent lane has numerator 0, exactly like a zero lane, so it is
    // the denominator that tells them apart. A density that counted the
    // two alike would offer a NIL-holding tensor to the sparse form, which
    // stores no absence and would silently read those lanes back as 0.
    let dense = DenseTensor::from_fractions(
        vec![
            Fraction::nil(),
            Fraction::nil(),
            Fraction::from(0_i64),
            Fraction::from(9_i64),
        ],
        vec![4],
    )
    .expect("small fractions admit dense representation");
    assert!(!dense.is_valid(0));
    assert!(dense.is_valid(2), "a zero lane is present, not absent");
    assert_eq!(dense.zero_count(), 1);
    assert_eq!(dense.nonzero_count(), 1);
    assert_eq!(dense.density(), 0.25);
    assert!(SparseTensor::from_dense(&dense).is_none());
}

#[test]
fn sparse_tensor_round_trips_dense_values_and_shape() {
    let dense = dense_from_i64(&[0, 0, 3, 0, -4, 0], vec![2, 3]);
    let sparse = SparseTensor::from_dense(&dense).expect("all-valid dense tensor is sparseable");
    assert_eq!(sparse.shape, vec![2, 3]);
    assert_eq!(sparse.len, 6);
    assert_eq!(sparse.indices, vec![2, 4]);
    assert_eq!(sparse.nonzero_count(), 2);
    assert!(sparse.indices.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(sparse.fraction_or_zero(0), Fraction::from(0_i64));
    assert_eq!(sparse.get_small_fraction(2), Some(Fraction::from(3_i64)));
    assert_eq!(sparse.to_dense(), dense);
}

#[test]
fn sparse_tensor_accepts_all_zero_dense_tensor() {
    let dense = dense_from_i64(&vec![0; 64], vec![8, 8]);
    let sparse = SparseTensor::from_dense(&dense).expect("all-zero all-valid tensor is sparseable");
    assert!(sparse.indices.is_empty());
    assert_eq!(sparse.nonzero_count(), 0);
    assert_eq!(sparse.density(), 0.0);
    assert_eq!(sparse.to_dense(), dense);
}

// ── reversed_lanes ──────────────────────────────────────────────────────────
//
// Reversing columns is easy; carrying the *reason* an absent lane is absent to
// its new index is the part that can silently go wrong. Whether a lane is absent
// travels with the denominator sentinel and so reverses with the column for
// free, which is exactly what would make a broken reason-remap invisible: the
// NILs would land in the right places holding the wrong explanations.

use crate::error::NilReason;
use crate::semantic::{AbsenceMetadata, AbsenceOrigin, Recoverability};
use std::collections::BTreeMap;

fn reasoned(reason: NilReason) -> AbsenceMetadata {
    AbsenceMetadata::with_reason(reason, AbsenceOrigin::Unknown, Recoverability::Unknown)
}

/// A flat tensor whose lane `absent_at` is absent for `reason`, and whose other
/// lanes hold their index.
fn with_absence_at(len: usize, absent_at: usize, reason: NilReason) -> DenseTensor {
    let numerators: Vec<i64> = (0..len).map(|i| i as i64).collect();
    let denominators: Vec<i64> = (0..len)
        .map(|i| if i == absent_at { 0 } else { 1 })
        .collect();
    let mut absences = BTreeMap::new();
    absences.insert(absent_at, reasoned(reason));
    DenseTensor::from_columns(numerators, denominators, vec![len], true, absences)
}

#[test]
fn reversed_lanes_reverses_both_columns() {
    let tensor = DenseTensor::from_integers(vec![1, 2, 3, 4, 5]);
    let reversed = tensor.reversed_lanes();
    assert_eq!(reversed.numerators, vec![5, 4, 3, 2, 1]);
    assert_eq!(reversed.denominators, vec![1, 1, 1, 1, 1]);
    assert_eq!(reversed.shape, vec![5]);
    assert!(reversed.is_pure_integer);
}

#[test]
fn reversed_lanes_carries_each_absence_reason_to_its_new_index() {
    // An absence at lane 1 of 8 belongs at lane 6 afterwards (len - 1 - index),
    // still holding the reason it was given — not the reason of whatever lane
    // happened to land where it used to be.
    let tensor = with_absence_at(8, 1, NilReason::DivisionByZero);
    let reversed = tensor.reversed_lanes();

    assert!(
        !reversed.is_valid(6),
        "the absent lane must land at index 6"
    );
    assert!(reversed.is_valid(1), "index 1 must now hold a number");
    assert_eq!(
        reversed.lane_reason(6),
        Some(NilReason::DivisionByZero),
        "the reason must travel with the lane"
    );
    assert_eq!(
        reversed.lane_reason(1),
        None,
        "a present lane has no reason to report"
    );
}

#[test]
fn reversed_lanes_is_its_own_inverse() {
    for tensor in [
        DenseTensor::from_integers(vec![7, -3, 0, 11]),
        with_absence_at(6, 0, NilReason::DomainMiss),
        with_absence_at(6, 5, NilReason::IndexOutOfBounds),
        with_absence_at(1, 0, NilReason::Literal),
    ] {
        assert_eq!(
            tensor.reversed_lanes().reversed_lanes(),
            tensor,
            "reversing twice must restore the tensor, reasons included"
        );
    }
}

#[test]
fn reversed_lanes_keeps_a_rational_tensor_rational() {
    // Halves, so `is_pure_integer` is false and must stay false: the flag
    // describes the lanes, and reversing does not change what they are.
    let tensor = DenseTensor::from_fractions(
        vec![
            Fraction::new(1.into(), 2.into()),
            Fraction::new(3.into(), 2.into()),
        ],
        vec![2],
    )
    .expect("halves build a dense tensor");
    let reversed = tensor.reversed_lanes();
    assert!(!reversed.is_pure_integer);
    assert_eq!(reversed.numerators, vec![3, 1]);
    assert_eq!(reversed.denominators, vec![2, 2]);
}
