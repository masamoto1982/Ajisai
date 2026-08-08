use super::graph::{
    GraphNode, GraphType, GraphValidationError, OperatorSemantics, SymbolicDimension,
};
use super::DType;
use num_bigint::BigInt;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn validate_operator(
    node: &GraphNode,
    semantics: OperatorSemantics,
    values: &BTreeMap<String, GraphType>,
) -> Result<(), GraphValidationError> {
    match semantics {
        OperatorSemantics::Matmul => validate_matmul(node, values),
        OperatorSemantics::Exp | OperatorSemantics::Log | OperatorSemantics::Rsqrt => {
            validate_shape_preserving_unary(node, values, DTypeDomain::Approximate)
        }
        OperatorSemantics::Regrid => {
            regrid_denominator(node)?;
            validate_shape_preserving_unary(node, values, DTypeDomain::Exact)
        }
        OperatorSemantics::ReduceSum | OperatorSemantics::ReduceMax => {
            validate_reduction(node, values)
        }
        OperatorSemantics::Where => validate_where(node, values),
        OperatorSemantics::Add
        | OperatorSemantics::Sub
        | OperatorSemantics::Mul
        | OperatorSemantics::Div => validate_elementwise_binary(node, values),
    }
}

/// Which numeric domain an operator's contract is written over.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DTypeDomain {
    /// Any numeric dtype, exact or approximate.
    Numeric,
    /// Approximate only: the operator's value is irrational for rational
    /// inputs, so there is no exact result to return.
    Approximate,
    /// Exact only: the operator acts on a denominator, which the approximate
    /// dtypes do not have.
    Exact,
}

/// The graph validator is the profile's contract certifier, so it — not the
/// backend — is where a tensor is refused entry to an operator whose contract
/// is not written over its dtype. Rejecting here means an unexecutable graph is
/// never certified as valid in the first place.
fn require_domain(
    node: &GraphNode,
    dtype: DType,
    domain: DTypeDomain,
) -> Result<(), GraphValidationError> {
    if !dtype.is_numeric() {
        return Err(GraphValidationError::NonNumericDType(node.id.clone()));
    }
    match domain {
        DTypeDomain::Numeric => Ok(()),
        DTypeDomain::Approximate if dtype.is_approximate() => Ok(()),
        DTypeDomain::Approximate => {
            Err(GraphValidationError::ExactDTypeUnsupported(node.id.clone()))
        }
        DTypeDomain::Exact if dtype.is_exact() => Ok(()),
        DTypeDomain::Exact => Err(GraphValidationError::ApproximateDTypeUnsupported(
            node.id.clone(),
        )),
    }
}

fn require_numeric(node: &GraphNode, dtype: DType) -> Result<(), GraphValidationError> {
    require_domain(node, dtype, DTypeDomain::Numeric)
}

/// The `denominator` attribute of a `QUANTIZE` node: the grid the operator
/// rounds onto, and therefore the denominator bound it establishes.
pub(crate) fn regrid_denominator(node: &GraphNode) -> Result<BigInt, GraphValidationError> {
    let invalid = || GraphValidationError::InvalidAttribute {
        node: node.id.clone(),
        attribute: "denominator".to_owned(),
    };
    let denominator = node
        .attributes
        .get("denominator")
        .ok_or_else(invalid)?
        .as_u64()
        .filter(|denominator| *denominator > 0)
        .ok_or_else(invalid)?;
    Ok(BigInt::from(denominator))
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
    require_numeric(node, *left_dtype)?;
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
    require_numeric(node, *dtype)?;
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
    domain: DTypeDomain,
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
    let GraphType::Tensor { dtype, .. } = &values[&node.inputs[0]];
    require_domain(node, *dtype, domain)?;
    if node.outputs[0].value_type != values[&node.inputs[0]] {
        return Err(GraphValidationError::OutputTypeMismatch(node.id.clone()));
    }
    Ok(())
}

/// `WHERE` is the one operator whose inputs deliberately do not share a dtype:
/// a `bool` predicate selects between two operands of a common dtype `D`, and
/// the output shape is the three-way broadcast of all of them.
fn validate_where(
    node: &GraphNode,
    values: &BTreeMap<String, GraphType>,
) -> Result<(), GraphValidationError> {
    if node.inputs.len() != 3 || node.outputs.len() != 1 {
        return Err(GraphValidationError::OperatorArity {
            operator: node.operator_semantic_id.clone(),
            expected_inputs: 3,
            actual_inputs: node.inputs.len(),
            expected_outputs: 1,
            actual_outputs: node.outputs.len(),
        });
    }
    let GraphType::Tensor {
        dtype: predicate_dtype,
        shape: predicate,
    } = &values[&node.inputs[0]];
    if *predicate_dtype != DType::Bool {
        return Err(GraphValidationError::PredicateDTypeMismatch(
            node.id.clone(),
        ));
    }
    let GraphType::Tensor {
        dtype: true_dtype,
        shape: on_true,
    } = &values[&node.inputs[1]];
    let GraphType::Tensor {
        dtype: false_dtype,
        shape: on_false,
    } = &values[&node.inputs[2]];
    if true_dtype != false_dtype {
        return Err(GraphValidationError::DTypeMismatch(node.id.clone()));
    }
    let values_shape = broadcast_dimensions(on_true, on_false)?;
    let output_shape = broadcast_dimensions(predicate, &values_shape)?;
    let inferred = GraphType::Tensor {
        dtype: *true_dtype,
        shape: output_shape,
    };
    if node.outputs[0].value_type != inferred {
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
    require_numeric(node, *left_dtype)?;
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
