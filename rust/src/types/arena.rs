use super::fraction::Fraction;
use super::{DenseTensor, Interpretation, Token, Value, ValueData};
use num_traits::ToPrimitive;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;

pub type NodeId = u32;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Nil,
    Boolean(bool),
    Scalar(Fraction),
    Vector {
        children: Vec<NodeId>,
    },
    Tensor {
        data: Vec<Fraction>,
        shape: Vec<usize>,
    },
    Record {
        pairs: Vec<NodeId>,
        shape: std::sync::Arc<crate::types::RecordShape>,
    },
    CodeBlock(Vec<Token>),
    ProcessHandle(u64),
    SupervisorHandle(u64),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ValueArena {
    pub nodes: Vec<NodeKind>,
    pub hints: Vec<Interpretation>,
}

impl ValueArena {
    pub fn new() -> Self {
        Self::default()
    }

    fn alloc_node(&mut self, kind: NodeKind, hint: Interpretation) -> NodeId {
        let id = self.nodes.len() as NodeId;
        self.nodes.push(kind);
        self.hints.push(hint);
        id
    }

    pub fn alloc_scalar(&mut self, fraction: Fraction, hint: Interpretation) -> NodeId {
        self.alloc_node(NodeKind::Scalar(fraction), hint)
    }

    pub fn alloc_vector(&mut self, children: Vec<NodeId>, hint: Interpretation) -> NodeId {
        self.alloc_node(NodeKind::Vector { children }, hint)
    }

    pub fn alloc_tensor(
        &mut self,
        data: Vec<Fraction>,
        shape: Vec<usize>,
        hint: Interpretation,
    ) -> NodeId {
        self.alloc_node(NodeKind::Tensor { data, shape }, hint)
    }

    pub fn alloc_record(
        &mut self,
        pairs: Vec<NodeId>,
        shape: std::sync::Arc<crate::types::RecordShape>,
        hint: Interpretation,
    ) -> NodeId {
        self.alloc_node(NodeKind::Record { pairs, shape }, hint)
    }

    pub fn alloc_string(&mut self, value: &str) -> NodeId {
        let mut children = Vec::with_capacity(value.chars().count());
        for ch in value.chars() {
            let scalar =
                self.alloc_scalar(Fraction::from(ch as u32 as i64), Interpretation::RawNumber);
            children.push(scalar);
        }
        if children.is_empty() {
            self.alloc_node(NodeKind::Nil, Interpretation::Text)
        } else {
            self.alloc_vector(children, Interpretation::Text)
        }
    }

    pub fn alloc_nil(&mut self, hint: Interpretation) -> NodeId {
        self.alloc_node(NodeKind::Nil, hint)
    }

    pub fn kind(&self, id: NodeId) -> &NodeKind {
        &self.nodes[id as usize]
    }

    pub fn hint(&self, id: NodeId) -> Interpretation {
        self.hints
            .get(id as usize)
            .copied()
            .unwrap_or(Interpretation::Unassigned)
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        match self.kind(id) {
            NodeKind::Vector { children } => children.as_slice(),
            NodeKind::Record { pairs, .. } => pairs.as_slice(),
            NodeKind::Tensor { .. }
            | NodeKind::Nil
            | NodeKind::Boolean(_)
            | NodeKind::Scalar(_)
            | NodeKind::CodeBlock(_)
            | NodeKind::ProcessHandle(_)
            | NodeKind::SupervisorHandle(_) => &[],
        }
    }
}

