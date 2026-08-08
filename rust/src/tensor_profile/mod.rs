//! Runtime foundations for the opt-in Ajisai Tensor Profile.
//!
//! This module is intentionally separate from [`crate::kernel`]. Approximate
//! tensors are profile values and never widen Core's exact `Scalar` domain.

mod cpu;
mod elementwise;
mod execute;
mod graph;
mod graph_operators;
mod reductions;
mod select;
mod shape;
mod tensor;

pub(crate) use cpu::require_numeric;
pub use cpu::{matmul, tensor_exp, tensor_log, tensor_rsqrt, TensorOperatorError};
pub use elementwise::{tensor_add, tensor_div, tensor_mul, tensor_sub};
pub use execute::{execute_graph, GraphExecutionError};
pub use graph::{
    ArtifactReference, Graph, GraphNode, GraphType, GraphValidationContext, GraphValidationError,
    GraphValue, OperatorSemantics, SymbolicDimension,
};
pub use reductions::{reduce_max, reduce_sum};
pub use select::tensor_where;
pub use shape::{CheckedShape, ShapeError, TensorMemoryBudget};
pub use tensor::{DType, Tensor, TensorData, TensorError};

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod graph_tests;

#[cfg(test)]
mod cpu_tests;

#[cfg(test)]
mod execute_tests;

#[cfg(test)]
mod composition_tests;
