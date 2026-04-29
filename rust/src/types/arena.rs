use super::fraction::Fraction;
use super::{DisplayHint, Token, Value, ValueData};
use num_traits::ToPrimitive;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::rc::Rc;

pub type NodeId = u32;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Nil,
    Scalar(Fraction),
    Vector {
        children: Vec<NodeId>,
    },
    Record {
        pairs: Vec<NodeId>,
        index: HashMap<String, usize>,
    },
    CodeBlock(Vec<Token>),
    ProcessHandle(u64),
    SupervisorHandle(u64),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ValueArena {
    pub nodes: Vec<NodeKind>,
    pub hints: Vec<DisplayHint>,
}

impl ValueArena {
    pub fn new() -> Self {
        Self::default()
    }

    fn alloc_node(&mut self, kind: NodeKind, hint: DisplayHint) -> NodeId {
        let id = self.nodes.len() as NodeId;
        self.nodes.push(kind);
        self.hints.push(hint);
        id
    }

    pub fn alloc_scalar(&mut self, fraction: Fraction, hint: DisplayHint) -> NodeId {
        self.alloc_node(NodeKind::Scalar(fraction), hint)
    }

    pub fn alloc_vector(&mut self, children: Vec<NodeId>, hint: DisplayHint) -> NodeId {
        self.alloc_node(NodeKind::Vector { children }, hint)
    }

    pub fn alloc_record(
        &mut self,
        pairs: Vec<NodeId>,
        index: HashMap<String, usize>,
        hint: DisplayHint,
    ) -> NodeId {
        self.alloc_node(NodeKind::Record { pairs, index }, hint)
    }

    pub fn alloc_string(&mut self, value: &str) -> NodeId {
        let mut children = Vec::with_capacity(value.chars().count());
        for ch in value.chars() {
            let scalar = self.alloc_scalar(Fraction::from(ch as u32 as i64), DisplayHint::Number);
            children.push(scalar);
        }
        if children.is_empty() {
            self.alloc_node(NodeKind::Nil, DisplayHint::String)
        } else {
            self.alloc_vector(children, DisplayHint::String)
        }
    }

    pub fn alloc_nil(&mut self, hint: DisplayHint) -> NodeId {
        self.alloc_node(NodeKind::Nil, hint)
    }

    pub fn kind(&self, id: NodeId) -> &NodeKind {
        &self.nodes[id as usize]
    }

    pub fn hint(&self, id: NodeId) -> DisplayHint {
        self.hints
            .get(id as usize)
            .copied()
            .unwrap_or(DisplayHint::Auto)
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        match self.kind(id) {
            NodeKind::Vector { children } => children.as_slice(),
            NodeKind::Record { pairs, .. } => pairs.as_slice(),
            _ => &[],
        }
    }
}

pub fn value_to_arena(root: &Value) -> (ValueArena, NodeId) {
    fn alloc_recursive(value: &Value, arena: &mut ValueArena) -> NodeId {
        match &value.data {
            ValueData::Nil => arena.alloc_nil(value.hint),
            ValueData::Scalar(f) => arena.alloc_scalar(f.clone(), value.hint),
            ValueData::Vector(children) => {
                let child_ids = children
                    .iter()
                    .map(|child| alloc_recursive(child, arena))
                    .collect();
                arena.alloc_vector(child_ids, value.hint)
            }
            ValueData::Record { pairs, index } => {
                let pair_ids = pairs
                    .iter()
                    .map(|pair| alloc_recursive(pair, arena))
                    .collect();
                arena.alloc_record(pair_ids, index.clone(), value.hint)
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
                nil_reason: None,
            },
            NodeKind::Scalar(f) => Value {
                data: ValueData::Scalar(f.clone()),
                hint: match arena.hint(id) {
                    DisplayHint::DateTime => DisplayHint::DateTime,
                    _ => DisplayHint::Number,
                },
                nil_reason: None,
            },
            NodeKind::Vector { children } => {
                let values = children
                    .iter()
                    .map(|child_id| rebuild_recursive(arena, *child_id))
                    .collect();
                Value {
                    data: ValueData::Vector(Rc::new(values)),
                    hint: arena.hint(id),
                    nil_reason: None,
                }
            }
            NodeKind::Record { pairs, index } => {
                let values = pairs
                    .iter()
                    .map(|pair_id| rebuild_recursive(arena, *pair_id))
                    .collect();
                Value {
                    data: ValueData::Record {
                        pairs: Rc::new(values),
                        index: index.clone(),
                    },
                    hint: arena.hint(id),
                    nil_reason: None,
                }
            }
            NodeKind::CodeBlock(tokens) => Value {
                data: ValueData::CodeBlock(tokens.clone()),
                hint: arena.hint(id),
                nil_reason: None,
            },
            NodeKind::ProcessHandle(handle_id) => Value {
                data: ValueData::ProcessHandle(*handle_id),
                hint: arena.hint(id),
                nil_reason: None,
            },
            NodeKind::SupervisorHandle(handle_id) => Value {
                data: ValueData::SupervisorHandle(*handle_id),
                hint: arena.hint(id),
                nil_reason: None,
            },
        }
    }

    rebuild_recursive(arena, root)
}

