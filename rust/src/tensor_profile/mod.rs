//! Runtime foundations for the opt-in Ajisai Tensor Profile.
//!
//! This module is intentionally separate from [`crate::kernel`]. Approximate
//! tensors are profile values and never widen Core's exact `Scalar` domain.

mod graph;
mod shape;
mod tensor;

pub use graph::{
    ArtifactReference, Graph, GraphNode, GraphType, GraphValidationContext, GraphValidationError,
    GraphValue, OperatorSemantics, SymbolicDimension,
};
pub use shape::{CheckedShape, ShapeError, TensorMemoryBudget};
pub use tensor::{DType, Tensor, TensorData, TensorError};

#[cfg(test)]
mod graph_tests;