pub fn value_to_arena(root: &Value) -> (ValueArena, NodeId) {
    fn alloc_recursive(value: &Value, arena: &mut ValueArena) -> NodeId {
        match &value.data {
            ValueData::Nil => arena.alloc_nil(value.hint),
            ValueData::Boolean(b) => arena.alloc_node(NodeKind::Boolean(*b), value.hint),
            ValueData::Scalar(f) => arena.alloc_scalar(f.clone(), value.hint),
            ValueData::ExactScalar(er) => {
                // ExactScalar cannot be stored exactly in the arena (which uses Fraction
                // scalars). Store as best rational approximation; lossy but safe for
                // arena consumers (JSON export, display via arena).
                use num_bigint::BigInt;
                let approx = er
                    .best_rational_approximation(&BigInt::from(1_000_000_000u64))
                    .unwrap_or_else(super::fraction::Fraction::nil);
                arena.alloc_scalar(approx, value.hint)
            }
            ValueData::Vector(children) => {
                let child_ids = children
                    .iter()
                    .map(|child| alloc_recursive(child, arena))
                    .collect();
                arena.alloc_vector(child_ids, value.hint)
            }
            ValueData::Tensor { data, shape } => {
                arena.alloc_tensor(data.to_fractions(), (**shape).clone(), value.hint)
            }
            ValueData::Record { pairs, shape } => {
                let pair_ids = pairs
                    .iter()
                    .map(|pair| alloc_recursive(pair, arena))
                    .collect();
                arena.alloc_record(pair_ids, shape.clone(), value.hint)
            }
            ValueData::CodeBlock(tokens) => {
                arena.alloc_node(NodeKind::CodeBlock(tokens.clone()), value.hint)
            }
            ValueData::ProcessHandle(id) => {
                arena.alloc_node(NodeKind::ProcessHandle(*id), value.hint)
            }
            ValueData::SupervisorHandle(id) => {
                arena.alloc_node(NodeKind::SupervisorHandle(*id), value.hint)
            }
        }
    }

    let mut arena = ValueArena::new();
    let root_id = alloc_recursive(root, &mut arena);
    (arena, root_id)
}

pub fn arena_to_value(arena: &ValueArena, root: NodeId) -> Value {
    fn rebuild_recursive(arena: &ValueArena, id: NodeId) -> Value {
        match arena.kind(id) {
            NodeKind::Nil => Value {
                data: ValueData::Nil,
                hint: arena.hint(id),
                absence: None,
            },
            NodeKind::Boolean(b) => Value {
                data: ValueData::Boolean(*b),
                hint: Interpretation::TruthValue,
                absence: None,
            },
            NodeKind::Scalar(f) => Value {
                data: ValueData::Scalar(f.clone()),
                hint: match arena.hint(id) {
                    Interpretation::Timestamp => Interpretation::Timestamp,
                    _ => Interpretation::RawNumber,
                },
                absence: None,
            },
            NodeKind::Vector { children } => {
                let values = children
                    .iter()
                    .map(|child_id| rebuild_recursive(arena, *child_id))
                    .collect();
                Value {
                    data: ValueData::Vector(Arc::new(values)),
                    hint: arena.hint(id),
                    absence: None,
                }
            }
            NodeKind::Tensor { data, shape } => Value {
                data: ValueData::Tensor {
                    data: Arc::new(
                        DenseTensor::from_fractions(data.clone(), shape.clone())
                            .expect("arena tensor nodes preserve shape-compatible dense data"),
                    ),
                    shape: Arc::new(shape.clone()),
                },
                hint: arena.hint(id),
                absence: None,
            },
            NodeKind::Record { pairs, shape } => {
                let values = pairs
                    .iter()
                    .map(|pair_id| rebuild_recursive(arena, *pair_id))
                    .collect();
                Value {
                    data: ValueData::Record {
                        pairs: Arc::new(values),
                        shape: shape.clone(),
                    },
                    hint: arena.hint(id),
                    absence: None,
                }
            }
            NodeKind::CodeBlock(tokens) => Value {
                data: ValueData::CodeBlock(tokens.clone()),
                hint: arena.hint(id),
                absence: None,
            },
            NodeKind::ProcessHandle(handle_id) => Value {
                data: ValueData::ProcessHandle(*handle_id),
                hint: arena.hint(id),
                absence: None,
            },
            NodeKind::SupervisorHandle(handle_id) => Value {
                data: ValueData::SupervisorHandle(*handle_id),
                hint: arena.hint(id),
                absence: None,
            },
        }
    }

    rebuild_recursive(arena, root)
}

