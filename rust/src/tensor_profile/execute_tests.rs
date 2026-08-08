use super::test_support::{bool_tensor, context, declare, example, f32_tensor, f32_type, OPEN};
use super::*;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn committed_graph_executes_end_to_end() {
    let inputs = BTreeMap::from([
        (
            "%left".to_owned(),
            f32_tensor(vec![2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        ),
        (
            "%right".to_owned(),
            f32_tensor(
                vec![3, 4],
                vec![
                    1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
                ],
            ),
        ),
    ]);
    let outputs = execute_graph(&example(), &context(), &inputs, OPEN).unwrap();
    assert_eq!(
        outputs.keys().cloned().collect::<BTreeSet<_>>(),
        BTreeSet::from(["%logits".to_owned()])
    );
    assert_eq!(
        outputs["%logits"].data(),
        &TensorData::F32(vec![38.0, 44.0, 50.0, 56.0, 83.0, 98.0, 113.0, 128.0])
    );
}

#[test]
fn execution_rejects_missing_input() {
    let error = execute_graph(&example(), &context(), &BTreeMap::new(), OPEN).unwrap_err();
    assert_eq!(error, GraphExecutionError::MissingInput("%left".to_owned()));
}

#[test]
fn execution_checks_declared_input_dimensions() {
    let inputs = BTreeMap::from([
        ("%left".to_owned(), f32_tensor(vec![1, 3], vec![0.0; 3])),
        ("%right".to_owned(), f32_tensor(vec![3, 4], vec![0.0; 12])),
    ]);
    assert_eq!(
        execute_graph(&example(), &context(), &inputs, OPEN),
        Err(GraphExecutionError::InputDimensionMismatch {
            id: "%left".to_owned(),
            axis: 0,
            expected: 2,
            actual: 1,
        })
    );
}

#[test]
fn execution_unifies_symbolic_dimensions_across_inputs() {
    let mut graph = example();
    graph.inputs[0].value_type = GraphType::Tensor {
        dtype: DType::F32,
        shape: vec![
            SymbolicDimension::Symbol("M".to_owned()),
            SymbolicDimension::Symbol("K".to_owned()),
        ],
    };
    graph.inputs[1].value_type = GraphType::Tensor {
        dtype: DType::F32,
        shape: vec![
            SymbolicDimension::Symbol("K".to_owned()),
            SymbolicDimension::Symbol("N".to_owned()),
        ],
    };
    graph.nodes[0].outputs[0].value_type = GraphType::Tensor {
        dtype: DType::F32,
        shape: vec![
            SymbolicDimension::Symbol("M".to_owned()),
            SymbolicDimension::Symbol("N".to_owned()),
        ],
    };
    let inputs = BTreeMap::from([
        ("%left".to_owned(), f32_tensor(vec![2, 3], vec![0.0; 6])),
        ("%right".to_owned(), f32_tensor(vec![3, 4], vec![0.0; 12])),
    ]);
    let outputs = execute_graph(&graph, &context(), &inputs, OPEN).unwrap();
    assert_eq!(outputs["%logits"].shape().dimensions(), &[2, 4]);
}

#[test]
fn execution_chains_matmul_exp_and_log_as_ssa_nodes() {
    let mut graph = example();
    graph.nodes[0].outputs[0].id = "%matmul".to_owned();
    let result_type = graph.nodes[0].outputs[0].value_type.clone();
    graph.nodes.extend([
        GraphNode {
            id: "@exp".to_owned(),
            operator_semantic_id: "tensor.exp.v1".to_owned(),
            inputs: vec!["%matmul".to_owned()],
            outputs: vec![GraphValue {
                id: "%exp".to_owned(),
                value_type: result_type.clone(),
            }],
            attributes: BTreeMap::new(),
        },
        GraphNode {
            id: "@log".to_owned(),
            operator_semantic_id: "tensor.log.v1".to_owned(),
            inputs: vec!["%exp".to_owned()],
            outputs: vec![GraphValue {
                id: "%logits".to_owned(),
                value_type: result_type,
            }],
            attributes: BTreeMap::new(),
        },
    ]);
    let inputs = BTreeMap::from([
        ("%left".to_owned(), f32_tensor(vec![2, 3], vec![0.0; 6])),
        ("%right".to_owned(), f32_tensor(vec![3, 4], vec![0.0; 12])),
    ]);
    let outputs = execute_graph(&graph, &context(), &inputs, OPEN).unwrap();
    assert_eq!(outputs["%logits"].data(), &TensorData::F32(vec![0.0; 8]));
}

#[test]
fn execution_reduces_a_graph_axis() {
    let mut graph = example();
    graph.nodes = vec![GraphNode {
        id: "@sum".to_owned(),
        operator_semantic_id: "tensor.reduce_sum.v1".to_owned(),
        inputs: vec!["%left".to_owned()],
        outputs: vec![GraphValue {
            id: "%sum".to_owned(),
            value_type: GraphType::Tensor {
                dtype: DType::F32,
                shape: vec![SymbolicDimension::Known(2)],
            },
        }],
        attributes: BTreeMap::from([("axes".to_owned(), serde_json::json!([1]))]),
    }];
    graph.outputs = vec!["%sum".to_owned()];
    let inputs = BTreeMap::from([
        (
            "%left".to_owned(),
            f32_tensor(vec![2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        ),
        ("%right".to_owned(), f32_tensor(vec![3, 4], vec![0.0; 12])),
    ]);
    let outputs = execute_graph(&graph, &context(), &inputs, OPEN).unwrap();
    assert_eq!(outputs["%sum"].data(), &TensorData::F32(vec![6.0, 15.0]));
}

#[test]
fn execution_reduces_max_over_a_graph_axis() {
    let mut graph = example();
    graph.nodes = vec![GraphNode {
        id: "@max".to_owned(),
        operator_semantic_id: "tensor.reduce_max.v1".to_owned(),
        inputs: vec!["%left".to_owned()],
        outputs: vec![GraphValue {
            id: "%max".to_owned(),
            value_type: GraphType::Tensor {
                dtype: DType::F32,
                shape: vec![SymbolicDimension::Known(2)],
            },
        }],
        attributes: BTreeMap::from([("axes".to_owned(), serde_json::json!([1]))]),
    }];
    graph.outputs = vec!["%max".to_owned()];
    let inputs = BTreeMap::from([
        (
            "%left".to_owned(),
            f32_tensor(vec![2, 3], vec![1.0, 5.0, 3.0, 4.0, 2.0, 6.0]),
        ),
        ("%right".to_owned(), f32_tensor(vec![3, 4], vec![0.0; 12])),
    ]);
    let outputs = execute_graph(&graph, &context(), &inputs, OPEN).unwrap();
    assert_eq!(outputs["%max"].data(), &TensorData::F32(vec![5.0, 6.0]));
}

/// RMSNorm is a library composition over RSQRT, not a profile primitive:
/// `x * rsqrt(mean(x^2) + epsilon)` over the trailing axis.
#[test]
fn execution_rejects_a_runtime_predicate_that_is_not_bool() {
    let graph = Graph {
        schema_version: 1,
        profiles: vec!["org.ajisai.tensor/0.1".to_owned()],
        inputs: vec![
            declare(
                "%keep",
                GraphType::Tensor {
                    dtype: DType::Bool,
                    shape: vec![SymbolicDimension::Known(2)],
                },
            ),
            declare("%on_true", f32_type(&[2])),
            declare("%on_false", f32_type(&[2])),
        ],
        nodes: vec![GraphNode {
            id: "@select".to_owned(),
            operator_semantic_id: "tensor.where.v1".to_owned(),
            inputs: vec![
                "%keep".to_owned(),
                "%on_true".to_owned(),
                "%on_false".to_owned(),
            ],
            outputs: vec![declare("%selected", f32_type(&[2]))],
            attributes: BTreeMap::new(),
        }],
        outputs: vec!["%selected".to_owned()],
        artifacts: vec![],
    };
    let mut inputs = BTreeMap::from([
        ("%keep".to_owned(), bool_tensor(vec![2], vec![true, false])),
        ("%on_true".to_owned(), f32_tensor(vec![2], vec![1.0, 2.0])),
        ("%on_false".to_owned(), f32_tensor(vec![2], vec![3.0, 4.0])),
    ]);
    let outputs = execute_graph(&graph, &context(), &inputs, OPEN).unwrap();
    assert_eq!(
        outputs["%selected"].data(),
        &TensorData::F32(vec![1.0, 4.0])
    );

    // A numeric tensor is not silently accepted where the graph declared a
    // predicate: there is no implicit cast in either direction.
    inputs.insert("%keep".to_owned(), f32_tensor(vec![2], vec![1.0, 0.0]));
    assert_eq!(
        execute_graph(&graph, &context(), &inputs, OPEN),
        Err(GraphExecutionError::InputDTypeMismatch("%keep".to_owned()))
    );
}