pub fn json_to_arena_node(arena: &mut ValueArena, json: JsonValue) -> Result<NodeId, String> {
    match json {
        JsonValue::Null => Ok(arena.alloc_nil(DisplayHint::Nil)),
        JsonValue::Bool(v) => Ok(arena.alloc_scalar(
            Fraction::from(if v { 1_i64 } else { 0_i64 }),
            DisplayHint::Boolean,
        )),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(arena.alloc_scalar(Fraction::from(i), DisplayHint::Number))
            } else if let Some(f) = n.as_f64() {
                let frac = Fraction::from_str(&f.to_string()).map_err(|e| e.to_string())?;
                Ok(arena.alloc_scalar(frac, DisplayHint::Number))
            } else {
                Err("unsupported json number".to_string())
            }
        }
        JsonValue::String(s) => Ok(arena.alloc_string(&s)),
        JsonValue::Array(items) => {
            if items.is_empty() {
                return Ok(arena.alloc_nil(DisplayHint::Auto));
            }
            let children = items
                .into_iter()
                .map(|item| json_to_arena_node(arena, item))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(arena.alloc_vector(children, DisplayHint::Auto))
        }
        JsonValue::Object(map) => {
            if map.is_empty() {
                return Ok(arena.alloc_nil(DisplayHint::Auto));
            }
            let mut pairs = Vec::with_capacity(map.len());
            let mut index = HashMap::with_capacity(map.len());
            for (key, value) in map {
                index.insert(key.clone(), pairs.len());
                let key_id = arena.alloc_string(&key);
                let value_id = json_to_arena_node(arena, value)?;
                let pair_id = arena.alloc_vector(vec![key_id, value_id], DisplayHint::Auto);
                pairs.push(pair_id);
            }
            Ok(arena.alloc_record(pairs, index, DisplayHint::Auto))
        }
    }
}

pub fn arena_node_to_json(arena: &ValueArena, root: NodeId) -> JsonValue {
    match arena.kind(root) {
        NodeKind::Nil => JsonValue::Null,
        NodeKind::Scalar(frac) => {
            if arena.hint(root) == DisplayHint::Boolean {
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
            if arena.hint(root) == DisplayHint::String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_arena_string_allocation_uses_string_hint() {
        let mut arena = ValueArena::new();
        let root = arena.alloc_string("Aj");
        assert_eq!(arena.hint(root), DisplayHint::String);
        assert_eq!(arena.children(root).len(), 2);
        for child in arena.children(root) {
            assert_eq!(arena.hint(*child), DisplayHint::Number);
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
        assert_ne!(arena.hint(root), DisplayHint::String);
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
                assert_ne!(arena.hint(current), DisplayHint::String);
                current = children[0];
            } else {
                assert_ne!(arena.hint(current), DisplayHint::String);
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
                let all_plain_ints = children.iter().all(|c| {
                    matches!(arena.kind(*c), NodeKind::Scalar(f) if f.is_integer())
                });
                if all_plain_ints && !children.is_empty() {
                    assert_ne!(arena.hint(id), DisplayHint::String, "node {id} incorrectly string");
                }
            }
        }
    }

    #[test]
    fn explicit_string_literal_retains_string_hint() {
        let mut arena = ValueArena::new();
        let root = arena.alloc_string("AB");
        assert_eq!(arena.hint(root), DisplayHint::String);
    }
}
