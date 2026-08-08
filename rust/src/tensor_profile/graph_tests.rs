use super::*;
use std::collections::BTreeMap;

fn tensor(id: &str, shape: Vec<SymbolicDimension>) -> GraphValue {
    GraphValue {
        id: id.to_owned(),
        value_type: GraphType::Tensor {
            dtype: DType::F32,
            shape,
        },
    }
}

fn context() -> GraphValidationContext {
    GraphValidationContext {
        profile_id: "org.ajisai.tensor/0.1".to_owned(),
        operator_semantics: BTreeMap::from([(
            "tensor.matmul.v1".to_owned(),
            OperatorSemantics::Matmul,
        )]),
    }
}

fn valid_graph() -> Graph {
    Graph {
        schema_version: 1,
        profiles: vec!["org.ajisai.tensor/0.1".to_owned()],
        inputs: vec![
            tensor(
                "%left",
                vec![SymbolicDimension::Known(2), SymbolicDimension::Known(3)],
            ),
            tensor(
                "%right",
                vec![SymbolicDimension::Known(3), SymbolicDimension::Known(4)],
            ),
        ],
        nodes: vec![GraphNode {
            id: "@multiply".to_owned(),
            operator_semantic_id: "tensor.matmul.v1".to_owned(),
            inputs: vec!["%left".to_owned(), "%right".to_owned()],
            outputs: vec![tensor(
                "%result",
                vec![SymbolicDimension::Known(2), SymbolicDimension::Known(4)],
            )],
            attributes: BTreeMap::new(),
        }],
        outputs: vec!["%result".to_owned()],
        artifacts: vec![],
    }
}

#[test]
fn valid_graph_round_trips_through_the_exchange_format() {
    let graph = valid_graph();
    graph.validate(&context()).unwrap();
    let json = serde_json::to_string(&graph).unwrap();
    assert_eq!(serde_json::from_str::<Graph>(&json).unwrap(), graph);
}

#[test]
fn use_before_definition_is_rejected() {
    let mut graph = valid_graph();
    graph.nodes[0].inputs[0] = "%future".to_owned();
    assert_eq!(
        graph.validate(&context()),
        Err(GraphValidationError::UndefinedValue("%future".to_owned()))
    );
}

#[test]
fn matmul_rejects_wrong_contraction_dimension() {
    let mut graph = valid_graph();
    graph.inputs[1] = tensor(
        "%right",
        vec![SymbolicDimension::Known(5), SymbolicDimension::Known(4)],
    );
    assert!(matches!(
        graph.validate(&context()),
        Err(GraphValidationError::ShapeMismatch(_))
    ));
}

#[test]
fn matmul_rejects_a_false_declared_output_shape() {
    let mut graph = valid_graph();
    graph.nodes[0].outputs[0] = tensor(
        "%result",
        vec![SymbolicDimension::Known(2), SymbolicDimension::Known(5)],
    );
    assert_eq!(
        graph.validate(&context()),
        Err(GraphValidationError::OutputTypeMismatch(
            "@multiply".to_owned()
        ))
    );
}

#[test]
fn matmul_broadcasts_leading_batch_dimensions() {
    let mut graph = valid_graph();
    graph.inputs[0] = tensor(
        "%left",
        vec![
            SymbolicDimension::Known(7),
            SymbolicDimension::Known(1),
            SymbolicDimension::Known(2),
            SymbolicDimension::Known(3),
        ],
    );
    graph.inputs[1] = tensor(
        "%right",
        vec![
            SymbolicDimension::Known(5),
            SymbolicDimension::Known(3),
            SymbolicDimension::Known(4),
        ],
    );
    graph.nodes[0].outputs[0] = tensor(
        "%result",
        vec![
            SymbolicDimension::Known(7),
            SymbolicDimension::Known(5),
            SymbolicDimension::Known(2),
            SymbolicDimension::Known(4),
        ],
    );
    graph.validate(&context()).unwrap();
}

#[test]
fn identity_is_stable_and_changes_with_semantics() {
    let graph = valid_graph();
    assert_eq!(
        graph.semantic_identity().unwrap(),
        graph.semantic_identity().unwrap()
    );
    let mut changed = graph.clone();
    changed.nodes[0].operator_semantic_id = "tensor.matmul.v2".to_owned();
    assert_ne!(
        graph.semantic_identity().unwrap(),
        changed.semantic_identity().unwrap()
    );
}

#[test]
fn duplicate_value_ids_are_rejected() {
    let mut graph = valid_graph();
    graph.nodes[0].outputs[0].id = "%left".to_owned();
    assert_eq!(
        graph.validate(&context()),
        Err(GraphValidationError::DuplicateIdentifier(
            "%left".to_owned()
        ))
    );
}

