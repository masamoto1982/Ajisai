//! Tier 2: general computable reals — the receptacle (SPEC §4.2).
//!
//! A `Computable` is a lazily refined, monotonically shrinking rational
//! enclosure: a pure, deterministic generator from a refinement step to
//! the interval known after that many steps. This is the tier for values
//! with no algebraic normal form (π, and a future e, log); their comparisons
//! genuinely consume water and may starve — the sole legitimate source
//! of the logical `Unknown` (U) that is not an absent operand.
//!
//! `PI` constructs this tier, so everything below is reachable from a
//! program. In particular the allocation identity that `PartialEq`/`Hash`
//! fall back on must never decide an answer a program can observe: see the
//! two impls below.

use crate::types::exact::observation::{Observation, RatInterval, Refine, Water};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use std::sync::Arc;

/// Deterministic generator: the enclosure known after `step` refinement
/// steps. Contract: nested (`gen(k+1) ⊆ gen(k)`) and shrinking toward
/// the value.
type EnclosureFn = dyn Fn(u64) -> RatInterval + Send + Sync;

#[derive(Clone)]
pub struct Computable {
    gen: Arc<EnclosureFn>,
    /// Short human-readable tag for diagnostics (`Debug` only — never
    /// observable through the language surface).
    tag: &'static str,
}

impl std::fmt::Debug for Computable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Computable")
            .field("tag", &self.tag)
            .finish()
    }
}

/// Identity of the observation *process*, not the limit value: equality of
/// two computable reals is undecidable, so `PartialEq` is pointer identity.
///
/// This answers from *how a value was made*, which no program may observe:
/// LANG.VALUES.DENOTATION makes construction history unreadable from a
/// value, and LANG.AUTHORITY.FREEDOM lists allocation identity as no
/// semantic discriminant. So a `true` here is not a licence to decide — two
/// π built separately and one π used twice are the same value, and letting
/// this impl settle either pair splits them. `Value` equality therefore
/// routes any Tier 2 operand to the budgeted comparison instead
/// (`Value::carries_computable`), which answers the honest UNKNOWN.
impl PartialEq for Computable {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.gen, &other.gen)
    }
}

/// Hashes the same pointer identity `==` compares, so the two agree the only
/// way they can — a value-identical hash would need a canonical bucket, and
/// finding one for a computable real is exactly the undecidable question.
///
/// KNOWN LIMITATION: the hash-keyed collection Words (`UNIQUE`, `TALLY`,
/// `GROUP`, `INDEX-OF`) still bucket through this, so they still answer from
/// allocation identity for a Tier 2 element — `PI PI 2 COLLECT UNIQUE` keeps
/// two, the same π bound once and used twice keeps one. Making them project
/// the undecidable NIL that `SORT`/`ORDER` project is the open follow-up;
/// the comparison family (`EQ`/`NEQ`) no longer reads this impl at all.
impl std::hash::Hash for Computable {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (Arc::as_ptr(&self.gen) as *const ()).hash(state);
    }
}

impl Computable {
    /// Build from a nested-enclosure generator. The reference way to
    /// define a Tier 2 value.
    pub fn from_enclosures(
        tag: &'static str,
        gen: impl Fn(u64) -> RatInterval + Send + Sync + 'static,
    ) -> Computable {
        Computable {
            gen: Arc::new(gen),
            tag,
        }
    }

    /// Reference value: enclosures `[-2⁻ᵏ, 2⁻ᵏ]` — a process converging
    /// to zero that no finite refinement separates from zero. The
    /// canonical starvation witness for Tier 2 tests.
    pub fn vanishing() -> Computable {
        Self::from_enclosures("vanishing", |step| {
            let scale = BigInt::from(1) << step.min(4096) as usize;
            RatInterval::new(
                Fraction::new(BigInt::from(-1), scale.clone()),
                Fraction::new(BigInt::from(1), scale),
            )
        })
    }

    /// The enclosure after `step` refinement steps.
    pub fn enclosure_at(&self, step: u64) -> RatInterval {
        (self.gen)(step)
    }
}

/// A `Computable` paired with its refinement progress: the stateful
/// observation the `Observation` trait needs. Refining spends one step
/// of water per unit and can always report only `Narrower` or `Starved`
/// — a Tier 2 process never proves its exact value.
pub struct ComputableObservation {
    value: Computable,
    step: u64,
}

impl ComputableObservation {
    pub fn new(value: Computable) -> Self {
        ComputableObservation { value, step: 0 }
    }
}

impl Observation for ComputableObservation {
    fn current_interval(&self) -> Option<RatInterval> {
        Some(self.value.enclosure_at(self.step))
    }

    fn refine(&mut self, w: Water) -> Refine {
        if w.0 == 0 {
            return Refine::Starved;
        }
        self.step = self.step.saturating_add(w.0);
        Refine::Narrower
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanishing_encloses_zero_at_every_step_and_narrows() {
        let v = Computable::vanishing();
        let mut prev = v.enclosure_at(0);
        for step in 1..12 {
            let now = v.enclosure_at(step);
            assert!(now.is_within(&prev), "enclosures must be nested");
            assert!(!now.lo.is_positive(), "zero stays inside (lo <= 0)");
            assert!(
                now.hi.is_positive() || now.hi.is_zero(),
                "zero stays inside (hi >= 0)"
            );
            prev = now;
        }
    }

    #[test]
    fn observation_narrows_but_never_settles() {
        let mut obs = ComputableObservation::new(Computable::vanishing());
        let first = obs.current_interval().expect("always enclosed");
        assert_eq!(obs.refine(Water(0)), Refine::Starved, "no water, no work");
        assert_eq!(obs.refine(Water(16)), Refine::Narrower);
        let second = obs.current_interval().expect("still enclosed");
        assert!(second.is_within(&first));
        assert!(second.width().lt(&first.width()));
    }

    #[test]
    fn identity_is_process_identity() {
        let a = Computable::vanishing();
        let b = a.clone();
        let c = Computable::vanishing();
        assert_eq!(a, b, "clones share the process");
        assert_ne!(a, c, "separate processes are conservatively unequal");
    }
}
