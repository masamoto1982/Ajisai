//! Ajisai core library.
//!
//! Phase 1 scope:
//!  - All numeric values are stored as continued fractions `(a0 (a1 (a2 ...)))`.
//!  - Stack-oriented interpreter (no return stack).
//!  - Four arithmetic operations on continued fractions via exact rational pivot.
//!  - DEF / DEL for user word definitions.
//!  - Nil as bubble (propagates through operations).
//!
//! Internal representation is intentionally hidden from observable semantics:
//! protocol fields exposed across the WASM boundary follow the semantic-plane
//! contract defined in `SPECIFICATION.md`.

pub mod cf;
pub mod error;
pub mod tokenizer;
pub mod value;
pub mod interpreter;
pub mod wasm;

pub use wasm::AjisaiInterpreter;

#[cfg(test)]
mod tests;
