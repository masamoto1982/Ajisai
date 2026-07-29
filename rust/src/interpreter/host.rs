//! Host abstraction for the one effect Ajisai has.
//!
//! Ajisai Core is host-independent. Output is the only effect
//! (LANG.EFFECTS.OUTPUT): `PRINT` appends to the ordered output stream, and
//! every other Word is pure. When `PRINT` runs it produces a structured
//! `HostEffect` rather than only appending a string to `output_buffer`.
//!
//! The conformance suite (`tests/conformance/`) observes the ordered sequence of
//! `HostEffect`s, not the human-readable `output_buffer`. Structuring the effect
//! this way lets two independent implementations be compared
//! language-independently: they agree iff they emit the same effect sequence.

use std::sync::{Arc, Mutex};

/// A structured effect request. Output is the only effect the language has, so
/// this carries exactly one variant; it stays an enum because the conformance
/// suite matches on the stable `kind` tag and the protocol pins that shape.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum HostEffect {
    Print(String),
}

impl HostEffect {
    /// Stable, language-independent kind tag. This is the string the
    /// conformance suite carries in `data-kind` on each `ajisai-effect`.
    pub fn kind(&self) -> &'static str {
        match self {
            HostEffect::Print(_) => "print",
        }
    }

    /// The effect payload. Conformance carries this in `data-payload`.
    pub fn payload(&self) -> &str {
        match self {
            HostEffect::Print(s) => s,
        }
    }
}

/// Runtime boundary supplied by the embedding host.
///
/// The interpreter owns no effect sink of its own. A host may render, capture,
/// or discard the output stream, but may not reorder it.
pub trait HostEnv: Send + Sync {
    fn emit_effect(&self, _effect: &HostEffect) {}
}

/// Default host: discards the structured effect, because the interpreter's own
/// `output_buffer` is what the GUI and CLI read.
#[derive(Debug, Default)]
pub struct DefaultHostEnv;

impl HostEnv for DefaultHostEnv {}

pub fn default_host_env() -> Arc<dyn HostEnv> {
    Arc::new(DefaultHostEnv)
}

/// Recording host used by conformance tests to observe the ordered effect
/// sequence directly rather than through rendered output text.
#[derive(Debug, Default)]
pub struct RecordingHostEnv {
    emitted_effects: Mutex<Vec<HostEffect>>,
}

impl RecordingHostEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn emitted_effects(&self) -> Vec<HostEffect> {
        self.emitted_effects.lock().unwrap().clone()
    }
}

impl HostEnv for RecordingHostEnv {
    fn emit_effect(&self, effect: &HostEffect) {
        self.emitted_effects.lock().unwrap().push(effect.clone());
    }
}
