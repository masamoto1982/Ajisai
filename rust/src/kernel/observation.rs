//! The outward-observable projection of the runtime.
//!
//! Conformance tests, host protocols, the CLI, and the GUI compare an
//! [`Observation`] — never the engine's private representation (migration plan
//! §17–18). This is what lets dense storage, exact scalars, caches, and
//! compiled plans change freely without breaking observed behavior: they are
//! below the spine, and an observation cannot see them.

use super::value::KernelValue;

/// A snapshot of what an Ajisai program has produced, as observed from outside
/// the runtime. Phase 1 models the stack; the dictionary and status projections
/// are added as later phases route their consumers through the spine.
#[derive(Clone, Debug, PartialEq)]
pub struct Observation {
    pub stack: Vec<ObservedValue>,
}

/// One observed stack entry: a [`KernelValue`] together with the
/// presentation-only annotation used to render it.
#[derive(Clone, Debug, PartialEq)]
pub struct ObservedValue {
    pub value: KernelValue,
    pub presentation: PresentationHint,
}

/// A presentation-only annotation carried alongside an observed value.
///
/// A presentation hint selects how a value is *displayed*; it never changes
/// what a program computes (migration plan §8). This is the safe home for the
/// display intents that used to ride on values as execution-affecting
/// interpretation roles: rendering a scalar pair as an interval, or an integer
/// as a timestamp, is a rendering choice made after observation, not a second
/// type system layered over the value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PresentationHint {
    /// No display intent; render the value structurally.
    #[default]
    Structural,
    /// Render as text.
    Text,
    /// Render a two-element vector as a closed interval.
    Interval,
    /// Render an integer as a timestamp.
    Timestamp,
}
