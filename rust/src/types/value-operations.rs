use super::fraction::Fraction;
use super::{DisplayHint, Token, Value, ValueData};
use std::rc::Rc;

impl Value {
    #[inline]
    pub fn nil() -> Self {
        Self {
            data: ValueData::Nil,
        }
    }

    #[inline]
    pub fn from_fraction(f: Fraction) -> Self {
        Self {
            data: ValueData::Scalar(f),
        }
    }

    #[inline]
    pub fn from_int(n: i64) -> Self {
        Self {
            data: ValueData::Scalar(Fraction::from(n)),
        }
    }

    #[inline]
    pub fn from_bool(b: bool) -> Self {
        Self {
            data: ValueData::Scalar(Fraction::from(if b { 1 } else { 0 })),
        }
    }

    pub fn from_string(s: &str) -> Self {
        let mut children: Vec<Value> = Vec::with_capacity(s.chars().count());
        for c in s.chars() {
            children.push(Value::from_int(c as u32 as i64));
        }
        if children.is_empty() {
            return Self::nil();
        }
        Self {
            data: ValueData::Vector(Rc::new(children)),
        }
    }

    pub fn from_symbol(s: &str) -> Self {
        Self::from_string(s)
    }

    #[inline]
    pub fn from_children(children: Vec<Value>) -> Self {
        Self {
            data: ValueData::Vector(Rc::new(children)),
        }
    }

    pub fn from_vector(values: Vec<Value>) -> Self {
        if values.is_empty() {
            return Self::nil();
        }
        Self {
            data: ValueData::Vector(Rc::new(values)),
        }
    }

    #[inline]
    pub fn from_number(f: Fraction) -> Self {
        Self::from_fraction(f)
    }

    #[inline]
    pub fn from_datetime(f: Fraction) -> Self {
        Self::from_fraction(f)
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
        matches!(self.data, ValueData::Vector(_) | ValueData::Record { .. })
    }

    #[inline]
    pub fn is_uniquely_owned(&self) -> bool {
        match &self.data {
            ValueData::Scalar(_) | ValueData::Nil => true,
            ValueData::Vector(rc) => Rc::strong_count(rc) == 1,
            ValueData::Record { pairs, .. } => Rc::strong_count(pairs) == 1,
            ValueData::CodeBlock(_) => false,
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
            ValueData::CodeBlock(_) => true,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        match &self.data {
            ValueData::Nil => 0,
            ValueData::Scalar(_) => 1,
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => v.len(),
            ValueData::CodeBlock(tokens) => tokens.len(),
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_child(&self, index: usize) -> Option<&Value> {
        match &self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => v.get(index),
            ValueData::Scalar(_) if index == 0 => Some(self),
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => None,
        }
    }

    pub fn get_child_mut(&mut self, index: usize) -> Option<&mut Value> {
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                Rc::make_mut(v).get_mut(index)
            }
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => None,
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
            ValueData::Scalar(_) => Some(self),
            ValueData::Nil => None,
            ValueData::CodeBlock(_) => None,
        }
    }

    pub fn push_child(&mut self, child: Value) {
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
            ValueData::CodeBlock(_) => {}
        }
    }

    pub fn pop_child(&mut self) -> Option<Value> {
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v).pop(),
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => None,
        }
    }

    pub fn insert_child(&mut self, index: usize, child: Value) {
        let v: &mut Vec<Value> = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v),
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => return,
        };
        if index <= v.len() {
            v.insert(index, child);
        }
    }

    pub fn remove_child(&mut self, index: usize) -> Option<Value> {
        let v: &mut Vec<Value> = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v),
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => return None,
        };
        if index < v.len() {
            Some(v.remove(index))
        } else {
            None
        }
    }

    pub fn replace_child(&mut self, index: usize, child: Value) -> Option<Value> {
        let v: &mut Vec<Value> = match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Rc::make_mut(v),
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => return None,
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
            ValueData::Vector(_) | ValueData::Record { .. } | ValueData::Nil | ValueData::CodeBlock(_) => None,
        }
    }

    #[inline]
    pub fn as_scalar_mut(&mut self) -> Option<&mut Fraction> {
        match &mut self.data {
            ValueData::Scalar(f) => Some(f),
            ValueData::Vector(_) | ValueData::Record { .. } | ValueData::Nil | ValueData::CodeBlock(_) => None,
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
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => None,
        }
    }

    #[inline]
    pub fn as_vector_mut(&mut self) -> Option<&mut Vec<Value>> {
        match &mut self.data {
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => Some(Rc::make_mut(v)),
            ValueData::Scalar(_) | ValueData::Nil | ValueData::CodeBlock(_) => None,
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
            ValueData::CodeBlock(_) => {}
        }
    }

    pub fn count_fractions(&self) -> usize {
        match &self.data {
            ValueData::Nil => 1,
            ValueData::Scalar(_) => 1,
            ValueData::Vector(v) | ValueData::Record { pairs: v, .. } => {
                v.iter().map(|c| c.count_fractions()).sum()
            }
            ValueData::CodeBlock(_) => 0,
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
            ValueData::CodeBlock(_) => vec![],
        }
    }

    #[inline]
    pub fn from_numbers(v: Vec<Fraction>) -> Self {
        if v.is_empty() {
            return Self::nil();
        }
        if v.len() == 1 {
            return Self {
                data: ValueData::Scalar(v[0].clone()),
            };
        }
        Self {
            data: ValueData::Vector(Rc::new(v.into_iter().map(Value::from_fraction).collect::<Vec<Value>>())),
        }
    }

    #[inline]
    pub fn from_vec(v: Vec<Fraction>) -> Self {
        if v.is_empty() {
            return Self::nil();
        }
        if v.len() == 1 {
            return Self {
                data: ValueData::Scalar(v[0].clone()),
            };
        }
        Self {
            data: ValueData::Vector(Rc::new(v.into_iter().map(Value::from_fraction).collect::<Vec<Value>>())),
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
        }
    }

    pub fn resolve_default_hint(&self) -> DisplayHint {
        match &self.data {
            ValueData::Nil => DisplayHint::Nil,
            ValueData::Scalar(_) => DisplayHint::Number,
            ValueData::Vector(_) | ValueData::Record { .. } => DisplayHint::Auto,
            ValueData::CodeBlock(_) => DisplayHint::Auto,
        }
    }
}
