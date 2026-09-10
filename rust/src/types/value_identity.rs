//! When a dense tensor and a nested `Vector` are the same value.
//!
//! `Value` has two representations for one thing — a rectangular numeric
//! collection is either `ValueData::Tensor` (struct-of-arrays columns) or
//! `ValueData::Vector` (a tree of child `Value`s) — and which one a program
//! ends up holding is a storage decision no clause of the specification makes.
//! LANG.STACK.ORDER makes value identity a semantic question, so the two forms
//! must answer `EQ` and hash alike whenever they hold the same content. This
//! module is that correspondence, kept in one place because `PartialEq` and
//! `Hash` have to agree about it and drift apart the moment they are written
//! twice.
//!
//! An absent lane is where the correspondence is easiest to get wrong. A NIL
//! child of a `Vector` carries its reason on the child `Value`; the same lane
//! of a `Tensor` carries it in the tensor's own per-lane absence map. Under
//! LANG.VALUES.NIL the reason is the whole observable content of an absence,
//! so both sides read it and neither reads more: `Value`'s equality is
//! `data == data && nil_reason == nil_reason`, and the rest of an
//! `AbsenceMetadata` is provenance.

use super::fraction::Fraction;
use super::{DenseTensor, Value, ValueData};
use crate::error::NilReason;

pub(super) fn tensor_eq_vector(data: &DenseTensor, shape: &[usize], v: &[Value]) -> bool {
    // A dense tensor is always rectangular, so a ragged nested vector (no
    // well-defined rectangular shape) can never equal one. `nested_vector_shape`
    // returns `None` for ragged structures, which fails the comparison here
    // rather than colliding with the dense shape via a count-only fallback.
    let Some(nested_shape) = nested_vector_shape(v) else {
        return false;
    };
    if nested_shape != shape {
        return false;
    }
    let mut idx = 0usize;
    nested_flatten_matches(v, data, &mut idx) && idx == data.len()
}

/// The rectangular shape of a nested vector, or `None` when the structure is
/// ragged (sibling elements with differing shapes, or mixed scalar/vector
/// siblings). Used only for dense-tensor equality, which requires a
/// rectangular counterpart.
fn nested_vector_shape(v: &[Value]) -> Option<Vec<usize>> {
    if v.is_empty() {
        return Some(vec![0]);
    }
    let first_shape = element_rect_shape(&v[0])?;
    for child in v.iter().skip(1) {
        if element_rect_shape(child)? != first_shape {
            return None;
        }
    }
    let mut s = vec![v.len()];
    s.extend(first_shape);
    Some(s)
}

/// Rectangular shape of a single value, or `None` for non-numeric leaves or
/// ragged sub-structures.
fn element_rect_shape(value: &Value) -> Option<Vec<usize>> {
    match &value.data {
        ValueData::Scalar(_) | ValueData::ExactScalar(_) | ValueData::Nil => Some(Vec::new()),
        // A String is not a numeric leaf, so it has no rectangular element
        // shape and forces the structural (non-dense) path, like a Boolean.
        ValueData::Text(_) => None,
        ValueData::Tensor { shape, .. } => Some((**shape).clone()),
        ValueData::Vector(items) => nested_vector_shape(items),
        // The logical Unknown (U — `Nil` carrying the `TruthValue` hint)
        // has no dedicated variant, so it takes the `Nil` arm above too and
        // counts as a rank-0 element (a nil lane, via the valid-mask), same
        // as an operational NIL.
        ValueData::Boolean(_) | ValueData::Symbol(_) => None,
    }
}

