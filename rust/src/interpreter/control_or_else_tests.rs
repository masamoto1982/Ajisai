//! Test suite for `crate::interpreter::control::op_or_else` (OR-ELSE).
//!
//! OR-ELSE is the value-based, block-taking counterpart to OR-NIL (`^`,
//! formerly named VENT). These tests pin the two behaviours that matter for
//! the P1 surface-syntax work
//! (docs/dev/external-evaluation-response-strategy.md): it mirrors OR-NIL's
//! NIL-fallback semantics on values, and — unlike `^` — its fallback is a whole
//! `{ ... }` block, so its meaning does not depend on the lexical structure of
//! the tokens that follow.

#[cfg(test)]
mod tests {}
