use super::{
    matmul, Graph, GraphType, GraphValidationContext, GraphValidationError, OperatorSemantics,
    SymbolicDimension, Tensor, TensorMemoryBudget, TensorOperatorError,
};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum GraphExecutionError {
    InvalidGraph(GraphValidationError),
    MissingInput(String),
    InputDTypeMismatch(String),
    InputRankMismatch {
        id: String,
        expected: usize,
        actual: usize,
    },
    InputDimensionMismatch {
        id: String,
        axis: usize,
        expected: usize,
        actual: usize,
    },
    SymbolBindingMismatch {
        symbol: String,
        expected: usize,
        actual: usize,
    },
    Operator(TensorOperatorError),
    MissingRuntimeValue(String),
}

impl fmt::Display for GraphExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGraph(error) => error.fmt(f),
            Self::MissingInput(id) => write!(f, "missing graph input {id}"),
            Self::InputDTypeMismatch(id) => write!(f, "runtime dtype does not match {id}"),
            Self::InputRankMismatch {
                id,
                expected,
                actual,
            } => {
                write!(f, "runtime rank for {id} is {actual}, expected {expected}")
            }
            Self::InputDimensionMismatch {
                id,
                axis,
                expected,
                actual,
            } => write!(
                f,
                "runtime dimension {axis} for {id} is {actual}, expected {expected}"
            ),
            Self::SymbolBindingMismatch {
                symbol,
                expected,
                actual,
            } => write!(
                f,
                "symbolic dimension {symbol} was bound to {expected}, received {actual}"
            ),
            Self::Operator(error) => error.fmt(f),
            Self::MissingRuntimeValue(id) => write!(f, "runtime value {id} is unavailable"),
        }
    }
}

impl std::error::Error for GraphExecutionError {}

impl From<GraphValidationError> for GraphExecutionError {
    fn from(value: GraphValidationError) -> Self {
        Self::InvalidGraph(value)
    }
}

impl From<TensorOperatorError> for GraphExecutionError {
    fn from(value: TensorOperatorError) -> Self {
        Self::Operator(value)
    }
}

/// Validate and execute a Tensor Profile graph on the deterministic reference
/// CPU backend. Inputs and outputs retain their SSA IDs.
pub fn execute_graph(
    graph: &Graph,
    context: &GraphValidationContext,
    inputs: &BTreeMap<String, Tensor>,
    budget: TensorMemoryBudget,
) -> Result<BTreeMap<String, Tensor>, GraphExecutionError> {
    graph.validate(context)?;
    let mut symbols = BTreeMap::new();
    let mut values = BTreeMap::new();
    for declaration in &graph.inputs {
        let tensor = inputs
            .get(&declaration.id)
            .ok_or_else(|| GraphExecutionError::MissingInput(declaration.id.clone()))?;
        check_runtime_type(
            &declaration.id,
            &declaration.value_type,
            tensor,
            &mut symbols,
        )?;
        values.insert(declaration.id.clone(), tensor.clone());
    }

    for node in &graph.nodes {
        let semantics = context
            .operator_semantics
            .get(&node.operator_semantic_id)
            .expect("graph validation accepted this operator");
        let result = match semantics {
            OperatorSemantics::Matmul => {
                let left = runtime_value(&values, &node.inputs[0])?;
                let right = runtime_value(&values, &node.inputs[1])?;
                matmul(left, right, budget)?
            }
        };
        check_runtime_type(
            &node.outputs[0].id,
            &node.outputs[0].value_type,
            &result,
            &mut symbols,
        )?;
        values.insert(node.outputs[0].id.clone(), result);
    }

    graph
        .outputs
        .iter()
        .map(|id| {
            values
                .get(id)
                .cloned()
                .map(|value| (id.clone(), value))
                .ok_or_else(|| GraphExecutionError::MissingRuntimeValue(id.clone()))
        })
        .collect()
}

fn runtime_value<'a>(
    values: &'a BTreeMap<String, Tensor>,
    id: &str,
) -> Result<&'a Tensor, GraphExecutionError> {
    values
        .get(id)
        .ok_or_else(|| GraphExecutionError::MissingRuntimeValue(id.to_owned()))
}

fn check_runtime_type(
    id: &str,
    expected: &GraphType,
    tensor: &Tensor,
    symbols: &mut BTreeMap<String, usize>,
) -> Result<(), GraphExecutionError> {
    let GraphType::Tensor { dtype, shape } = expected;
    if *dtype != tensor.dtype() {
        return Err(GraphExecutionError::InputDTypeMismatch(id.to_owned()));
    }
    let actual = tensor.shape().dimensions();
    if shape.len() != actual.len() {
        return Err(GraphExecutionError::InputRankMismatch {
            id: id.to_owned(),
            expected: shape.len(),
            actual: actual.len(),
        });
    }
    for (axis, (expected, actual)) in shape.iter().zip(actual).enumerate() {
        match expected {
            SymbolicDimension::Known(expected) if expected != actual => {
                return Err(GraphExecutionError::InputDimensionMismatch {
                    id: id.to_owned(),
                    axis,
                    expected: *expected,
                    actual: *actual,
                })
            }
            SymbolicDimension::Symbol(symbol) => match symbols.get(symbol) {
                Some(expected) if expected != actual => {
                    return Err(GraphExecutionError::SymbolBindingMismatch {
                        symbol: symbol.clone(),
                        expected: *expected,
                        actual: *actual,
                    })
                }
                None => {
                    symbols.insert(symbol.clone(), *actual);
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(())
}
