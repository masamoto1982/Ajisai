//! Responsibility-focused authored portions of Core Word metadata.
//!
//! The modules own definitions; `builtin_word_definitions` owns the sole
//! canonical presentation order and validates it against the generated registry.

pub(super) mod collections;
pub(super) mod execution;
pub(super) mod numeric;
pub(super) mod scalar;
