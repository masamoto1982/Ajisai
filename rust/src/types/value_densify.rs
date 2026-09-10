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
pub(super) fn try_collect_dense(values: &[Value]) -> Option<DenseCollect> {
    if values.is_empty() {
        return None;
    }
    let first = try_dense_value(&values[0])?;
    let inner_shape = first.shape;
    let mut data = first.data;
    let mut absences = first.absences;
    for v in values.iter().skip(1) {
        let child = try_dense_value(v)?;
        if child.shape != inner_shape {
            return None;
        }
        let offset = data.len();
        data.extend(child.data);
        absences.extend(
            child
                .absences
                .into_iter()
                .map(|(index, metadata)| (index + offset, metadata)),
        );
    }
    let mut shape = vec![values.len()];
    shape.extend(inner_shape);
    Some(DenseCollect {
        data,
        shape,
        absences,
    })
}

fn try_dense_value(v: &Value) -> Option<DenseCollect> {
    match &v.data {
        ValueData::Scalar(f) => Some(DenseCollect {
            data: vec![f.clone()],
            shape: Vec::new(),
            absences: BTreeMap::new(),
        }),
        // A NIL is a rank-0 numeric lane: the absence sentinel, and the reason
        // for it stored beside the lane. Storing the reason is what makes this
        // arm possible — see [`DenseCollect`].
        ValueData::Nil => Some(DenseCollect {
            data: vec![Fraction::nil()],
            shape: Vec::new(),
            absences: v
                .absence_metadata()
                .map(|metadata| BTreeMap::from([(0, metadata.clone())]))
                .unwrap_or_default(),
        }),
        ValueData::ExactScalar(_) => None, // ExactScalar cannot be densified into a Fraction tensor
        ValueData::Tensor { data, shape } => Some(DenseCollect {
            data: data.to_fractions(),
            shape: (**shape).clone(),
            absences: data
                .absences()
                .map(|(index, metadata)| (index, metadata.clone()))
                .collect(),
        }),
        ValueData::Vector(children) => try_collect_dense(children),
        ValueData::Boolean(_) | ValueData::Text(_) | ValueData::Symbol(_) => None,
    }
}
