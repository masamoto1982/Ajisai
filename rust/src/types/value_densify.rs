//! Deciding whether a value can be stored densely, and flattening it when it
//! can.
//!
//! Promotion is the storage half of the two representations `value_identity`
//! reconciles: this module answers "may these children be columns", and that
//! one answers "is the result still the same value". They constrain each
//! other — promoting something whose dense form could not be read back as the
//! value that went in is exactly how a representation loses information — so
//! the arms here and the arms there are meant to be read as a pair.

use super::fraction::Fraction;
use super::{Value, ValueData};
use crate::semantic::AbsenceMetadata;
use std::collections::BTreeMap;

/// A rectangular numeric value flattened into the three things a dense tensor
/// stores: the lanes, their shape, and the reason each absent lane is absent.
///
/// The third used to be missing, and so did the lanes it describes:
/// [`try_dense_value`] refused a NIL child outright, because the only thing
/// dense storage could have said about one was that it was absent. Refusing
/// was the right call while that was true — a densified NIL came back claiming
/// the program had written it — and it is why a vector holding an absence kept
/// its nested form. With the reason stored beside the lane the refusal is no
/// longer needed, so a NIL is an ordinary lane again.
pub(super) struct DenseCollect {
    pub(super) data: Vec<Fraction>,
    pub(super) shape: Vec<usize>,
    pub(super) absences: BTreeMap<usize, AbsenceMetadata>,
}

/// Walk a list of `Value`s into a [`DenseCollect`] if every leaf is a numeric
/// lane — a Fraction scalar, a NIL, or a child Tensor — and the shape is
/// rectangular. `None` if any leaf is outside that (Boolean, Text, Symbol,
/// ExactScalar) or if shapes disagree.
///
/// The lanes are appended into one buffer as they are found. Each leaf used to
/// answer with a `DenseCollect` of its own, which meant a scalar — the common
/// leaf, and the only one `MAP` over a flat vector ever produces — was carried
/// from the value to the buffer inside a freshly heap-allocated one-element
/// `Vec` that was consumed and dropped on the next line. Promoting n scalars
/// therefore made n allocations whose entire contents were one `Fraction`.
pub(super) fn try_collect_dense(values: &[Value]) -> Option<DenseCollect> {
    let mut data = Vec::with_capacity(values.len());
    let mut absences = BTreeMap::new();
    let shape = append_dense_values(values, &mut data, &mut absences)?;
    Some(DenseCollect {
        data,
        shape,
        absences,
    })
}

/// Append every value's lanes to `data`, recording absences at their absolute
/// index, and answer with the shape the whole run forms.
///
/// A refusal part-way through leaves junk in the buffers, which is sound
/// because the only caller owns them and drops them on `None`; nothing reads a
/// buffer this returned `None` for.
fn append_dense_values(
    values: &[Value],
    data: &mut Vec<Fraction>,
    absences: &mut BTreeMap<usize, AbsenceMetadata>,
) -> Option<Vec<usize>> {
    if values.is_empty() {
        return None;
    }
    let mut inner_shape: Option<Vec<usize>> = None;
    for value in values {
        let child_shape = append_dense_value(value, data, absences)?;
        match &inner_shape {
            None => inner_shape = Some(child_shape),
            Some(expected) if *expected == child_shape => {}
            Some(_) => return None,
        }
    }
    let mut shape = vec![values.len()];
    shape.extend(inner_shape.expect("values is non-empty, so a shape was set"));
    Some(shape)
}

/// Append one value's lanes, answering with its own shape (empty for a leaf).
fn append_dense_value(
    value: &Value,
    data: &mut Vec<Fraction>,
    absences: &mut BTreeMap<usize, AbsenceMetadata>,
) -> Option<Vec<usize>> {
    let offset = data.len();
    match &value.data {
        ValueData::Scalar(fraction) => {
            data.push(fraction.clone());
            Some(Vec::new())
        }
        // A NIL is a rank-0 numeric lane: the absence sentinel, and the reason
        // for it stored beside the lane. Storing the reason is what makes this
        // arm possible — see [`DenseCollect`].
        ValueData::Nil => {
            data.push(Fraction::nil());
            if let Some(metadata) = value.absence_metadata() {
                absences.insert(offset, metadata.clone());
            }
            Some(Vec::new())
        }
        // ExactScalar cannot be densified into a Fraction tensor
        ValueData::ExactScalar(_) => None,
        // A Record is not a numeric lane.
        ValueData::Record(_) => None,
        ValueData::Tensor {
            data: tensor,
            shape,
        } => {
            data.extend(tensor.iter());
            for (index, metadata) in tensor.absences() {
                absences.insert(offset + index, metadata.clone());
            }
            Some((**shape).clone())
        }
        ValueData::Vector(children) => append_dense_values(children, data, absences),
        ValueData::Boolean(_) | ValueData::Text(_) | ValueData::Symbol(_) => None,
    }
}