#[test]
fn committed_example_satisfies_runtime_matmul_contract() {
    let graph: Graph = serde_json::from_str(include_str!(
        "../../../spec/examples/tiny-matmul.graph.json"
    ))
    .unwrap();
    graph.validate(&context()).unwrap();
}

#[test]
fn unary_numeric_operator_must_preserve_declared_type() {
    let mut graph = valid_graph();
    graph.nodes = vec![GraphNode {
        id: "@exp".to_owned(),
        operator_semantic_id: "tensor.exp.v1".to_owned(),
        inputs: vec!["%left".to_owned()],
        outputs: vec![tensor(
            "%result",
            vec![SymbolicDimension::Known(2), SymbolicDimension::Known(4)],
        )],
        attributes: BTreeMap::new(),
    }];
    let mut context = context();
    context
        .operator_semantics
        .insert("tensor.exp.v1".to_owned(), OperatorSemantics::Exp);
    assert_eq!(
        graph.validate(&context),
        Err(GraphValidationError::OutputTypeMismatch("@exp".to_owned()))
    );
}

#[test]
fn where_infers_the_three_way_broadcast_of_predicate_and_branches() {
    let mut graph = valid_graph();
    graph.inputs.push(GraphValue {
        id: "%mask".to_owned(),
        value_type: GraphType::Tensor {
            dtype: DType::Bool,
            shape: vec![SymbolicDimension::Known(2), SymbolicDimension::Known(1)],
        },
    });
    graph.nodes = vec![GraphNode {
        id: "@select".to_owned(),
        operator_semantic_id: "tensor.where.v1".to_owned(),
        inputs: vec!["%mask".to_owned(), "%left".to_owned(), "%left".to_owned()],
        outputs: vec![tensor(
            "%result",
            vec![SymbolicDimension::Known(2), SymbolicDimension::Known(3)],
        )],
        attributes: BTreeMap::new(),
    }];
    let mut context = context();
    context
        .operator_semantics
        .insert("tensor.where.v1".to_owned(), OperatorSemantics::Where);
    graph.validate(&context).unwrap();
}

#[test]
fn where_requires_a_bool_predicate_not_a_numeric_one() {
    let mut graph = valid_graph();
    graph.nodes = vec![GraphNode {
        id: "@select".to_owned(),
        operator_semantic_id: "tensor.where.v1".to_owned(),
        inputs: vec!["%left".to_owned(), "%left".to_owned(), "%left".to_owned()],
        outputs: vec![tensor(
            "%result",
            vec![SymbolicDimension::Known(2), SymbolicDimension::Known(3)],
        )],
        attributes: BTreeMap::new(),
    }];
    let mut context = context();
    context
        .operator_semantics
        .insert("tensor.where.v1".to_owned(), OperatorSemantics::Where);
    assert_eq!(
        graph.validate(&context),
        Err(GraphValidationError::PredicateDTypeMismatch(
            "@select".to_owned()
        ))
    );
}

#[test]
fn arithmetic_operators_refuse_the_predicate_dtype_at_validation_time() {
    let mut graph = valid_graph();
    graph.inputs[0].value_type = GraphType::Tensor {
        dtype: DType::Bool,
        shape: vec![SymbolicDimension::Known(2), SymbolicDimension::Known(3)],
    };
    graph.nodes = vec![GraphNode {
        id: "@rsqrt".to_owned(),
        operator_semantic_id: "tensor.rsqrt.v1".to_owned(),
        inputs: vec!["%left".to_owned()],
        outputs: vec![GraphValue {
            id: "%result".to_owned(),
            value_type: graph.inputs[0].value_type.clone(),
        }],
        attributes: BTreeMap::new(),
    }];
    let mut context = context();
    context
        .operator_semantics
        .insert("tensor.rsqrt.v1".to_owned(), OperatorSemantics::Rsqrt);
    assert_eq!(
        graph.validate(&context),
        Err(GraphValidationError::NonNumericDType("@rsqrt".to_owned()))
    );
}

#[test]
fn reduce_sum_infers_removed_axes_from_attributes() {
    let mut graph = valid_graph();
    graph.nodes = vec![GraphNode {
        id: "@sum".to_owned(),
        operator_semantic_id: "tensor.reduce_sum.v1".to_owned(),
        inputs: vec!["%left".to_owned()],
        outputs: vec![tensor("%result", vec![SymbolicDimension::Known(2)])],
        attributes: BTreeMap::from([("axes".to_owned(), serde_json::json!([1]))]),
    }];
    let mut context = context();
    context.operator_semantics.insert(
        "tensor.reduce_sum.v1".to_owned(),
        OperatorSemantics::ReduceSum,
    );
    graph.validate(&context).unwrap();
}
