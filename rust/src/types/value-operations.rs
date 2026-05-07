use super::fraction::Fraction;
use super::interval::Interval;
use super::{DisplayHint, Token, Value, ValueData};
use crate::error::NilReason;
use std::rc::Rc;

impl Value {
    #[inline]
    pub fn nil() -> Self {
        Self {
            data: ValueData::Nil,
            hint: DisplayHint::Nil,
            nil_reason: None,
        }
    }

    #[inline]
    pub fn nil_with_reason(reason: NilReason) -> Self {
        Self {
            data: ValueData::Nil,
            hint: DisplayHint::Nil,
            nil_reason: Some(reason),
        }
    }

    #[inline]
    pub fn nil_reason(&self) -> Option<&NilReason> {
        self.nil_reason.as_ref()
    }

    #[inline]
    pub fn from_fraction(f: Fraction) -> Self {
        Self {
            data: ValueData::Scalar(f),
            hint: DisplayHint::Number,
            nil_reason: None,
        }
    }

    #[inline]
    pub fn from_int(n: i64) -> Self {
        Self {
            data: ValueData::Scalar(Fraction::from(n)),
            hint: DisplayHint::Number,
            nil_reason: None,
        }
    }

    #[inline]
    pub fn from_bool(b: bool) -> Self {
        Self {
            data: ValueData::Scalar(Fraction::from(if b { 1 } else { 0 })),
            hint: DisplayHint::Number,
            nil_reason: None,
        }
    }

    pub fn from_string(s: &str) -> Self {
        let mut children: Vec<Value> = Vec::with_capacity(s.chars().count());
        for c in s.chars() {
            children.push(Value::from_int(c as u32 as i64));
        }
        if children.is_empty() {
            return Self::nil_with_reason(NilReason::EmptySequence);
        }
        Self {
            data: ValueData::Vector(Rc::new(children)),
            hint: DisplayHint::String,
            nil_reason: None,
        }
    }

    pub fn from_symbol(s: &str) -> Self {
        Self::from_string(s)
    }

    #[inline]
    pub fn from_children(children: Vec<Value>) -> Self {
        Self {
            data: ValueData::Vector(Rc::new(children)),
            hint: DisplayHint::Auto,
            nil_reason: None,
        }
    }

    #[inline]
    pub fn from_children_with_hint(children: Vec<Value>, hint: DisplayHint) -> Self {
        Self {
            data: ValueData::Vector(Rc::new(children)),
            hint,
            nil_reason: None,
        }
    }

    pub fn from_vector(values: Vec<Value>) -> Self {
        if values.is_empty() {
            return Self::nil_with_reason(NilReason::EmptySequence);
        }
        Self {
            data: ValueData::Vector(Rc::new(values)),
            hint: DisplayHint::Auto,
            nil_reason: None,
        }
    }

    pub fn from_vector_with_hint(values: Vec<Value>, hint: DisplayHint) -> Self {
        if values.is_empty() {
            return Self::nil_with_reason(NilReason::EmptySequence);
        }
        Self {
            data: ValueData::Vector(Rc::new(values)),
            hint,
            nil_reason: None,
        }
    }

    #[inline]
    pub fn from_number(f: Fraction) -> Self {
        Self::from_fraction(f)
    }

    #[inline]
    pub fn from_interval(interval: Interval) -> Self {
        Self {
            data: ValueData::Vector(Rc::new(vec![
                Value::from_fraction(interval.lo),
                Value::from_fraction(interval.hi),
            ])),
            hint: DisplayHint::Interval,
            nil_reason: None,
        }
    }

    #[inline]
    pub fn from_datetime(f: Fraction) -> Self {
        Self {
            data: ValueData::Scalar(f),
            hint: DisplayHint::DateTime,
            nil_reason: None,
        }
    }

    #[inline]
    pub fn is_nil(&self) -> bool {
        matches!(self.data, ValueData::Nil)
    }

    #[inline]
    pub fn is_scalar(&self) -> bool {
        matches!(self.data, ValueData::Scalar(_))
    }

