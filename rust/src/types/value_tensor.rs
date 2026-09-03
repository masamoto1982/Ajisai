//! Dense tensor construction, promotion, and representation-boundary helpers.
//!
//! Invariant: promotion is lossless and occurs only for rectangular numeric
//! values; all other vectors retain their ordinary nested representation.

use super::fraction::Fraction;
use super::{DenseTensor, Interpretation, Value, ValueData};
use std::sync::Arc;

impl Value {
    /// Construct a dense `Tensor` value. `data.len()` must equal the product of
    /// `shape` (or `shape` may be empty for a flat 1-D buffer; in that case
    /// `[data.len()]` is used).
    /// Wrap a flat `Vec<i64>` as a 1-D pure-integer dense `Tensor` (SoA),
    /// without materializing per-element `Value`s or `Fraction`s. This is the
    /// output constructor for the integer SIMD lane: it keeps the result in
    /// the same dense column representation as its inputs instead of degrading
    /// to an AoS `Vector` (handoff 手1). The `hint` matches `from_tensor` /
    /// `from_children` (`Unassigned`) so downstream interpretation is unchanged.
    pub fn from_int_tensor(numerators: Vec<i64>) -> Self {
        let len = numerators.len();
        let tensor = DenseTensor::from_integers(numerators);
        Self {
            data: ValueData::Tensor {
                data: Arc::new(tensor),
                shape: Arc::new(vec![len]),
            },
            hint: Interpretation::Unassigned,
            absence: None,
        }
    }

    pub fn from_tensor(data: Vec<Fraction>, shape: Vec<usize>) -> Self {
        let resolved_shape = if shape.is_empty() {
            vec![data.len()]
        } else {
            shape
        };
        let Some(tensor) = DenseTensor::from_fractions(data.clone(), resolved_shape.clone()) else {
            return Self::from_vector_with_hint(
                tensor_fractions_to_nested_values(&data, &resolved_shape),
                Interpretation::Unassigned,
            );
        };
        Self {
            data: ValueData::Tensor {
                data: Arc::new(tensor),
                shape: Arc::new(resolved_shape),
            },
            hint: Interpretation::Unassigned,
            absence: None,
        }
    }

    /// Like [`from_vector_with_hint`] but promotes the value to a dense
    /// `Tensor` when every leaf is a Fraction scalar and the shape is
    /// rectangular. Otherwise the nested form is preserved.
    ///
    /// The `String` display hint suppresses promotion at every level so that
    /// codepoint-based strings retain their nested representation.
    pub fn from_vector_promoted_with_hint(values: Vec<Value>, hint: Interpretation) -> Self {
        if let Some((data, shape)) = try_collect_dense(&values) {
            if let Some(tensor) = DenseTensor::from_fractions(data, shape.clone()) {
                return Self {
                    data: ValueData::Tensor {
                        data: Arc::new(tensor),
                        shape: Arc::new(shape),
                    },
                    hint,
                    absence: None,
                };
            }
        }
        Self {
            data: ValueData::Vector(Arc::new(values)),
            hint,
            absence: None,
        }
    }

    /// Convenience wrapper around [`from_vector_promoted_with_hint`] using
    /// `Interpretation::Unassigned`.
    pub fn from_vector_promoted(values: Vec<Value>) -> Self {
        Self::from_vector_promoted_with_hint(values, Interpretation::Unassigned)
    }
}

/// Walk a list of `Value`s and return `(flat data, shape)` if every leaf is a
/// Fraction scalar (or a child Tensor) and the shape is rectangular. Returns
/// `None` if any leaf is non-numeric (NIL, Record, CodeBlock, Vector with
/// String hint, etc.) or if shapes disagree.
fn try_collect_dense(values: &[Value]) -> Option<(Vec<Fraction>, Vec<usize>)> {
    if values.is_empty() {
        return None;
    }
    let first = try_dense_value(&values[0])?;
    let inner_shape = first.1;
    let mut data = first.0;
    for v in values.iter().skip(1) {
        let (cdata, cshape) = try_dense_value(v)?;
        if cshape != inner_shape {
            return None;
        }
        data.extend(cdata);
    }
    let mut shape = vec![values.len()];
    shape.extend(inner_shape);
    Some((data, shape))
}

/// Materialize the i-th child of a dense Tensor as an owned `Value`. For 1-D
/// shape `[n]` the child is a Scalar; for higher rank the child is itself a
/// dense Tensor with the trailing dimensions.
pub(super) fn tensor_child(data: &DenseTensor, shape: &[usize], index: usize) -> Option<Value> {
    if shape.is_empty() {
        return None;
    }
    let outer = shape[0];
    if index >= outer {
        return None;
    }
    if shape.len() == 1 {
        // An absent lane is still a child — it materializes as NIL through
        // `from_fraction`, not as "no such index".
        return Some(Value::from_fraction(data.fraction_or_nil(index)));
    }
    let rest: Vec<usize> = shape[1..].to_vec();
    let stride: usize = rest.iter().product();
    let start = index * stride;
    let slice: Vec<Fraction> = (start..start + stride)
        .map(|lane| data.fraction_or_nil(lane))
        .collect();
    Some(Value::from_tensor(slice, rest))
}