pub fn json_to_arena_node(arena: &mut ValueArena, json: JsonValue) -> Result<NodeId, String> {
    match json {
        JsonValue::Null => Ok(arena.alloc_nil(Interpretation::Nil)),
        JsonValue::Bool(v) => {
            Ok(arena.alloc_node(NodeKind::Boolean(v), Interpretation::TruthValue))
        }
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(arena.alloc_scalar(Fraction::from(i), Interpretation::RawNumber))
            } else if let Some(f) = n.as_f64() {
                let frac = Fraction::from_str(&f.to_string()).map_err(|e| e.to_string())?;
                Ok(arena.alloc_scalar(frac, Interpretation::RawNumber))
            } else {
                Err("unsupported json number".to_string())
            }
        }
        JsonValue::String(s) => Ok(arena.alloc_string(&s)),
        JsonValue::Array(items) => {
            if items.is_empty() {
                return Ok(arena.alloc_nil(Interpretation::Unassigned));
            }
            let children = items
                .into_iter()
                .map(|item| json_to_arena_node(arena, item))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(arena.alloc_vector(children, Interpretation::Unassigned))
        }
        JsonValue::Object(map) => {
            if map.is_empty() {
                return Ok(arena.alloc_nil(Interpretation::Unassigned));
            }
            let mut pairs = Vec::with_capacity(map.len());
            let mut index = HashMap::with_capacity(map.len());
            for (key, value) in map {
                index.insert(key.clone(), pairs.len());
                let key_id = arena.alloc_string(&key);
                let value_id = json_to_arena_node(arena, value)?;
                let pair_id =
                    arena.alloc_vector(vec![key_id, value_id], Interpretation::Unassigned);
                pairs.push(pair_id);
            }
            Ok(arena.alloc_record(
                pairs,
                crate::types::record_shape::intern_record_shape(index),
                Interpretation::Unassigned,
            ))
        }
    }
}

pub fn arena_node_to_json(arena: &ValueArena, root: NodeId) -> JsonValue {
    match arena.kind(root) {
        // The logical Unknown (U, SPEC §7.5) is a TruthValue-role NIL node;
        // it serializes to the string `"unknown"` (display-only surface,
        // SPEC §12.2) rather than JSON null.
        NodeKind::Nil if arena.hint(root) == Interpretation::TruthValue => {
            JsonValue::String("unknown".to_string())
        }
        NodeKind::Nil => JsonValue::Null,
        NodeKind::Boolean(b) => JsonValue::Bool(*b),
        NodeKind::Scalar(frac) => {
            if arena.hint(root) == Interpretation::TruthValue {
                return JsonValue::Bool(!frac.is_zero());
            }
            if frac.is_integer() {
                if let Some(int_val) = frac.to_i64() {
                    return JsonValue::Number(serde_json::Number::from(int_val));
                }
            }

            let as_f64 = frac.numerator().to_f64().zip(frac.denominator().to_f64());
            if let Some((n, d)) = as_f64 {
                if let Some(num) = serde_json::Number::from_f64(n / d) {
                    return JsonValue::Number(num);
                }
            }
            JsonValue::Null
        }
        NodeKind::Vector { children } => {
            if arena.hint(root) == Interpretation::Text {
                let mut buf = String::new();
                for child in children {
                    if let NodeKind::Scalar(codepoint) = arena.kind(*child) {
                        if let Some(n) = codepoint.to_i64() {
                            if let Some(ch) = char::from_u32(n as u32) {
                                buf.push(ch);
                            }
                        }
                    }
                }
                return JsonValue::String(buf);
            }

            let arr = children
                .iter()
                .map(|child| arena_node_to_json(arena, *child))
                .collect();
            JsonValue::Array(arr)
        }
        NodeKind::Tensor { data, shape } => tensor_to_json(arena.hint(root), data, shape),
        NodeKind::Record { pairs, .. } => {
            let mut map = serde_json::Map::new();
            for pair_id in pairs {
                let NodeKind::Vector { children } = arena.kind(*pair_id) else {
                    continue;
                };
                if children.len() != 2 {
                    continue;
                }

                let key = match arena_node_to_json(arena, children[0]) {
                    JsonValue::String(s) => s,
                    other => other.to_string(),
                };
                map.insert(key, arena_node_to_json(arena, children[1]));
            }
            JsonValue::Object(map)
        }
        NodeKind::CodeBlock(_) | NodeKind::ProcessHandle(_) | NodeKind::SupervisorHandle(_) => {
            JsonValue::Null
        }
    }
}

