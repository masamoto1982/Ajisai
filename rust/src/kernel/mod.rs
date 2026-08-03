//! # Semantic Spine (Phase 1 skeleton)
//!
//! The Semantic Spine is the single place where Ajisai's *meaning* is allowed
//! to exist. Its public API names only concepts that the language
//! specification exposes to a program: the canonical value domains, the
//! reason-centric absence model, and the outward-observable projection of the
//! runtime.
//!
//! Governing invariant (migration plan §11, SPEC §20):
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
//! This module is deliberately unwired in Phase 1: nothing in the existing
//! runtime references it yet. Phase 2 adds the `From`/`Into` adapters that let
//! the two models coexist while consumers migrate onto the spine one at a time.
//! See `docs/dev/semantic-spine-migration-plan.md`.

pub mod arithmetic;
pub mod execute;
pub mod generated;
pub mod host_protocol_v2;
pub mod nil;
pub mod observation;
pub mod scalar;
pub mod value;
pub mod word_contract;

// The temporary legacy <-> spine bridge. A private module: its `From` impls are
// coherent crate-wide regardless, and keeping it unexported holds the spine's
// *named* public surface to the six domains while consumers migrate (Phase 2).
mod legacy_adapter;

pub use nil::NilReason;
pub use observation::{Observation, ObservedValue, PresentationHint};
pub use scalar::Scalar;
pub use value::{CodeBlock, KernelValue};

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
            KernelValue::CodeBlock(CodeBlock::new(Arc::from([]))),
        ];
        // The spine has exactly six value domains; this array is the whole set.
        assert_eq!(domains.len(), 6);
    }

    #[test]
    fn observation_pairs_a_value_with_a_presentation_hint() {
        let observed = ObservedValue {
            value: KernelValue::String(Arc::from("2026-07-30")),
            presentation: PresentationHint::Timestamp,
        };
        let observation = Observation {
            stack: vec![observed.clone()],
        };
        assert_eq!(observation.stack, vec![observed]);
    }
}