fn try_dense_value(v: &Value) -> Option<(Vec<Fraction>, Vec<usize>)> {
    match &v.data {
        ValueData::Scalar(f) => Some((vec![f.clone()], Vec::new())),
        ValueData::ExactScalar(_) => None, // ExactScalar cannot be densified into a Fraction tensor
        ValueData::Tensor { data, shape } => Some((data.to_fractions(), (**shape).clone())),
        ValueData::Vector(children) => try_collect_dense(children),
        ValueData::Boolean(_) | ValueData::Text(_) | ValueData::Nil | ValueData::Symbol(_) => None,
    }
}

fn tensor_fractions_to_nested_values(data: &[Fraction], shape: &[usize]) -> Vec<Value> {
    fn build(data: &[Fraction], shape: &[usize], offset: usize) -> Vec<Value> {
        if shape.is_empty() || shape.len() == 1 {
            let len = shape
                .first()
                .copied()
                .unwrap_or_else(|| data.len().saturating_sub(offset));
            return data[offset..offset + len]
                .iter()
                .cloned()
                .map(Value::from_fraction)
                .collect();
        }
        let outer = shape[0];
        let rest = &shape[1..];
        let stride: usize = rest.iter().product();
        let mut out = Vec::with_capacity(outer);
        for i in 0..outer {
            out.push(Value::from_children(build(data, rest, offset + i * stride)));
        }
        out
    }
    build(data, shape, 0)
}

/// Materialize a dense Tensor (`data` + `shape`) as a tree of nested `Value`s.
/// Used by mutating helpers that need a uniform `Vec<Value>` representation,
pub(super) fn tensor_to_nested_values(data: &DenseTensor, shape: &[usize]) -> Vec<Value> {
    fn build(data: &DenseTensor, shape: &[usize], offset: usize) -> Vec<Value> {
        if shape.is_empty() || shape.len() == 1 {
            let len = shape
                .first()
                .copied()
                .unwrap_or_else(|| data.len().saturating_sub(offset));
            return (offset..offset + len)
                .map(|lane| Value::from_fraction(data.fraction_or_nil(lane)))
                .collect();
        }
        let outer = shape[0];
        let rest = &shape[1..];
        let stride: usize = rest.iter().product();
        let mut out = Vec::with_capacity(outer);
        for i in 0..outer {
            let inner = build(data, rest, offset + i * stride);
            out.push(Value::from_children(inner));
        }
        out
    }
    build(data, shape, 0)
}

#[cfg(test)]
mod tensor_boundary_tests {
    use super::*;

    #[test]
    fn tensor_and_nested_vector_compare_equal_when_flatten_matches() {
        let dense = Value::from_tensor(
            vec![
                Fraction::from(1),
                Fraction::from(2),
                Fraction::from(3),
                Fraction::from(4),
            ],
            vec![2, 2],
        );
        let nested = Value::from_children(vec![
            Value::from_children(vec![Value::from_int(1), Value::from_int(2)]),
            Value::from_children(vec![Value::from_int(3), Value::from_int(4)]),
        ]);
        assert_eq!(dense.data, nested.data);
        assert_eq!(nested.data, dense.data);
    }

    #[test]
    fn tensor_shape_matches_nested_shape() {
        let dense = Value::from_tensor(
            vec![
                Fraction::from(1),
                Fraction::from(2),
                Fraction::from(3),
                Fraction::from(4),
            ],
            vec![2, 2],
        );
        assert_eq!(dense.shape(), vec![2, 2]);
        assert_eq!(dense.count_fractions(), 4);
        assert_eq!(dense.collect_fractions_flat().len(), 4);
    }

    #[test]
    fn tensor_with_different_shape_compares_unequal_to_nested() {
        let dense = Value::from_tensor(
            vec![
                Fraction::from(1),
                Fraction::from(2),
                Fraction::from(3),
                Fraction::from(4),
            ],
            vec![4],
        );
        let nested = Value::from_children(vec![
            Value::from_children(vec![Value::from_int(1), Value::from_int(2)]),
            Value::from_children(vec![Value::from_int(3), Value::from_int(4)]),
        ]);
        assert_ne!(dense.data, nested.data);
    }

    #[test]
    fn tensor_is_vector_predicate_holds() {
        let dense = Value::from_tensor(vec![Fraction::from(1)], vec![1]);
        assert!(dense.is_vector());
        assert!(dense.is_tensor());
    }