    #[inline]
    pub fn is_vector(&self) -> bool {
        matches!(
            self.data,
            ValueData::Vector(_) | ValueData::Tensor { .. } | ValueData::Record { .. }
        )
    }

    #[inline]
    pub fn is_tensor(&self) -> bool {
        matches!(self.data, ValueData::Tensor { .. })
    }

    /// Borrow the dense numeric backing of a `Tensor` value as
    /// `(data, shape)`. Returns `None` for any other representation.
    /// Use this on hot HOF paths to iterate `Fraction`s directly without
    /// materializing per-element `Value`s.
    #[inline]
    pub fn as_dense_tensor(&self) -> Option<(&[Fraction], &[usize])> {
        match &self.data {
            ValueData::Tensor { data, shape } => Some((data.as_slice(), shape.as_slice())),
            _ => None,
        }
    }

    /// Borrow the children of an iterable `Value` as a `Cow<[Value]>`.
    /// `Vector` and `Record` borrow their backing slice directly; `Tensor`
    /// materializes its children once into an owned `Vec<Value>`. Non-iterable
    /// kinds (`Scalar`, `Nil`, `CodeBlock`, handles) return `None`.
    ///
    /// Use this in non-hot consumers (JSON serialization, sort, structural
    /// helpers) so they only need a single iteration path regardless of
    /// whether the value is `Vector` or `Tensor`. For tight numeric loops
    /// prefer [`as_dense_tensor`] which returns `&[Fraction]` without
    /// materializing per-element `Value`s.
    pub fn as_vector_view(&self) -> Option<std::borrow::Cow<'_, [Value]>> {
        match &self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                Some(std::borrow::Cow::Borrowed(v.as_slice()))
            }
            ValueData::Tensor { data, shape } => Some(std::borrow::Cow::Owned(
                tensor_to_nested_values(data, shape),
            )),
            ValueData::Scalar(_)
            | ValueData::Nil
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    /// Return a `Cow<Value>` that is guaranteed to use a non-`Tensor`
    /// representation. `Tensor` values are converted into a nested
    /// `ValueData::Vector` (preserving `hint` and `nil_reason`); every other
    /// variant is borrowed in place.
    ///
    /// Useful at user-visible boundaries (PRINT, JSON-EXPORT, GUI hand-off,
    /// error message formatting) where the caller wants to operate on a
    /// uniform `Vector` shape without caring whether the producer happened to
    /// emit a dense `Tensor`.
    pub fn ensure_hydrated(&self) -> std::borrow::Cow<'_, Value> {
        match &self.data {
            ValueData::Tensor { data, shape } => {
                let children = tensor_to_nested_values(data, shape);
                std::borrow::Cow::Owned(Value {
                    data: ValueData::Vector(Rc::new(children)),
                    hint: self.hint,
                    nil_reason: self.nil_reason.clone(),
                })
            }
            _ => std::borrow::Cow::Borrowed(self),
        }
    }

    #[inline]
    pub fn is_uniquely_owned(&self) -> bool {
        match &self.data {
            ValueData::Scalar(_) | ValueData::Nil => true,
            ValueData::Vector(rc) => Rc::strong_count(rc) == 1,
            ValueData::Tensor { data, shape } => {
                Rc::strong_count(data) == 1 && Rc::strong_count(shape) == 1
            }
            ValueData::Record { pairs, .. } => Rc::strong_count(pairs) == 1,
            ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => false,
        }
    }

    #[inline]
    pub fn is_truthy(&self) -> bool {
        match &self.data {
            ValueData::Nil => false,
            ValueData::Scalar(f) => !f.is_zero() && !f.is_nil(),
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                !v.is_empty() && !v.iter().all(|c| !c.is_truthy())
            }
            ValueData::Tensor { data, .. } => {
                !data.is_empty() && !data.iter().all(|f| f.is_zero() || f.is_nil())
            }
            ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => true,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match &self.data {
            ValueData::Nil => 0,
            ValueData::Scalar(_) => 1,
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => v.len(),
            ValueData::Tensor { data, shape } => {
                if shape.is_empty() {
                    data.len()
                } else {
                    shape[0]
                }
            }
            ValueData::CodeBlock(tokens) => tokens.len(),
            ValueData::ProcessHandle(_) | ValueData::SupervisorHandle(_) => 1,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_child(&self, index: usize) -> Option<&Value> {
        match &self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => v.get(index),
            ValueData::Tensor { .. } => None,
            ValueData::Scalar(_) if index == 0 => Some(self),
            ValueData::Scalar(_)
            | ValueData::Nil
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
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
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => v.get(index).cloned(),
            ValueData::Scalar(_) if index == 0 => Some(self.clone()),
            ValueData::Tensor { data, shape } => tensor_child(data, shape, index),
            ValueData::Scalar(_)
            | ValueData::Nil
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    pub fn get_child_mut(&mut self, index: usize) -> Option<&mut Value> {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                Rc::make_mut(v).get_mut(index)
            }
            ValueData::Tensor { .. }
            | ValueData::Scalar(_)
            | ValueData::Nil
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    #[inline]
    pub fn first(&self) -> Option<&Value> {
        self.get_child(0)
    }

    #[inline]
    pub fn last(&self) -> Option<&Value> {
        match &self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => v.last(),
            ValueData::Tensor { .. } => None,
            ValueData::Scalar(_) => Some(self),
            ValueData::Nil => None,
            ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
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
        self.data = ValueData::Vector(Rc::new(children));
    }

    pub fn push_child(&mut self, child: Value) {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                Rc::make_mut(v).push(child);
            }
            ValueData::Nil => {
                self.data = ValueData::Vector(Rc::new(vec![child]));
            }
            ValueData::Scalar(f) => {
                let old = Value::from_fraction(f.clone());
                self.data = ValueData::Vector(Rc::new(vec![old, child]));
            }
            ValueData::Tensor { .. }
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => {}
        }
    }

    pub fn pop_child(&mut self) -> Option<Value> {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v).pop(),
            ValueData::Tensor { .. }
            | ValueData::Scalar(_)
            | ValueData::Nil
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    pub fn insert_child(&mut self, index: usize, child: Value) {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        let v: &mut Vec<Value> = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v),
            ValueData::Tensor { .. }
            | ValueData::Scalar(_)
            | ValueData::Nil
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => return,
        };
        if index <= v.len() {
            v.insert(index, child);
        }
    }

    pub fn remove_child(&mut self, index: usize) -> Option<Value> {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        let v: &mut Vec<Value> = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v),
            ValueData::Tensor { .. }
            | ValueData::Scalar(_)
            | ValueData::Nil
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => return None,
        };
        if index < v.len() {
            Some(v.remove(index))
        } else {
            None
        }
    }

    pub fn replace_child(&mut self, index: usize, child: Value) -> Option<Value> {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        let v: &mut Vec<Value> = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v),
            ValueData::Tensor { .. }
            | ValueData::Scalar(_)
            | ValueData::Nil
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => return None,
        };
        if index < v.len() {
            Some(std::mem::replace(&mut v[index], child))
        } else {
            None
        }
    }

    #[inline]
    pub fn as_scalar(&self) -> Option<&Fraction> {
        match &self.data {
            ValueData::Scalar(f) => Some(f),
            ValueData::Vector(_)
            | ValueData::Tensor { .. }
            | ValueData::Record { .. }
            | ValueData::Nil
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    #[inline]
    pub fn as_scalar_mut(&mut self) -> Option<&mut Fraction> {
        match &mut self.data {
            ValueData::Scalar(f) => Some(f),
            ValueData::Vector(_)
            | ValueData::Tensor { .. }
            | ValueData::Record { .. }
            | ValueData::Nil
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
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
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Some(v),
            ValueData::Tensor { .. } => None,
            ValueData::Scalar(_)
            | ValueData::Nil
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
        }
    }

    #[inline]
    pub fn as_vector_mut(&mut self) -> Option<&mut Vec<Value>> {
        if matches!(self.data, ValueData::Tensor { .. }) {
            self.hydrate_tensor_to_vector();
        }
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Some(Rc::make_mut(v)),
            ValueData::Tensor { .. }
            | ValueData::Scalar(_)
            | ValueData::Nil
            | ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => None,
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
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                for child in v.iter() {
                    child.collect_fractions_flat_into(buf);
                }
            }
            ValueData::Tensor { data, .. } => {
                buf.extend(data.iter().cloned());
            }
            ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => {}
        }
    }

    pub fn count_fractions(&self) -> usize {
        match &self.data {
            ValueData::Nil => 1,
            ValueData::Scalar(_) => 1,
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                v.iter().map(|c| c.count_fractions()).sum()
            }
            ValueData::Tensor { data, .. } => data.len(),
            ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => 0,
        }
    }

    pub fn shape(&self) -> Vec<usize> {
        match &self.data {
            ValueData::Nil => vec![],
            ValueData::Scalar(_) => vec![],
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
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
            ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => vec![],
        }
    }

    #[inline]
    pub fn is_code_block(&self) -> bool {
        matches!(self.data, ValueData::CodeBlock(_))
    }

    #[inline]
    pub fn as_code_block(&self) -> Option<&Vec<Token>> {
        let ValueData::CodeBlock(tokens) = &self.data else {
            return None;
        };
        Some(tokens)
    }

    pub fn from_code_block(tokens: Vec<Token>) -> Self {
        Self {
            data: ValueData::CodeBlock(tokens),
            hint: DisplayHint::Auto,
            nil_reason: None,
        }
    }

    pub fn from_process_handle(id: u64) -> Self {
        Self {
            data: ValueData::ProcessHandle(id),
            hint: DisplayHint::Auto,
            nil_reason: None,
        }
    }

    pub fn as_process_handle(&self) -> Option<u64> {
        match self.data {
            ValueData::ProcessHandle(id) => Some(id),
            _ => None,
        }
    }

    pub fn from_supervisor_handle(id: u64) -> Self {
        Self {
            data: ValueData::SupervisorHandle(id),
            hint: DisplayHint::Auto,
            nil_reason: None,
        }
    }

    pub fn resolve_default_hint(&self) -> DisplayHint {
        match &self.data {
            ValueData::Nil => DisplayHint::Nil,
            ValueData::Scalar(_) => DisplayHint::Number,
            ValueData::Vector(_) | ValueData::Tensor { .. } | ValueData::Record { .. } => {
                DisplayHint::Auto
            }
            ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => DisplayHint::Auto,
        }
    }

    /// Construct a dense `Tensor` value. `data.len()` must equal the product of
    /// `shape` (or `shape` may be empty for a flat 1-D buffer; in that case
    /// `[data.len()]` is used).
    pub fn from_tensor(data: Vec<Fraction>, shape: Vec<usize>) -> Self {
        if data.is_empty() {
            return Self::nil_with_reason(NilReason::EmptySequence);
        }
        let resolved_shape = if shape.is_empty() {
            vec![data.len()]
        } else {
            shape
        };
        Self {
            data: ValueData::Tensor {
                data: Rc::new(data),
                shape: Rc::new(resolved_shape),
            },
            hint: DisplayHint::Auto,
            nil_reason: None,
        }
    }

    /// Like [`from_vector_with_hint`] but promotes the value to a dense
    /// `Tensor` when every leaf is a Fraction scalar and the shape is
    /// rectangular. Otherwise the nested form is preserved.
    ///
    /// The `String` display hint suppresses promotion at every level so that
    /// codepoint-based strings retain their nested representation.
    pub fn from_vector_promoted_with_hint(values: Vec<Value>, hint: DisplayHint) -> Self {
        if values.is_empty() {
            return Self::nil_with_reason(NilReason::EmptySequence);
        }
        if hint == DisplayHint::String {
            return Self {
                data: ValueData::Vector(Rc::new(values)),
                hint,
                nil_reason: None,
            };
        }
        if let Some((data, shape)) = try_collect_dense(&values) {
            return Self {
                data: ValueData::Tensor {
                    data: Rc::new(data),
                    shape: Rc::new(shape),
                },
                hint,
                nil_reason: None,
            };
        }
        Self {
            data: ValueData::Vector(Rc::new(values)),
            hint,
            nil_reason: None,
        }
    }

    /// Convenience wrapper around [`from_vector_promoted_with_hint`] using
    /// `DisplayHint::Auto`.
    pub fn from_vector_promoted(values: Vec<Value>) -> Self {
        Self::from_vector_promoted_with_hint(values, DisplayHint::Auto)
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
fn tensor_child(
    data: &[Fraction],
    shape: &[usize],
    index: usize,
) -> Option<Value> {
    if shape.is_empty() {
        return None;
    }
    let outer = shape[0];
    if index >= outer {
        return None;
    }
    if shape.len() == 1 {
        return Some(Value::from_fraction(data[index].clone()));
    }
    let rest: Vec<usize> = shape[1..].to_vec();
    let stride: usize = rest.iter().product();
    let slice: Vec<Fraction> = data[index * stride..(index + 1) * stride].to_vec();
    Some(Value::from_tensor(slice, rest))
}

fn try_dense_value(v: &Value) -> Option<(Vec<Fraction>, Vec<usize>)> {
    if v.hint == DisplayHint::String {
        return None;
    }
    match &v.data {
        ValueData::Scalar(f) => Some((vec![f.clone()], Vec::new())),
        ValueData::Tensor { data, shape } => Some(((**data).clone(), (**shape).clone())),
        ValueData::Vector(children) => try_collect_dense(children),
        ValueData::Nil
        | ValueData::Record { .. }
        | ValueData::CodeBlock(_)
        | ValueData::ProcessHandle(_)
        | ValueData::SupervisorHandle(_) => None,
    }
}

/// Materialize a dense Tensor (`data` + `shape`) as a tree of nested `Value`s.
/// Used by mutating helpers that need a uniform `Vec<Value>` representation,
/// and by display fallbacks.
pub(super) fn tensor_to_nested_values(
    data: &[Fraction],
    shape: &[usize],
) -> Vec<Value> {
    if shape.is_empty() {
        return data.iter().map(|f| Value::from_fraction(f.clone())).collect();
    }
    if shape.len() == 1 {
        return data.iter().map(|f| Value::from_fraction(f.clone())).collect();
    }
    let outer = shape[0];
    let rest = &shape[1..];
    let stride: usize = rest.iter().product();
    let mut out = Vec::with_capacity(outer);
    for i in 0..outer {
        let slice = &data[i * stride..(i + 1) * stride];
        let inner = tensor_to_nested_values(slice, rest);
        out.push(Value::from_children(inner));
    }
    out
}

#[cfg(test)]
mod vtu_tensor_tests {
    use super::*;

    #[test]
    fn tensor_and_nested_vector_compare_equal_when_flatten_matches() {
        let dense = Value::from_tensor(
            vec![Fraction::from(1), Fraction::from(2), Fraction::from(3), Fraction::from(4)],
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
            vec![Fraction::from(1), Fraction::from(2), Fraction::from(3), Fraction::from(4)],
            vec![2, 2],
        );
        assert_eq!(dense.shape(), vec![2, 2]);
        assert_eq!(dense.count_fractions(), 4);
        assert_eq!(dense.collect_fractions_flat().len(), 4);
    }

    #[test]
    fn tensor_with_different_shape_compares_unequal_to_nested() {
        let dense = Value::from_tensor(
            vec![Fraction::from(1), Fraction::from(2), Fraction::from(3), Fraction::from(4)],
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
        let mut dense = Value::from_tensor(
            vec![Fraction::from(1), Fraction::from(2)],
            vec![2],
        );
        dense.push_child(Value::from_int(3));
        assert!(matches!(dense.data, ValueData::Vector(_)));
        assert_eq!(dense.len(), 3);
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
            other => panic!("expected Cow::Borrowed for Vector, got {:?}", other.is_some()),
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
        dense.hint = DisplayHint::Number;
        let hydrated = dense.ensure_hydrated();
        match hydrated {
            Cow::Owned(v) => {
                assert!(matches!(v.data, ValueData::Vector(_)));
                assert_eq!(v.hint, DisplayHint::Number);
                assert_eq!(v.len(), 3);
            }
            Cow::Borrowed(_) => panic!("Tensor should hydrate into an owned Vector"),
        }
    }
}
