//! The operations an LLM is actually built from are graph compositions over
//! profile primitives, not primitives of their own. These tests execute those
//! compositions end to end so that "softmax/RMSNorm/masking are library code"
//! is a checked property rather than a stated intention.

use super::test_support::{
    bool_tensor, bool_type, context, declare, f32_tensor, f32_type, keeping_reduction, node, OPEN,
};
use super::*;
use std::collections::BTreeMap;

#[test]
fn stable_softmax_is_a_graph_composition_not_a_primitive() {
    let matrix = f32_type(&[2, 3]);
    let column = f32_type(&[2, 1]);
    let graph = Graph {
        schema_version: 1,
        profiles: vec!["org.ajisai.tensor/0.1".to_owned()],
        inputs: vec![declare("%input", matrix.clone())],
        nodes: softmax_chain("%input", &matrix, &column),
        outputs: vec!["%softmax".to_owned()],
        artifacts: vec![],
    };
    let inputs = BTreeMap::from([(
        "%input".to_owned(),
        // Large logits: without the REDUCE_MAX centering step these overflow.
        f32_tensor(vec![2, 3], vec![1000.0, 1001.0, 1002.0, 1.0, 2.0, 3.0]),
    )]);
    let outputs = execute_graph(&graph, &context(), &inputs, OPEN).unwrap();
    let TensorData::F32(values) = outputs["%softmax"].data() else {
        panic!("dtype changed")
    };
    for row in values.chunks_exact(3) {
        assert!((row.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert!(row.iter().all(|value| value.is_finite()));
    }
}

/// A causal attention mask is WHERE prefixed onto the same softmax chain.
/// Masked positions must come out exactly zero, and the negative infinity that
/// produces them must never become an arithmetic input.
#[test]
fn masked_softmax_composes_where_with_the_stable_softmax_chain() {
    let matrix = f32_type(&[2, 3]);
    let column = f32_type(&[2, 1]);
    let mut nodes = vec![node(
        "@mask",
        "tensor.where.v1",
        &["%keep", "%scores", "%fill"],
        declare("%masked", matrix.clone()),
    )];
    nodes.extend(softmax_chain("%masked", &matrix, &column));
    let graph = Graph {
        schema_version: 1,
        profiles: vec!["org.ajisai.tensor/0.1".to_owned()],
        inputs: vec![
            declare("%scores", matrix),
            declare("%keep", bool_type(&[2, 3])),
            declare("%fill", f32_type(&[])),
        ],
        nodes,
        outputs: vec!["%softmax".to_owned()],
        artifacts: vec![],
    };
    let inputs = BTreeMap::from([
        (
            "%scores".to_owned(),
            f32_tensor(vec![2, 3], vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        ),
        (
            "%keep".to_owned(),
            bool_tensor(vec![2, 3], vec![true, false, false, true, true, false]),
        ),
        (
            "%fill".to_owned(),
            f32_tensor(vec![], vec![f32::NEG_INFINITY]),
        ),
    ]);
    let outputs = execute_graph(&graph, &context(), &inputs, OPEN).unwrap();
    let TensorData::F32(values) = outputs["%softmax"].data() else {
        panic!("dtype changed")
    };
    assert_eq!(values[0], 1.0);
    assert_eq!(&values[1..3], &[0.0, 0.0]);
    assert_eq!(values[5], 0.0);
    assert!((values[3] + values[4] - 1.0).abs() < 1e-6);
    assert!(values.iter().all(|value| value.is_finite()));
}

/// RMSNorm is a composition over RSQRT: `x * rsqrt(mean(x^2) + epsilon)` along
/// the trailing axis.
#[test]
fn rms_normalization_is_a_graph_composition_over_rsqrt() {
    let row = f32_type(&[2, 3]);
    let column = f32_type(&[2, 1]);
    let scalar = f32_type(&[]);
    let graph = Graph {
        schema_version: 1,
        profiles: vec!["org.ajisai.tensor/0.1".to_owned()],
        inputs: vec![
            declare("%input", row.clone()),
            declare("%reciprocal_width", scalar.clone()),
            declare("%epsilon", scalar),
        ],
        nodes: vec![
            node(
                "@square",
                "tensor.mul.v1",
                &["%input", "%input"],
                declare("%squared", row.clone()),
            ),
            keeping_reduction(
                "@total",
                "tensor.reduce_sum.v1",
                "%squared",
                declare("%total", column.clone()),
                &[1],
            ),
            node(
                "@mean",
                "tensor.mul.v1",
                &["%total", "%reciprocal_width"],
                declare("%mean", column.clone()),
            ),
            node(
                "@stabilize",
                "tensor.add.v1",
                &["%mean", "%epsilon"],
                declare("%stabilized", column.clone()),
            ),
            node(
                "@scale",
                "tensor.rsqrt.v1",
                &["%stabilized"],
                declare("%scale", column),
            ),
            node(
                "@normalize",
                "tensor.mul.v1",
                &["%input", "%scale"],
                declare("%normalized", row),
            ),
        ],
        outputs: vec!["%normalized".to_owned()],
        artifacts: vec![],
    };
    let inputs = BTreeMap::from([
        (
            "%input".to_owned(),
            f32_tensor(vec![2, 3], vec![1.0, 2.0, 2.0, 3.0, 0.0, 4.0]),
        ),
        (
            "%reciprocal_width".to_owned(),
            f32_tensor(vec![], vec![1.0 / 3.0]),
        ),
        ("%epsilon".to_owned(), f32_tensor(vec![], vec![1e-12])),
    ]);
    let outputs = execute_graph(&graph, &context(), &inputs, OPEN).unwrap();
    let TensorData::F32(values) = outputs["%normalized"].data() else {
        panic!("dtype changed")
    };
    // Every row leaves the composition with unit root-mean-square.
    for row in values.chunks_exact(3) {
        let mean_square = row.iter().map(|value| value * value).sum::<f32>() / 3.0;
        assert!((mean_square - 1.0).abs() < 1e-5);
    }
}

/// REDUCE_MAX with retained axes, subtraction, EXP, REDUCE_SUM with retained
/// axes, and division — producing `%softmax` from `input`.
fn softmax_chain(input: &str, matrix: &GraphType, column: &GraphType) -> Vec<GraphNode> {
    vec![
        keeping_reduction(
            "@max",
            "tensor.reduce_max.v1",
            input,
            declare("%max", column.clone()),
            &[1],
        ),
        node(
            "@center",
            "tensor.sub.v1",
            &[input, "%max"],
            declare("%centered", matrix.clone()),
        ),
        node(
            "@exp",
            "tensor.exp.v1",
            &["%centered"],
            declare("%exp", matrix.clone()),
        ),
        keeping_reduction(
            "@sum",
            "tensor.reduce_sum.v1",
            "%exp",
            declare("%sum", column.clone()),
            &[1],
        ),
        node(
            "@divide",
            "tensor.div.v1",
            &["%exp", "%sum"],
            declare("%softmax", matrix.clone()),
        ),
    ]
}
