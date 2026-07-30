//! The `Scalar` value domain and its private representation.
//!
//! A `Scalar` is *one* value domain. Whether it is stored as a rational
//! ([`Fraction`]) or as an exact real ([`ExactReal`]) is an internal
//! optimization, not a distinction the language exposes (migration plan §6).
//! The exact-real backing is never named in this type's public API — only the
//! rational fast-path view and a representation predicate are exposed.

use crate::types::exact::ExactReal;
use crate::types::fraction::Fraction;

/// A single Ajisai number.
#[derive(Clone, Debug, PartialEq)]
pub struct Scalar {
    repr: ScalarRepr,
}

/// Private storage for a [`Scalar`]. Demoted here from the value model so the
/// exact-real machinery stays below the Semantic Spine.
#[derive(Clone, Debug, PartialEq)]
enum ScalarRepr {
    /// A rational number — the fast path.
    Float(Fraction),
    /// An exact real backed by a continued-fraction representation.
    Exact(ExactReal),
}

impl Scalar {
    /// Build a scalar from a rational.
    pub fn from_fraction(value: Fraction) -> Self {
        Self {
            repr: ScalarRepr::Float(value),
        }
    }

    /// Build a scalar from an exact real.
    pub fn from_exact(value: ExactReal) -> Self {
        Self {
            repr: ScalarRepr::Exact(value),
        }
    }

    /// The rational view of the scalar, when it is stored as a rational.
    /// Returns `None` for an exact-real scalar; callers that need a rational
    /// there ask the exact layer for an approximation below the spine.
    pub fn as_fraction(&self) -> Option<&Fraction> {
        match &self.repr {
            ScalarRepr::Float(value) => Some(value),
            ScalarRepr::Exact(_) => None,
        }
    }

    /// Whether the scalar is stored in the exact-real representation. This is a
    /// *representation* predicate, not a value-domain distinction: an exact and
    /// a rational scalar are both `Scalar` to a program.
    pub fn is_exact(&self) -> bool {
        matches!(self.repr, ScalarRepr::Exact(_))
    }

    /// Crate-internal view of the exact-real backing. Used only by the
    /// temporary legacy adapter to raise a spine scalar back to the old value
    /// model; it is not part of the public API, because a program never
    /// observes the backing — only that the value is a `Scalar`.
    pub(crate) fn exact_backing(&self) -> Option<&ExactReal> {
        match &self.repr {
            ScalarRepr::Exact(value) => Some(value),
            ScalarRepr::Float(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_scalar_exposes_its_fraction_and_is_not_exact() {
        let scalar = Scalar::from_fraction(Fraction::from(3_i64));
        assert_eq!(scalar.as_fraction(), Some(&Fraction::from(3_i64)));
        assert!(!scalar.is_exact());
    }

    #[test]
    fn exact_scalar_hides_its_backing_from_the_public_api() {
        let scalar = Scalar::from_exact(ExactReal::from_fraction(Fraction::from(2_i64)));
        // The exact-real representation is not exposed as a rational view…
        assert_eq!(scalar.as_fraction(), None);
        // …only as an internal representation predicate.
        assert!(scalar.is_exact());
    }
}
