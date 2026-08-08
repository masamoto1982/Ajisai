//! Runtime foundations for the opt-in Ajisai Tensor Profile.
//!
//! This module is intentionally separate from [`crate::kernel`]. Profile
//! tensors never widen Core's `Scalar` domain — including the exact `q`
//! tensors, whose elements are Core fractions but whose containing value is a
//! profile value that Core cannot observe.

mod cpu;
mod elementwise;
mod execute;
mod graph;
mod graph_operators;
mod reductions;
mod regrid;
mod select;
mod shape;
mod tensor;

pub use cpu::{matmul, tensor_exp, tensor_log, tensor_rsqrt, TensorOperatorError};
pub(crate) use cpu::{require_exact, require_numeric};
pub use elementwise::{tensor_add, tensor_div, tensor_mul, tensor_sub};
pub use execute::{execute_graph, GraphExecutionError};
pub use graph::{
    ArtifactReference, Graph, GraphNode, GraphType, GraphValidationContext, GraphValidationError,
    GraphValue, OperatorSemantics, SymbolicDimension,
};
pub use reductions::{reduce_max, reduce_sum};
pub use regrid::tensor_regrid;
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

#[cfg(test)]
mod rational_growth_tests;
