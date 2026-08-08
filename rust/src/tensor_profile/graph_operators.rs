use super::graph::{
    GraphNode, GraphType, GraphValidationError, OperatorSemantics, SymbolicDimension,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn validate_operator(
    node: &GraphNode,
    semantics: OperatorSemantics,
    values: &BTreeMap<String, GraphType>,
) -> Result<(), GraphValidationError> {
    match semantics {
        OperatorSemantics::Matmul => validate_matmul(node, values),
        OperatorSemantics::Exp | OperatorSemantics::Log => {
            validate_shape_preserving_unary(node, values)
        }
        OperatorSemantics::ReduceSum | OperatorSemantics::ReduceMax => {
            validate_reduction(node, values)
        }
        OperatorSemantics::Add
        | OperatorSemantics::Sub
        | OperatorSemantics::Mul
        | OperatorSemantics::Div => validate_elementwise_binary(node, values),
    }
}

fn validate_elementwise_binary(
    node: &GraphNode,
    values: &BTreeMap<String, GraphType>,
) -> Result<(), GraphValidationError> {
    if node.inputs.len() != 2 || node.outputs.len() != 1 {
        return Err(GraphValidationError::OperatorArity {
            operator: node.operator_semantic_id.clone(),
            expected_inputs: 2,
            actual_inputs: node.inputs.len(),
            expected_outputs: 1,
            actual_outputs: node.outputs.len(),
        });
    }
    let GraphType::Tensor {
        dtype: left_dtype,
        shape: left,
    } = &values[&node.inputs[0]];
    let GraphType::Tensor {
        dtype: right_dtype,
        shape: right,
    } = &values[&node.inputs[1]];
    if left_dtype != right_dtype {
        return Err(GraphValidationError::DTypeMismatch(node.id.clone()));
    }
    let output_shape = broadcast_dimensions(left, right)?;
    let inferred = GraphType::Tensor {
        dtype: *left_dtype,
        shape: output_shape,
    };
    if node.outputs[0].value_type != inferred {
        return Err(GraphValidationError::OutputTypeMismatch(node.id.clone()));
    }
    Ok(())
}

fn validate_reduction(
    node: &GraphNode,
    values: &BTreeMap<String, GraphType>,
) -> Result<(), GraphValidationError> {
    if node.inputs.len() != 1 || node.outputs.len() != 1 {
        return Err(GraphValidationError::OperatorArity {
            operator: node.operator_semantic_id.clone(),
            expected_inputs: 1,
            actual_inputs: node.inputs.len(),
            expected_outputs: 1,
            actual_outputs: node.outputs.len(),
        });
    }
    let (axes, keep_dimensions) = reduction_attributes(node)?;
    let GraphType::Tensor { dtype, shape } = &values[&node.inputs[0]];
    let mut seen = BTreeSet::new();
    for axis in axes {
        if axis >= shape.len() || !seen.insert(axis) {
            return Err(GraphValidationError::InvalidAttribute {
                node: node.id.clone(),
                attribute: "axes".to_owned(),
            });
        }
    }
    let output_shape = shape
        .iter()
        .enumerate()
        .filter_map(|(axis, dimension)| {
            if seen.contains(&axis) {
                keep_dimensions.then_some(SymbolicDimension::Known(1))
            } else {
                Some(dimension.clone())
            }
        })
        .collect();
    let inferred = GraphType::Tensor {
        dtype: *dtype,
        shape: output_shape,
    };
    if node.outputs[0].value_type != inferred {
        return Err(GraphValidationError::OutputTypeMismatch(node.id.clone()));
    }
    Ok(())
}

pub(crate) fn reduction_attributes(
    node: &GraphNode,
) -> Result<(Vec<usize>, bool), GraphValidationError> {
    let axes = node
        .attributes
        .get("axes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| GraphValidationError::InvalidAttribute {
            node: node.id.clone(),
            attribute: "axes".to_owned(),
        })?
        .iter()
        .map(|axis| axis.as_u64().and_then(|axis| usize::try_from(axis).ok()))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| GraphValidationError::InvalidAttribute {
            node: node.id.clone(),
            attribute: "axes".to_owned(),
        })?;
    let keep_dimensions = match node.attributes.get("keepDimensions") {
        Some(value) => value
            .as_bool()
            .ok_or_else(|| GraphValidationError::InvalidAttribute {
                node: node.id.clone(),
                attribute: "keepDimensions".to_owned(),
            })?,
        None => false,
    };
    Ok((axes, keep_dimensions))
}

