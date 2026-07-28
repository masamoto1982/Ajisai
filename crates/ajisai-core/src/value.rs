//! Values: what flows.
//!
//! Six shapes, and no more. Every one of them is reachable from source, every
//! one has stated propagation rules, and none of them exists to make a
//! metaphor come out even.

use std::fmt;
use std::sync::Arc;

use crate::number::Number;
use crate::role::Role;
use crate::syntax::Node;

/// The Data Plane: what a value is.
#[derive(Clone, Debug)]
pub enum ValueData {
    /// A settled truth value.
    Boolean(bool),
    /// The third truth value of Strong Kleene logic. The flow arrived and the
    /// observation does not settle the question. Distinct at the type level
    /// from [`ValueData::Nil`], so no absence check can silently absorb it.
    Unknown,
    /// The flow arrived carrying no value.
    Nil,
    /// An exact rational.
    Number(Number),
    /// An ordered run of values. Vectors nest; a matrix is a vector of
    /// vectors, and Ajisai needs no separate rank-aware type to say so.
    Vector(Arc<Vec<Value>>),
    /// An unevaluated flow held as a value.
    Quote(Arc<Vec<Node>>),
}

/// A value: what it is, and how it is read.
#[derive(Clone, Debug)]
pub struct Value {
    data: ValueData,
    role: Role,
}

impl Value {
    /// Build a value with a role, without checking that the shape admits it.
    ///
    /// Crate-private on purpose. `SPECIFICATION.md` §6.5 states as an invariant
    /// that a value's role is always admitted by its shape, and an unchecked
    /// public constructor would let a caller outside this crate build a
    /// counterexample. Use [`Value::read_as`] instead, which is the same check
    /// `>TEXT` and `>INTERVAL` perform.
    pub(crate) fn new(data: ValueData, role: Role) -> Self {
        Self { data, role }
    }

    pub fn data(&self) -> &ValueData {
        &self.data
    }

    /// The value's reading on the Semantic Plane. This field is the single
    /// canonical home of that reading; nothing else stores it.
    pub fn role(&self) -> Role {
        self.role
    }

    /// Set the role without checking. Every internal caller either knows the
    /// shape by construction or has already been through
    /// [`role::retain`](crate::role::retain).
    pub(crate) fn with_role(mut self, role: Role) -> Self {
        self.role = role;
        self
    }

    /// Read the value as `role`, checked.
    ///
    /// This is the public way to put a role on a value, and it is the same
    /// check `>TEXT` and `>INTERVAL` perform: a reading the shape contradicts
    /// is refused rather than stored.
    pub fn read_as(self, role: Role) -> crate::error::Result<Value> {
        crate::role::admits(role, &self).map_err(|reason| crate::error::Error::BadRole {
            role: role.name().to_string(),
            reason,
        })?;
        Ok(self.with_role(role))
    }

    pub fn boolean(value: bool) -> Self {
        Self::new(ValueData::Boolean(value), Role::Raw)
    }

    pub fn unknown() -> Self {
        Self::new(ValueData::Unknown, Role::Raw)
    }

    pub fn nil() -> Self {
        Self::new(ValueData::Nil, Role::Raw)
    }

    pub fn number(value: Number) -> Self {
        Self::new(ValueData::Number(value), Role::Raw)
    }

    pub fn integer(value: i64) -> Self {
        Self::number(Number::integer(value))
    }

    pub fn vector(items: Vec<Value>) -> Self {
        Self::new(ValueData::Vector(Arc::new(items)), Role::Raw)
    }

    pub fn quote(body: Arc<Vec<Node>>) -> Self {
        Self::new(ValueData::Quote(body), Role::Raw)
    }

    /// Build a text value: a vector of codepoints read as `TEXT`.
    pub fn text(source: &str) -> Self {
        Value::vector(
            source
                .chars()
                .map(|c| Value::integer(c as i64))
                .collect::<Vec<_>>(),
        )
        .with_role(Role::Text)
    }

    /// The text a `TEXT`-shaped vector spells, if it spells one.
    pub fn as_text(&self) -> Option<String> {
        match &self.data {
            ValueData::Vector(items) => items
                .iter()
                .map(|item| match &item.data {
                    ValueData::Number(n) => n.as_codepoint(),
                    _ => None,
                })
                .collect(),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<&Number> {
        match &self.data {
            ValueData::Number(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_vector(&self) -> Option<&Arc<Vec<Value>>> {
        match &self.data {
            ValueData::Vector(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_quote(&self) -> Option<&Arc<Vec<Node>>> {
        match &self.data {
            ValueData::Quote(body) => Some(body),
            _ => None,
        }
    }

    pub fn is_nil(&self) -> bool {
        matches!(self.data, ValueData::Nil)
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self.data, ValueData::Unknown)
    }

    /// The name used in diagnostics and by the contract lint.
    pub fn type_name(&self) -> &'static str {
        match self.data {
            ValueData::Boolean(_) => "boolean",
            ValueData::Unknown => "UNKNOWN",
            ValueData::Nil => "NIL",
            ValueData::Number(_) => "number",
            ValueData::Vector(_) => "vector",
            ValueData::Quote(_) => "quote",
        }
    }
}

/// Data Plane equality.
///
/// Two values are equal when their Data Planes agree. The Semantic Plane is
/// deliberately excluded: `"A"` and `[ 65 ]` hold the same water and compare
/// equal, and `EQ` does not become a way to smuggle a reading into a
/// computation. `docs/semantic-plane.md` states this as a normative rule and
/// `tests/semantic_plane.rs` holds it.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (&self.data, &other.data) {
            (ValueData::Boolean(a), ValueData::Boolean(b)) => a == b,
            (ValueData::Unknown, ValueData::Unknown) => true,
            (ValueData::Nil, ValueData::Nil) => true,
            (ValueData::Number(a), ValueData::Number(b)) => a == b,
            (ValueData::Vector(a), ValueData::Vector(b)) => a == b,
            (ValueData::Quote(a), ValueData::Quote(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.data, self.role) {
            (ValueData::Boolean(true), _) => f.write_str("TRUE"),
            (ValueData::Boolean(false), _) => f.write_str("FALSE"),
            (ValueData::Unknown, _) => f.write_str("UNKNOWN"),
            (ValueData::Nil, _) => f.write_str("NIL"),
            (ValueData::Number(n), _) => write!(f, "{n}"),
            (ValueData::Quote(body), _) => {
                if body.is_empty() {
                    return f.write_str("{ }");
                }
                f.write_str("{")?;
                for node in body.iter() {
                    write!(f, " {node}")?;
                }
                f.write_str(" }")
            }
            (ValueData::Vector(items), Role::Text) => {
                // A `TEXT` role is only ever set through a path that checks
                // the shape, so `as_text` succeeds; the fallback keeps the
                // renderer total rather than panicking.
                match self.as_text() {
                    Some(text) => write!(f, "\"{}\"", escape_text(&text)),
                    None => write_vector(f, items),
                }
            }
            (ValueData::Vector(items), Role::Interval) if items.len() == 2 => {
                write!(f, "{}..{}", items[0], items[1])
            }
            (ValueData::Vector(items), _) => write_vector(f, items),
        }
    }
}

fn write_vector(f: &mut fmt::Formatter<'_>, items: &[Value]) -> fmt::Result {
    if items.is_empty() {
        return f.write_str("[ ]");
    }
    f.write_str("[")?;
    for item in items {
        write!(f, " {item}")?;
    }
    f.write_str(" ]")
}

fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}
