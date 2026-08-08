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
    pub operator_semantic_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphValidationError {
    UnsupportedSchemaVersion(u32),
    ProfileNotSelected(String),
    InvalidIdentifier(String),
    DuplicateIdentifier(String),
    UndefinedValue(String),
    UnsupportedOperator(String),
    InvalidSymbolicDimension(String),
    InvalidArtifactHash(String),
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
            Self::InvalidSymbolicDimension(symbol) => {
                write!(f, "invalid symbolic dimension {symbol}")
            }
            Self::InvalidArtifactHash(hash) => write!(f, "invalid artifact hash {hash}"),
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
        let mut values = BTreeSet::new();
        for input in &self.inputs {
            validate_value(input)?;
            insert_unique(&mut values, &input.id)?;
        }
        for node in &self.nodes {
            if !valid_identifier(&node.id, '@') {
                return Err(GraphValidationError::InvalidIdentifier(node.id.clone()));
            }
            insert_unique(&mut node_ids, &node.id)?;
            if !context
                .operator_semantic_ids
                .contains(&node.operator_semantic_id)
            {
                return Err(GraphValidationError::UnsupportedOperator(
                    node.operator_semantic_id.clone(),
                ));
            }
            for input in &node.inputs {
                if !values.contains(input) {
                    return Err(GraphValidationError::UndefinedValue(input.clone()));
                }
            }
            for output in &node.outputs {
                validate_value(output)?;
                insert_unique(&mut values, &output.id)?;
            }
        }
        for output in &self.outputs {
            if !values.contains(output) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

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
            operator_semantic_ids: BTreeSet::from(["tensor.matmul.v1".to_owned()]),
        }
    }

    fn valid_graph() -> Graph {
        let matrix = vec![
            SymbolicDimension::Symbol("M".to_owned()),
            SymbolicDimension::Symbol("K".to_owned()),
        ];
        Graph {
            schema_version: 1,
            profiles: vec!["org.ajisai.tensor/0.1".to_owned()],
            inputs: vec![tensor("%left", matrix.clone()), tensor("%right", matrix)],
            nodes: vec![GraphNode {
                id: "@multiply".to_owned(),
                operator_semantic_id: "tensor.matmul.v1".to_owned(),
                inputs: vec!["%left".to_owned(), "%right".to_owned()],
                outputs: vec![tensor("%result", vec![SymbolicDimension::Known(2)])],
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
    fn committed_example_uses_the_committed_operator_contract() {
        #[derive(Deserialize)]
        struct Contract {
            profile: String,
            operators: Vec<Operator>,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Operator {
            semantic_id: String,
        }

        let contract: Contract =
            serde_json::from_str(include_str!("../../../spec/tensor-profile-v0.1.json")).unwrap();
        let graph: Graph = serde_json::from_str(include_str!(
            "../../../spec/examples/tiny-matmul.graph.json"
        ))
        .unwrap();
        let context = GraphValidationContext {
            profile_id: contract.profile,
            operator_semantic_ids: contract
                .operators
                .into_iter()
                .map(|operator| operator.semantic_id)
                .collect(),
        };
        graph.validate(&context).unwrap();
    }
}
