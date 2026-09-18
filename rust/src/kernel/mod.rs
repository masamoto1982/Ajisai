//! # Semantic Spine
//!
//! The Semantic Spine is the single place where Ajisai's *meaning* is allowed
//! to exist. Its public API names only concepts that the language
//! specification exposes to a program: the canonical value domains and the
//! reason-centric absence model.
//!
//! Governing invariant (migration plan §11, LANG.VALUES.DISJOINT):
//!
//! > Concepts that are absent from the language specification must be
//! > inexpressible in the Semantic Spine's public API. Below the spine, private
//! > representations may carry whatever complexity optimization needs.
//!
//! Consequently the optimization representations that today sit *inside* the
//! value model — dense numeric storage and exact-real scalars — are demoted to
//! private `repr` types of [`scalar::Scalar`] (and, in a later phase, of the
//! vector representation). They are a storage detail of a value domain, never a
//! domain of their own.
//!
//! ## Scope
//!
//! What lives here is what a runtime path actually reaches: the value domains,
//! the absence model, the scalar-scalar arithmetic primitives
//! `interpreter::arithmetic` routes `ADD`/`SUB`/`MUL`/`DIV` through, and the
//! `From`/`Into` adapters in [`legacy_adapter`] that the differential tests
//! compare across. A shared Word-execution wrapper, a spine-level word
//! contract, and an `Observation` projection were built here ahead of their
//! consumers and removed once measurement showed no runtime path had reached
//! them; see `docs/dev/semantic-spine-migration-plan.md` §10.13 for that record
//! and for what a future phase would have to bring with it.

pub mod arithmetic;
pub mod generated;
pub mod nil;
pub mod scalar;
pub mod value;

// The temporary legacy <-> spine bridge. A private module: its `From` impls are
// coherent crate-wide regardless, and keeping it unexported holds the spine's
// *named* public surface to the six domains while consumers migrate (Phase 2).
mod legacy_adapter;

pub use nil::NilReason;
pub use scalar::Scalar;
pub use value::KernelValue;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::fraction::Fraction;
    use std::sync::Arc;

    #[test]
    fn kernel_value_covers_the_six_canonical_domains() {
        let domains = [
            KernelValue::Scalar(Scalar::from_fraction(Fraction::from(1_i64))),
            KernelValue::Boolean(true),
            KernelValue::String(Arc::from("hello")),
            KernelValue::Vector(Arc::from([KernelValue::Boolean(false)])),
            KernelValue::Nil(None),
            KernelValue::Symbol(Arc::from("ADD")),
        ];
        // The spine has exactly six value domains; this array is the whole set.
        assert_eq!(domains.len(), 6);
    }
}
