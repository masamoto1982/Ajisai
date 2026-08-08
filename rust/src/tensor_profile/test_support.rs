//! Shared builders for the graph execution and composition test suites.

use super::{
    DType, Graph, GraphType, GraphValidationContext, GraphValue, OperatorSemantics,
    SymbolicDimension, Tensor, TensorData, TensorMemoryBudget,
};
use std::collections::BTreeMap;

pub(super) const OPEN: TensorMemoryBudget = TensorMemoryBudget::new(usize::MAX, usize::MAX);

/// Every operator semantic ID the reference backend can execute. Tests that
/// need to reject an operator remove it rather than rebuilding the map.
pub(super) fn context() -> GraphValidationContext {
    GraphValidationContext {
        profile_id: "org.ajisai.tensor/0.1".to_owned(),
        operator_semantics: BTreeMap::from([
            ("tensor.matmul.v1".to_owned(), OperatorSemantics::Matmul),
            ("tensor.exp.v1".to_owned(), OperatorSemantics::Exp),
            ("tensor.log.v1".to_owned(), OperatorSemantics::Log),
            ("tensor.rsqrt.v1".to_owned(), OperatorSemantics::Rsqrt),
            ("tensor.where.v1".to_owned(), OperatorSemantics::Where),
            (
                "tensor.reduce_sum.v1".to_owned(),
                OperatorSemantics::ReduceSum,
            ),
            (
                "tensor.reduce_max.v1".to_owned(),
                OperatorSemantics::ReduceMax,
            ),
            ("tensor.add.v1".to_owned(), OperatorSemantics::Add),
            ("tensor.sub.v1".to_owned(), OperatorSemantics::Sub),
            ("tensor.mul.v1".to_owned(), OperatorSemantics::Mul),
            ("tensor.div.v1".to_owned(), OperatorSemantics::Div),
        ]),
    }
}

/// The committed exchange-format example, so the tests exercise the same graph
/// the specification ships.
pub(super) fn example() -> Graph {
    serde_json::from_str(include_str!(
        "../../../spec/examples/tiny-matmul.graph.json"
    ))
    .unwrap()
}

pub(super) fn f32_tensor(shape: Vec<usize>, values: Vec<f32>) -> Tensor {
    Tensor::new(shape, TensorData::F32(values), OPEN).unwrap()
}

pub(super) fn bool_tensor(shape: Vec<usize>, values: Vec<bool>) -> Tensor {
    Tensor::new(shape, TensorData::Bool(values), OPEN).unwrap()
}

pub(super) fn f32_type(shape: &[usize]) -> GraphType {
    typed(DType::F32, shape)
}

pub(super) fn bool_type(shape: &[usize]) -> GraphType {
    typed(DType::Bool, shape)
}

fn typed(dtype: DType, shape: &[usize]) -> GraphType {
    GraphType::Tensor {
        dtype,
        shape: shape
            .iter()
            .copied()
            .map(SymbolicDimension::Known)
            .collect(),
    }
}

pub(super) fn declare(id: &str, value_type: GraphType) -> GraphValue {
    GraphValue {
        id: id.to_owned(),
        value_type,
    }
}

/// A node with no attributes, which covers every operator except the
/// reductions.
pub(super) fn node(
    id: &str,
    operator: &str,
    inputs: &[&str],
    output: GraphValue,
) -> super::GraphNode {
    super::GraphNode {
        id: id.to_owned(),
        operator_semantic_id: operator.to_owned(),
        inputs: inputs.iter().copied().map(str::to_owned).collect(),
        outputs: vec![output],
        attributes: BTreeMap::new(),
    }
}

/// A reduction over `axes` that retains the reduced axes, the form the softmax
/// and normalization compositions rely on for broadcasting.
pub(super) fn keeping_reduction(
    id: &str,
    operator: &str,
    input: &str,
    output: GraphValue,
    axes: &[usize],
) -> super::GraphNode {
    super::GraphNode {
        id: id.to_owned(),
        operator_semantic_id: operator.to_owned(),
        inputs: vec![input.to_owned()],
        outputs: vec![output],
        attributes: BTreeMap::from([
            ("axes".to_owned(), serde_json::json!(axes)),
            ("keepDimensions".to_owned(), serde_json::json!(true)),
        ]),
    }
}