fn fraction_to_json(frac: &Fraction) -> JsonValue {
    if frac.is_integer() {
        if let Some(int_val) = frac.to_i64() {
            return JsonValue::Number(serde_json::Number::from(int_val));
        }
    }
    let pair = frac.numerator().to_f64().zip(frac.denominator().to_f64());
    if let Some((n, d)) = pair {
        if let Some(num) = serde_json::Number::from_f64(n / d) {
            return JsonValue::Number(num);
        }
    }
    JsonValue::Null
}

fn tensor_to_json(hint: Interpretation, data: &[Fraction], shape: &[usize]) -> JsonValue {
    if hint == Interpretation::Text && (shape.is_empty() || shape.len() == 1) {
        let mut buf = String::new();
        for codepoint in data {
            if let Some(n) = codepoint.to_i64() {
                if let Some(ch) = char::from_u32(n as u32) {
                    buf.push(ch);
                }
            }
        }
        return JsonValue::String(buf);
    }
    if shape.is_empty() || shape.len() == 1 {
        let arr = data.iter().map(fraction_to_json).collect();
        return JsonValue::Array(arr);
    }
    let outer = shape[0];
    let rest = &shape[1..];
    let stride: usize = rest.iter().product();
    if outer == 0 || stride == 0 {
        return JsonValue::Array(Vec::new());
    }
    let arr = (0..outer)
        .map(|i| tensor_to_json(hint, &data[i * stride..(i + 1) * stride], rest))
        .collect();
    JsonValue::Array(arr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_arena_string_allocation_uses_string_hint() {
        let mut arena = ValueArena::new();
        let root = arena.alloc_string("Aj");
        assert_eq!(arena.hint(root), Interpretation::Text);
        assert_eq!(arena.children(root).len(), 2);
        for child in arena.children(root) {
            assert_eq!(arena.hint(*child), Interpretation::RawNumber);
        }
    }

    #[test]
    fn value_roundtrip_through_arena_preserves_shape() {
        let value = Value::from_children(vec![
            Value::from_children(vec![Value::from_int(88)]),
            Value::from_children(vec![Value::from_int(99), Value::from_int(100)]),
            Value::nil(),
        ]);

        let (arena, root) = value_to_arena(&value);
        assert_eq!(arena.nodes.len(), arena.hints.len());

        let rebuilt = arena_to_value(&arena, root);
        assert_eq!(rebuilt, value);
    }

    #[test]
    fn json_roundtrip_through_arena_node_keeps_primitives_and_objects() {
        let json = serde_json::json!({
            "name": "Ajisai",
            "values": [1, 2, 3],
            "flag": true
        });
        let mut arena = ValueArena::new();
        let root = json_to_arena_node(&mut arena, json.clone()).expect("json to arena");
        let restored = arena_node_to_json(&arena, root);
        assert_eq!(restored, json);
    }

    #[test]
    fn nested_numeric_vectors_are_not_stringified_in_json() {
        let value = Value::from_children(vec![
            Value::from_children(vec![
                Value::from_children(vec![Value::from_int(88)]),
                Value::from_children(vec![Value::from_int(99)]),
                Value::from_children(vec![Value::from_int(100)]),
            ]),
            Value::from_children(vec![
                Value::from_children(vec![Value::from_int(50)]),
                Value::from_children(vec![Value::from_int(32)]),
                Value::from_children(vec![Value::from_int(44)]),
                Value::from_int(22),
            ]),
        ]);

        let (arena, root) = value_to_arena(&value);
        let json = arena_node_to_json(&arena, root);

        assert!(json.is_array(), "root should be array, got {json}");
        let first = json.as_array().expect("root array")[0].clone();
        assert!(first.is_array(), "nested numeric vector must stay array");
    }

    #[test]
    fn two_element_numeric_vector_stays_number_hint() {
        let value = Value::from_children(vec![Value::from_int(65), Value::from_int(66)]);
        let (arena, root) = value_to_arena(&value);
        assert_ne!(arena.hint(root), Interpretation::Text);
    }

    #[test]
    fn deeply_nested_numeric_vector_preserves_number_hint() {
        let mut inner = Value::from_children(vec![Value::from_int(65), Value::from_int(66)]);
        for _ in 0..14 {
            inner = Value::from_children(vec![inner]);
        }

        let (arena, root) = value_to_arena(&inner);
        let mut current = root;
        for depth in 0..15 {
            let children = arena.children(current);
            if depth < 14 {
                assert_eq!(children.len(), 1, "depth {depth}: expected one child");
                assert_ne!(arena.hint(current), Interpretation::Text);
                current = children[0];
            } else {
                assert_ne!(arena.hint(current), Interpretation::Text);
            }
        }
    }

    #[test]
    fn nested_mixed_numeric_vectors_stay_numeric() {
        let value = Value::from_children(vec![
            Value::from_children(vec![
                Value::from_children(vec![Value::from_int(88)]),
                Value::from_children(vec![Value::from_int(99)]),
                Value::from_children(vec![Value::from_int(100)]),
            ]),
            Value::from_children(vec![
                Value::from_children(vec![Value::from_int(50)]),
                Value::from_children(vec![Value::from_int(32)]),
                Value::from_children(vec![Value::from_int(44)]),
                Value::from_int(22),
            ]),
        ]);

        let (arena, _root) = value_to_arena(&value);
        for id in 0..arena.nodes.len() as u32 {
            if let NodeKind::Vector { children } = arena.kind(id) {
                let all_plain_ints = children
                    .iter()
                    .all(|c| matches!(arena.kind(*c), NodeKind::Scalar(f) if f.is_integer()));
                if all_plain_ints && !children.is_empty() {
                    assert_ne!(
                        arena.hint(id),
                        Interpretation::Text,
                        "node {id} incorrectly string"
                    );
                }
            }
        }
    }

    #[test]
    fn explicit_string_literal_retains_string_hint() {
        let mut arena = ValueArena::new();
        let root = arena.alloc_string("AB");
        assert_eq!(arena.hint(root), Interpretation::Text);
    }

    #[test]
    fn tensor_roundtrip_through_arena_preserves_dense_form() {
        use crate::types::Value;

        let value = Value::from_tensor(
            vec![
                Fraction::from(1),
                Fraction::from(2),
                Fraction::from(3),
                Fraction::from(4),
            ],
            vec![2, 2],
        );
        let (arena, root) = value_to_arena(&value);
        assert!(matches!(arena.kind(root), NodeKind::Tensor { .. }));
        let rebuilt = arena_to_value(&arena, root);
        assert_eq!(rebuilt, value);
        assert!(rebuilt.is_tensor());
    }

    #[test]
    fn tensor_arena_node_to_json_emits_nested_array() {
        use crate::types::Value;

        let value = Value::from_tensor(
            vec![
                Fraction::from(1),
                Fraction::from(2),
                Fraction::from(3),
                Fraction::from(4),
            ],
            vec![2, 2],
        );
        let (arena, root) = value_to_arena(&value);
        let json = arena_node_to_json(&arena, root);
        let expected = serde_json::json!([[1, 2], [3, 4]]);
        assert_eq!(json, expected);
    }
}
