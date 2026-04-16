use super::fraction::Fraction;
use super::{DisplayHint, Token, Value, ValueData};
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
            ValueData::Nil => arena.alloc_nil(DisplayHint::Auto),
            ValueData::Scalar(f) => arena.alloc_scalar(f.clone(), DisplayHint::Auto),
            ValueData::Vector(children) => {
                let child_ids = children
                    .iter()
                    .map(|child| alloc_recursive(child, arena))
                    .collect();
                arena.alloc_vector(child_ids, DisplayHint::Auto)
            }
            ValueData::Record { pairs, index } => {
                let pair_ids = pairs
                    .iter()
                    .map(|pair| alloc_recursive(pair, arena))
                    .collect();
                arena.alloc_record(pair_ids, index.clone(), DisplayHint::Auto)
            }
            ValueData::CodeBlock(tokens) => {
                arena.alloc_node(NodeKind::CodeBlock(tokens.clone()), DisplayHint::Auto)
            }
            ValueData::ProcessHandle(id) => {
                arena.alloc_node(NodeKind::ProcessHandle(*id), DisplayHint::Auto)
            }
            ValueData::SupervisorHandle(id) => {
                arena.alloc_node(NodeKind::SupervisorHandle(*id), DisplayHint::Auto)
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
            NodeKind::Nil => Value::nil(),
            NodeKind::Scalar(f) => Value::from_fraction(f.clone()),
            NodeKind::Vector { children } => {
                let values = children
                    .iter()
                    .map(|child_id| rebuild_recursive(arena, *child_id))
                    .collect();
                Value::from_children(values)
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
                }
            }
            NodeKind::CodeBlock(tokens) => Value::from_code_block(tokens.clone()),
            NodeKind::ProcessHandle(id) => Value::from_process_handle(*id),
            NodeKind::SupervisorHandle(id) => Value::from_supervisor_handle(*id),
        }
    }

    rebuild_recursive(arena, root)
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
}
