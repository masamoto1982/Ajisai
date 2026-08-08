use super::DType;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const GRAPH_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SymbolicDimension {
    Known(usize),
    Symbol(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "domain", rename_all = "lowercase")]
pub enum GraphType {
    Tensor {
        dtype: DType,
        shape: Vec<SymbolicDimension>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphValue {
    pub id: String,
    #[serde(rename = "type")]
    pub value_type: GraphType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub operator_semantic_id: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<GraphValue>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactReference {
    pub semantic_id: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Graph {
    pub schema_version: u32,
    pub profiles: Vec<String>,
    pub inputs: Vec<GraphValue>,
    pub nodes: Vec<GraphNode>,
    pub outputs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ArtifactReference>,
}

/// The machine-readable profile facts needed to validate a graph without
/// teaching the graph module operator semantics of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphValidationContext {
    pub profile_id: String,
    pub operator_semantics: BTreeMap<String, OperatorSemantics>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorSemantics {
    Matmul,
    Exp,
    Log,
    ReduceSum,
    ReduceMax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphValidationError {
    UnsupportedSchemaVersion(u32),
    ProfileNotSelected(String),
    InvalidIdentifier(String),
    DuplicateIdentifier(String),
    UndefinedValue(String),
    UnsupportedOperator(String),
    OperatorArity {
        operator: String,
        expected_inputs: usize,
        actual_inputs: usize,
        expected_outputs: usize,
        actual_outputs: usize,
    },
    DTypeMismatch(String),
    ShapeMismatch(String),
    OutputTypeMismatch(String),
    InvalidSymbolicDimension(String),
    InvalidArtifactHash(String),
    InvalidAttribute {
        node: String,
        attribute: String,
    },
    NoOutputs,
}

impl fmt::Display for GraphValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported graph schema version {version}")
            }
            Self::ProfileNotSelected(profile) => write!(f, "graph does not select {profile}"),
            Self::InvalidIdentifier(id) => write!(f, "invalid graph identifier {id}"),
            Self::DuplicateIdentifier(id) => write!(f, "duplicate graph identifier {id}"),
            Self::UndefinedValue(id) => write!(f, "graph value {id} is used before definition"),
            Self::UnsupportedOperator(id) => write!(f, "unsupported operator semantic ID {id}"),
            Self::OperatorArity { operator, expected_inputs, actual_inputs, expected_outputs, actual_outputs } => write!(f, "{operator} requires {expected_inputs} inputs and {expected_outputs} output(s), received {actual_inputs} and {actual_outputs}"),
            Self::DTypeMismatch(id) => write!(f, "{id} input dtypes do not match"),
            Self::ShapeMismatch(message) => write!(f, "shape mismatch: {message}"),
            Self::OutputTypeMismatch(id) => write!(f, "{id} declared output type does not match its inferred type"),
            Self::InvalidSymbolicDimension(symbol) => {
                write!(f, "invalid symbolic dimension {symbol}")
            }
            Self::InvalidArtifactHash(hash) => write!(f, "invalid artifact hash {hash}"),
            Self::InvalidAttribute { node, attribute } => {
                write!(f, "{node} has an invalid {attribute} attribute")
            }
            Self::NoOutputs => write!(f, "graph must have at least one output"),
        }
    }
}

impl std::error::Error for GraphValidationError {}

impl Graph {
    /// Validate schema/profile selection and the SSA use-before-definition
    /// invariant. Node order is semantic graph order, so cycles are rejected
    /// without a second graph representation.
    pub fn validate(&self, context: &GraphValidationContext) -> Result<(), GraphValidationError> {
        if self.schema_version != GRAPH_SCHEMA_VERSION {
            return Err(GraphValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if !self
            .profiles
            .iter()
            .any(|profile| profile == &context.profile_id)
        {
            return Err(GraphValidationError::ProfileNotSelected(
                context.profile_id.clone(),
            ));
        }
        if self.outputs.is_empty() {
            return Err(GraphValidationError::NoOutputs);
        }

        let mut node_ids = BTreeSet::new();
        let mut values = BTreeMap::new();
        for input in &self.inputs {
            validate_value(input)?;
            insert_value(&mut values, input)?;
        }
        for node in &self.nodes {
            if !valid_identifier(&node.id, '@') {
                return Err(GraphValidationError::InvalidIdentifier(node.id.clone()));
            }
            insert_unique(&mut node_ids, &node.id)?;
            let semantics = context
                .operator_semantics
                .get(&node.operator_semantic_id)
                .ok_or_else(|| {
                    GraphValidationError::UnsupportedOperator(node.operator_semantic_id.clone())
                })?;
            for input in &node.inputs {
                if !values.contains_key(input) {
                    return Err(GraphValidationError::UndefinedValue(input.clone()));
                }
            }
            validate_operator(node, *semantics, &values)?;
            for output in &node.outputs {
                validate_value(output)?;
                insert_value(&mut values, output)?;
            }
        }
        for output in &self.outputs {
            if !values.contains_key(output) {
                return Err(GraphValidationError::UndefinedValue(output.clone()));
            }
        }
        for artifact in &self.artifacts {
            if !valid_sha256(&artifact.content_hash) {
                return Err(GraphValidationError::InvalidArtifactHash(
                    artifact.content_hash.clone(),
                ));
            }
        }
        Ok(())
    }

    /// Content identity over the canonical serialized graph. Backend, device,
    /// and execution receipt are deliberately absent from this structure.
    pub fn semantic_identity(&self) -> Result<String, serde_json::Error> {
        let bytes = serde_json::to_vec(self)?;
        Ok(format!("blake3:{}", blake3::hash(&bytes).to_hex()))
    }
}

fn validate_value(value: &GraphValue) -> Result<(), GraphValidationError> {
    if !valid_identifier(&value.id, '%') {
        return Err(GraphValidationError::InvalidIdentifier(value.id.clone()));
    }
    let GraphType::Tensor { shape, .. } = &value.value_type;
    for dimension in shape {
        if let SymbolicDimension::Symbol(symbol) = dimension {
            let valid = symbol
                .chars()
                .next()
                .is_some_and(|first| first.is_ascii_alphabetic())
                && symbol
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_');
            if !valid {
                return Err(GraphValidationError::InvalidSymbolicDimension(
                    symbol.clone(),
                ));
            }
        }
    }
    Ok(())
}

fn insert_unique(set: &mut BTreeSet<String>, id: &str) -> Result<(), GraphValidationError> {
    if !set.insert(id.to_owned()) {
        return Err(GraphValidationError::DuplicateIdentifier(id.to_owned()));
    }
    Ok(())
}

fn insert_value(
    values: &mut BTreeMap<String, GraphType>,
    value: &GraphValue,
) -> Result<(), GraphValidationError> {
    if values
        .insert(value.id.clone(), value.value_type.clone())
        .is_some()
    {
        return Err(GraphValidationError::DuplicateIdentifier(value.id.clone()));
    }
    Ok(())
}

fn validate_operator(
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
    }
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

fn valid_identifier(id: &str, prefix: char) -> bool {
    id.strip_prefix(prefix).is_some_and(|body| {
        !body.is_empty()
            && body
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
    })
}

fn valid_sha256(hash: &str) -> bool {
    hash.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_uppercase())
    })
}
