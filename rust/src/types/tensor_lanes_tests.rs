//! A row of a dense tensor (`Value::child`, `DenseTensor::lanes`) against
//! the row built lane by lane, as it was: every lane read as a `Fraction`,
//! the absences inside the row rebased, and the whole packed again by
//! `Value::from_tensor_with_absences`. The two must be the same value down
//! to its columns, its purity flag and the reason of every absent lane.

use std::collections::BTreeMap;

use proptest::prelude::*;

use super::fraction::Fraction;
use super::{DenseTensor, Value, ValueData};
use crate::error::NilReason;
use crate::semantic::Recoverability;

fn reference_row(data: &DenseTensor, shape: &[usize], index: usize) -> Value {
    let rest: Vec<usize> = shape[1..].to_vec();
    let stride: usize = rest.iter().product();
    let start = index * stride;
    let slice: Vec<Fraction> = (start..start + stride)
        .map(|lane| data.fraction_or_nil(lane))
        .collect();
    let absences = data
        .absences()
        .filter(|(lane, _)| *lane >= start && *lane < start + stride)
        .map(|(lane, metadata)| (lane - start, metadata.clone()))
        .collect();
    Value::from_tensor_with_absences(slice, rest, absences)
}

/// A lane: a small rational, or NIL for one of a few reasons.
fn lane() -> impl Strategy<Value = Option<(i64, i64, u8)>> {
    prop_oneof![
        6 => (-50i64..50, 1i64..4).prop_map(|(n, d)| Some((n, d, 0))),
        1 => (0u8..3).prop_map(|r| Some((0, 0, r + 1))),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn a_row_is_the_lanes_it_was_read_from(
        dims in prop::collection::vec(1usize..4, 2..4),
        lanes in prop::collection::vec(lane(), 64),
    ) {
        let len: usize = dims.iter().product();
        let mut fractions = Vec::with_capacity(len);
        let mut absences = BTreeMap::new();
        for (index, lane) in lanes.iter().cycle().take(len).enumerate() {
            match lane {
                Some((n, d, 0)) => fractions.push(Fraction::new((*n).into(), (*d).into())),
                Some((_, _, reason)) => {
                    fractions.push(Fraction::nil());
                    let reason = match reason {
                        1 => NilReason::DivisionByZero,
                        2 => NilReason::IndexOutOfBounds,
                        _ => NilReason::NotFound,
                    };
                    let nil = Value::nil_with_reason(reason, Recoverability::Recoverable);
                    absences.insert(index, nil.absence.as_deref().cloned().unwrap());
                }
                None => unreachable!(),
            }
        }
        let tensor = DenseTensor::from_fractions_with_absences(fractions, dims.clone(), absences)
            .expect("small lanes pack");
        let value = Value::new(
            ValueData::Tensor {
                data: std::sync::Arc::new(tensor.clone()),
                shape: std::sync::Arc::new(dims.clone()),
            },
            None,
        );
        for index in 0..dims[0] {
            let row = value.child(index).expect("a row");
            let reference = reference_row(&tensor, &dims, index);
            prop_assert_eq!(format!("{row:?}"), format!("{reference:?}"));
            prop_assert_eq!(row.nesting(), reference.nesting());
        }
    }
}