fn nested_flatten_matches(v: &[Value], data: &DenseTensor, idx: &mut usize) -> bool {
    for child in v {
        match &child.data {
            ValueData::Scalar(f) => {
                if *idx >= data.len() || data.fraction_or_nil(*idx) != *f {
                    return false;
                }
                *idx += 1;
            }
            // A NIL matches an absent lane carrying the same reason. Both
            // halves matter: `is_valid` alone would let a NIL equal a zero
            // (an absent lane's numerator is 0 too), and the lane alone would
            // let `NIL(divisionByZero)` equal a written `NIL`, which the same
            // two values as scalars already refuse.
            ValueData::Nil => {
                if *idx >= data.len()
                    || data.is_valid(*idx)
                    || data.lane_reason(*idx) != child.nil_reason().copied()
                {
                    return false;
                }
                *idx += 1;
            }
            // ExactScalar cannot equal a dense-tensor Fraction element
            ValueData::ExactScalar(_) => return false,
            ValueData::Vector(inner) => {
                if !nested_flatten_matches(inner, data, idx) {
                    return false;
                }
            }
            ValueData::Tensor {
                data: inner_data, ..
            } => {
                for lane in 0..inner_data.len() {
                    if *idx >= data.len()
                        || data.fraction_or_nil(*idx) != inner_data.fraction_or_nil(lane)
                        || data.lane_reason(*idx) != inner_data.lane_reason(lane)
                    {
                        return false;
                    }
                    *idx += 1;
                }
            }
            _ => return false,
        }
    }
    true
}

/// Flatten a nested `Vector` into `(shape, leaves, lane reasons)` the same way
/// `nested_flatten_matches` walks it against a dense tensor's lanes:
/// `Scalar` contributes its `Fraction`, `Nil` an absent lane and the reason
/// for it, `Tensor` its own dense lanes, `Vector` recurses, and anything else
/// (`ExactScalar`, `Boolean`, `Text`, `Symbol`) fails the flatten — mirroring
/// exactly which leaves `nested_flatten_matches` is willing to match against a
/// tensor lane. Used only to make [`ValueData`]'s `Hash` agree with the
/// `Vector`/`Tensor` cross-equality in `PartialEq`: a value that *can*
/// equal a dense tensor must hash the way that tensor does.
pub(super) type DenseFlatten = (Vec<usize>, Vec<Fraction>, Vec<(usize, Option<NilReason>)>);

pub(super) fn dense_flatten(v: &[Value]) -> Option<DenseFlatten> {
    let shape = nested_vector_shape(v)?;
    let mut leaves = Vec::new();
    let mut reasons = Vec::new();
    if collect_dense_leaves(v, &mut leaves, &mut reasons) {
        Some((shape, leaves, reasons))
    } else {
        None
    }
}

/// The absent lanes of a dense tensor and the reason for each, in lane order
/// — the hash counterpart of what [`DenseTensor`]'s `PartialEq` compares. An
/// absent lane always contributes an entry, `None` when it carries no reason,
/// so a reasonless absence cannot hash like a reasoned one.
pub(super) fn dense_lane_reasons(data: &DenseTensor) -> Vec<(usize, Option<NilReason>)> {
    (0..data.len())
        .filter(|index| !data.is_valid(*index))
        .map(|index| (index, data.lane_reason(index)))
        .collect()
}

fn collect_dense_leaves(
    v: &[Value],
    out: &mut Vec<Fraction>,
    reasons: &mut Vec<(usize, Option<NilReason>)>,
) -> bool {
    for child in v {
        match &child.data {
            ValueData::Scalar(f) => out.push(f.clone()),
            ValueData::Nil => {
                reasons.push((out.len(), child.nil_reason().copied()));
                out.push(Fraction::nil());
            }
            ValueData::Vector(inner) => {
                if !collect_dense_leaves(inner, out, reasons) {
                    return false;
                }
            }
            ValueData::Tensor { data, .. } => {
                let offset = out.len();
                reasons.extend(
                    dense_lane_reasons(data)
                        .into_iter()
                        .map(|(index, reason)| (index + offset, reason)),
                );
                out.extend(data.iter());
            }
            _ => return false,
        }
    }
    true
}
