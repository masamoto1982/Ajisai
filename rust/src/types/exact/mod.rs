//! Exact-real numeric core (LANG.VALUES.EXACT): the rationals and the
//! multiquadratic algebraic field `SQRT` builds over them.
//!
//! The public surface is representation-independent: which representation
//! holds a value is never observable (LANG.AUTHORITY.FREEDOM), and every
//! sign, floor and comparison over it decides.

pub mod algebraic;
mod algebraic_field;
mod algebraic_floor;
#[cfg(test)]
mod algebraic_tests;
pub(crate) mod basis;
mod power;
pub mod squarefree;
pub mod value;
mod value_approx;

pub use algebraic::{Algebraic, AlgebraicResult};
pub use power::PowOutcome;
pub use value::ExactReal;