fn validate_shape_preserving_unary(
    node: &GraphNode,
    values: &BTreeMap<String, GraphType>,
) -> Result<(), GraphValidationError> {
    if node.inputs.len() != 1 || node.outputs.len() != 1 {
        return Err(GraphValidationError::OperatorArity {
            operator: node.operator_semantic_id.clone(),
            expected_inputs: 1,
            actual_inputs: node.inputs.len(),
            expected_outputs: 1,
            actual_outputs: node.outputs.len(),
        });
    }
    if node.outputs[0].value_type != values[&node.inputs[0]] {
        return Err(GraphValidationError::OutputTypeMismatch(node.id.clone()));
    }
    Ok(())
}

fn validate_matmul(
    node: &GraphNode,
    values: &BTreeMap<String, GraphType>,
) -> Result<(), GraphValidationError> {
    if node.inputs.len() != 2 || node.outputs.len() != 1 {
        return Err(GraphValidationError::OperatorArity {
            operator: node.operator_semantic_id.clone(),
            expected_inputs: 2,
            actual_inputs: node.inputs.len(),
            expected_outputs: 1,
            actual_outputs: node.outputs.len(),
        });
    }
    let GraphType::Tensor {
        dtype: left_dtype,
        shape: left,
    } = &values[&node.inputs[0]];
    let GraphType::Tensor {
        dtype: right_dtype,
        shape: right,
    } = &values[&node.inputs[1]];
    if left_dtype != right_dtype {
        return Err(GraphValidationError::DTypeMismatch(node.id.clone()));
    }
    if left.len() < 2 || right.len() < 2 {
        return Err(GraphValidationError::ShapeMismatch(format!(
            "{} requires rank >= 2",
            node.id
        )));
    }
    if left[left.len() - 1] != right[right.len() - 2] {
        return Err(GraphValidationError::ShapeMismatch(format!(
            "{} contraction dimensions differ",
            node.id
        )));
    }
    let batch = broadcast_dimensions(&left[..left.len() - 2], &right[..right.len() - 2])?;
    let mut output_shape = batch;
    output_shape.push(left[left.len() - 2].clone());
    output_shape.push(right[right.len() - 1].clone());
    let inferred = GraphType::Tensor {
        dtype: *left_dtype,
        shape: output_shape,
    };
    if node.outputs[0].value_type != inferred {
        return Err(GraphValidationError::OutputTypeMismatch(node.id.clone()));
    }
    Ok(())
}

fn broadcast_dimensions(
    left: &[SymbolicDimension],
    right: &[SymbolicDimension],
) -> Result<Vec<SymbolicDimension>, GraphValidationError> {
    let rank = left.len().max(right.len());
    let mut output = Vec::with_capacity(rank);
    for offset in (0..rank).rev() {
        let left_dimension = left.len().checked_sub(offset + 1).map(|index| &left[index]);
        let right_dimension = right
            .len()
            .checked_sub(offset + 1)
            .map(|index| &right[index]);
        let dimension = match (left_dimension, right_dimension) {
            (Some(left), Some(right)) if left == right => left.clone(),
            (Some(SymbolicDimension::Known(1)), Some(right)) => right.clone(),
            (Some(left), Some(SymbolicDimension::Known(1))) => left.clone(),
            (Some(_), Some(_)) => {
                return Err(GraphValidationError::ShapeMismatch(
                    "batch dimensions are not broadcast-compatible".to_owned(),
                ))
            }
            (Some(dimension), None) | (None, Some(dimension)) => dimension.clone(),
            (None, None) => unreachable!("rank bounds the loop"),
        };
        output.push(dimension);
    }
    Ok(output)
}
