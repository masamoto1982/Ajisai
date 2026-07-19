//! Exact-real numeric core: the observation interface and the tiered
//! representations behind it (SPEC §4.2).
//!
//! The public surface is representation-independent: values are observed
//! through [`observation::Observation`], and which tier implements a value
//! is never observable (SPEC §4.8).

pub mod algebraic;
mod algebraic_field;
mod algebraic_floor;
#[cfg(test)]
mod algebraic_tests;
pub(crate) mod basis;
pub mod computable;
pub mod observation;
pub mod pi;
pub mod value;
mod value_approx;

pub use algebraic::{Algebraic, AlgebraicResult};
pub use algebraic_floor::AlgebraicObservation;
pub use computable::{Computable, ComputableObservation};
pub use observation::{Observation, RatInterval, Refine, Water};
pub use value::{ExactCmp, ExactReal, DEFAULT_COMPARISON_WATER};