    #[test]
    fn tensor_hydrates_to_vector_on_push_child() {
        let mut dense = Value::from_tensor(vec![Fraction::from(1), Fraction::from(2)], vec![2]);
        dense.push_child(Value::from_int(3));
        assert!(matches!(dense.data, ValueData::Vector(_)));
        assert_eq!(dense.len(), 3);
    }

    #[test]
    fn dense_tensor_uses_soa_buffers() {
        let dense = Value::from_tensor(
            vec![Fraction::from(1), Fraction::new(3.into(), 2.into())],
            vec![2],
        );
        let ValueData::Tensor { data, shape } = dense.data else {
            panic!("expected DenseTensor representation");
        };
        assert_eq!(&*shape, &[2]);
        assert_eq!(data.numerators, vec![1, 3]);
        assert_eq!(data.denominators, vec![1, 2]);
        assert!(!data.is_pure_integer);
    }

    #[test]
    fn dense_tensor_reads_an_absent_lane_from_the_denominator_sentinel() {
        let tensor = DenseTensor::from_fractions(
            vec![Fraction::from(1), Fraction::nil(), Fraction::from(3)],
            vec![3],
        )
        .expect("small fractions should admit dense representation");

        assert_eq!(tensor.denominators, vec![1, 0, 1]);
        assert!(!tensor.is_valid(1));
        assert!(!tensor.all_lanes_valid());
        assert_eq!(tensor.get_small_fraction(0), Some(Fraction::from(1)));
        assert_eq!(tensor.get_small_fraction(1), None);
        assert!(tensor.fraction_or_nil(1).is_nil());
        assert_eq!(
            tensor.to_fractions(),
            vec![Fraction::from(1), Fraction::nil(), Fraction::from(3)]
        );
    }

    #[test]
    fn big_fraction_tensor_falls_back_without_losing_shape() {
        use num_bigint::BigInt;

        let big = Fraction::new(BigInt::from(i128::from(i64::MAX) + 1), 1.into());
        let value = Value::from_tensor(vec![big.clone()], vec![1]);
        assert!(matches!(value.data, ValueData::Vector(_)));
        assert_eq!(value.shape(), vec![1]);
        assert_eq!(value.collect_fractions_flat(), vec![big]);
    }

    // -----------------------------------------------------------------------
    // VTU Phase III boundary helpers: as_vector_view / ensure_hydrated
    // -----------------------------------------------------------------------

    #[test]
    fn as_vector_view_borrows_for_vector_owns_for_tensor() {
        use std::borrow::Cow;

        let nested = Value::from_children(vec![Value::from_int(1), Value::from_int(2)]);
        match nested.as_vector_view() {
            Some(Cow::Borrowed(slice)) => {
                assert_eq!(slice.len(), 2);
            }
            other => panic!(
                "expected Cow::Borrowed for Vector, got {:?}",
                other.is_some()
            ),
        }

        let dense = Value::from_tensor(vec![Fraction::from(1), Fraction::from(2)], vec![2]);
        match dense.as_vector_view() {
            Some(Cow::Owned(vec)) => {
                assert_eq!(vec.len(), 2);
                assert_eq!(vec[0].as_scalar().map(|f| f.to_i64().unwrap()), Some(1));
                assert_eq!(vec[1].as_scalar().map(|f| f.to_i64().unwrap()), Some(2));
            }
            other => panic!(
                "expected Cow::Owned for Tensor, got {}",
                if other.is_some() { "Borrowed" } else { "None" }
            ),
        }
    }

    #[test]
    fn as_vector_view_returns_none_for_scalar_and_nil() {
        assert!(Value::from_int(7).as_vector_view().is_none());
        assert!(Value::nil().as_vector_view().is_none());
    }

    #[test]
    fn ensure_hydrated_borrows_non_tensor_in_place() {
        use std::borrow::Cow;

        let nested = Value::from_children(vec![Value::from_int(1)]);
        match nested.ensure_hydrated() {
            Cow::Borrowed(_) => {}
            Cow::Owned(_) => panic!("Vector should not be re-allocated"),
        }

        let scalar = Value::from_int(3);
        match scalar.ensure_hydrated() {
            Cow::Borrowed(_) => {}
            Cow::Owned(_) => panic!("Scalar should be borrowed in place"),
        }
    }

    #[test]
    fn ensure_hydrated_converts_tensor_into_vector_preserving_hint() {
        use std::borrow::Cow;

        let mut dense = Value::from_tensor(
            vec![Fraction::from(1), Fraction::from(2), Fraction::from(3)],
            vec![3],
        );
        dense.hint = Interpretation::RawNumber;
        let hydrated = dense.ensure_hydrated();
        match hydrated {
            Cow::Owned(v) => {
                assert!(matches!(v.data, ValueData::Vector(_)));
                assert_eq!(v.hint, Interpretation::RawNumber);
                assert_eq!(v.len(), 3);
            }
            Cow::Borrowed(_) => panic!("Tensor should hydrate into an owned Vector"),
        }
    }
}
