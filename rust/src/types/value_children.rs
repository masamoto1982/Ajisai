//! Sequence and child-access behavior for [`Value`].
//!
//! Invariant: mutation hydrates dense tensors before exposing child storage, so
//! callers observe one sequence API regardless of the internal representation.

use super::fraction::Fraction;
use super::value_tensor::{tensor_child, tensor_to_nested_values};
use super::{Value, ValueData};
use std::sync::Arc;

impl Value {
    #[inline]
    pub fn len(&self) -> usize {
        match &self.data {
            ValueData::Nil => 0,
            // CS4 PR-2: U is a single scalar truth value, so it has length 1
            // like a Boolean (not 0 like an absence). It is not indexable —
            // `get_child`/`child` return `None`, exactly as for a Boolean.
            // A String is one value, not a sequence of characters. Its
            // character count is reached through `CHARS`, which is what makes
            // the Vector domain explicit; LENGTH raises `nonVector` on it.
            ValueData::Boolean(_) | ValueData::Text(_) | ValueData::Symbol(_) => 1,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => 1,
            ValueData::Vector(v) => v.len(),
            ValueData::Tensor { data, shape } => {
                if shape.is_empty() {
                    data.len()
                } else {
                    shape[0]
                }
            }
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_child(&self, index: usize) -> Option<&Value> {
        match &self.data {
            ValueData::Vector(v) => v.get(index),
            ValueData::Tensor { .. } => None,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) if index == 0 => Some(self),
            ValueData::Boolean(_)
            | ValueData::Text(_)
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Symbol(_) => None,
        }
    }

    /// Representation-agnostic child accessor. Works for both `Vector` and
    /// `Tensor` payloads by materializing a sub-Value (Scalar leaf or
    /// sub-Tensor) when the receiver is dense. Cloning is cheap because
    /// inner buffers are reference-counted.
    ///
    /// Prefer this over [`get_child`] when the call site can be reached with
    /// a dense `Tensor` input. Use `get_child` only when the caller is known
    /// to operate on `Record` or already-nested `Vector` payloads.
    pub fn child(&self, index: usize) -> Option<Value> {
        match &self.data {
            ValueData::Vector(v) => v.get(index).cloned(),
            ValueData::Scalar(_) | ValueData::ExactScalar(_) if index == 0 => Some(self.clone()),
            ValueData::Tensor { data, shape } => tensor_child(data, shape, index),
            ValueData::Boolean(_)
            | ValueData::Text(_)
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Symbol(_) => None,
        }
    }

    #[inline]
    pub fn first(&self) -> Option<&Value> {
        self.get_child(0)
    }

    #[inline]
    pub fn last(&self) -> Option<&Value> {
        match &self.data {
            ValueData::Vector(v) => v.last(),
            ValueData::Tensor { .. } => None,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => Some(self),
            ValueData::Nil => None,
            ValueData::Boolean(_) | ValueData::Text(_) | ValueData::Symbol(_) => None,
        }
    }

    /// Convert a `ValueData::Tensor` in-place to a nested `ValueData::Vector`
    /// so that mutating helpers (push/pop/insert/remove/replace) can operate
    /// on a uniform `Vec<Value>` representation.
    fn hydrate_tensor_to_vector(&mut self) {
        let ValueData::Tensor { data, shape } = &self.data else {
            return;
        };
        let children = tensor_to_nested_values(data, shape);
        self.data = ValueData::Vector(Arc::new(children));
    }

    pub fn push_child(&mut self, child: Value) {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        match &mut self.data {
            ValueData::Vector(v) => {
                Arc::make_mut(v).push(child);
            }
            ValueData::Nil => {
                self.data = ValueData::Vector(Arc::new(vec![child]));
            }
            ValueData::Scalar(f) => {
                let old = Value::from_fraction(f.clone());
                self.data = ValueData::Vector(Arc::new(vec![old, child]));
            }
            ValueData::ExactScalar(_) => {
                // Cannot push_child into an ExactScalar — silently ignore
                // (ExactScalar is always a scalar leaf, never mutated into a vector).
            }
            // CS4 PR-2: pushing into U is a no-op, like a Boolean — U is a
            // scalar truth value, not an empty container to be seeded into a
            // vector (that NIL affordance does not apply to a definite datum).
            ValueData::Boolean(_)
            | ValueData::Text(_)
            | ValueData::Tensor { .. }
            | ValueData::Symbol(_) => {}
        }
    }

    #[inline]
    pub fn as_scalar(&self) -> Option<&Fraction> {
        match &self.data {
            ValueData::Scalar(f) => Some(f),
            ValueData::Boolean(_)
            | ValueData::Text(_)
            | ValueData::ExactScalar(_)
            | ValueData::Vector(_)
            | ValueData::Tensor { .. }
            | ValueData::Nil
            | ValueData::Symbol(_) => None,
        }
    }

    #[inline]
    pub fn as_i64(&self) -> Option<i64> {
        self.as_scalar().and_then(|f| f.to_i64())
    }

    #[inline]
    pub fn as_usize(&self) -> Option<usize> {
        self.as_scalar().and_then(|f| f.as_usize())
    }

    #[inline]
    pub fn as_vector(&self) -> Option<&Vec<Value>> {
        match &self.data {
            ValueData::Vector(v) => Some(v),
            ValueData::Tensor { .. } => None,
            ValueData::Boolean(_)
            | ValueData::Text(_)
            | ValueData::Scalar(_)
            | ValueData::ExactScalar(_)
            | ValueData::Nil
            | ValueData::Symbol(_) => None,
        }
    }

    pub fn collect_fractions_flat(&self) -> Vec<Fraction> {
        let mut buf = Vec::new();
        self.collect_fractions_flat_into(&mut buf);
        buf
    }

    pub fn collect_fractions_flat_into(&self, buf: &mut Vec<Fraction>) {
        match &self.data {
            ValueData::Nil => buf.push(Fraction::nil()),
            ValueData::Scalar(f) => buf.push(f.clone()),
            ValueData::ExactScalar(er) => {
                // Use best rational approximation for ExactScalar in flat collection
                if let Some(f) = er.as_rational() {
                    buf.push(f.clone());
                }
                // non-rational ExactScalars are not representable as a single Fraction
            }
            ValueData::Vector(v) => {
                for child in v.iter() {
                    child.collect_fractions_flat_into(buf);
                }
            }
            ValueData::Tensor { data, .. } => {
                buf.extend(data.iter());
            }
            // CS4 PR-2: U is a truth value, not numeric content — it flattens
            // to no fraction lane, like a Boolean (NIL flattens to a nil
            // lane). Kept in lock-step with `count_fractions` below so buffer
            // sizing stays exact.
            ValueData::Boolean(_) | ValueData::Text(_) | ValueData::Symbol(_) => {}
        }
    }

    pub fn count_fractions(&self) -> usize {
        match &self.data {
            ValueData::Nil => 1,
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => 1,
            ValueData::Vector(v) => v.iter().map(|c| c.count_fractions()).sum(),
            ValueData::Tensor { data, .. } => data.len(),
            // CS4 PR-2: U contributes no fraction lane (see
            // `collect_fractions_flat_into`), matching a Boolean.
            ValueData::Boolean(_) | ValueData::Text(_) | ValueData::Symbol(_) => 0,
        }
    }

    pub fn shape(&self) -> Vec<usize> {
        match &self.data {
            // U and NIL are both rank-0 (empty shape), like a Boolean/Scalar.
            ValueData::Nil => vec![],
            ValueData::Scalar(_) | ValueData::ExactScalar(_) => vec![],
            ValueData::Vector(v) => {
                if v.is_empty() {
                    vec![0]
                } else {
                    let first_shape: Vec<usize> = v[0].shape();
                    let all_same: bool = v.iter().skip(1).all(|c| c.shape() == first_shape);
                    if all_same && !first_shape.is_empty() {
                        let mut shape = vec![v.len()];
                        shape.extend(first_shape);
                        shape
                    } else {
                        vec![v.len()]
                    }
                }
            }
            ValueData::Tensor { shape, .. } => (**shape).clone(),
            ValueData::Boolean(_) | ValueData::Text(_) | ValueData::Symbol(_) => vec![],
        }
    }

    // is_code_block/as_code_block/from_code_block removed with the CodeBlock
    // domain — see docs/dev/type-unification-work-order-2026-08.md. Call
    // sites now use as_vector()/as_vector_view() (Tensor-aware) plus the
    // value-to-tokens bridge in interpreter/value_as_code.rs for "is this
    // executable" and "run it" respectively.

}
