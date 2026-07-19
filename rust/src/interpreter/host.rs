//! Host abstraction scaffolding (portability first step).
//!
//! Ajisai Core is host-independent; anything that reaches outside the pure
//! value model (the wall clock, a CSPRNG, a serial port, audio, JSON export)
//! is a *host capability*. When such a word runs it produces a structured
//! `HostEffect` rather than only appending a string to `output_buffer`.
//!
//! The conformance suite (`tests/conformance/`) observes the ordered sequence
//! of `HostEffect`s, not the human-readable `output_buffer`. Structuring the
//! effects this way lets two independent implementations be compared
//! language-independently: they agree iff they emit the same effect列.
//!
//! This is a first scaffold. Payloads are kept as `String` for now and the
//! legacy `output_buffer` string protocol (`SERIAL:`, `AUDIO:`, ...) is still
//! emitted in parallel so the Web/Tauri front-ends keep working unchanged.

use crate::error::AjisaiError;
use std::sync::{Arc, Mutex};

/// A capability the host must provide for a Hosted-profile word to run.
///
/// Core-profile words never require a `HostCapability`. Hosted-profile words
/// declare exactly one. This enum is the vocabulary used to describe those
/// requirements. Hosted words must query the active `HostEnv` before touching
/// their boundary and must fail through `missing_capability_error` when absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HostCapability {
    Clock,
    SecureRandom,
    Serial,
    Audio,
    JsonExport,
    Config,
    Effect,
}

impl HostCapability {
    /// Every modeled capability, in a stable order. Used to enumerate which
    /// capabilities the active host grants for an execution receipt (Phase 6).
    pub const ALL: [HostCapability; 7] = [
        HostCapability::Clock,
        HostCapability::SecureRandom,
        HostCapability::Serial,
        HostCapability::Audio,
        HostCapability::JsonExport,
        HostCapability::Config,
        HostCapability::Effect,
    ];
}

impl HostCapability {
    pub fn as_protocol_str(self) -> &'static str {
        match self {
            HostCapability::Clock => "clock",
            HostCapability::SecureRandom => "secureRandom",
            HostCapability::Serial => "serial",
            HostCapability::Audio => "audio",
            HostCapability::JsonExport => "jsonExport",
            HostCapability::Config => "config",
            HostCapability::Effect => "effect",
        }
    }
}

/// A structured, observable side effect produced by a Hosted-profile word.
///
/// Each variant corresponds to a conformance `data-kind` (see `kind`). The
/// payload is the same text that is (for now) also written to the legacy
/// `output_buffer` protocol line, so the two observation channels stay in sync.
#[derive(Debug, Clone, PartialEq)]
pub enum HostEffect {
    Print(String),
    Audio(String),
    Config(String),
    Effect(String),
    Serial(String),
    JsonExport(String),
    Diagnostic(String),
}

impl HostEffect {
    /// Stable, language-independent kind tag. This is the string the
    /// conformance suite carries in `data-kind` on each `ajisai-effect`.
    pub fn kind(&self) -> &'static str {
        match self {
            HostEffect::Print(_) => "print",
            HostEffect::Audio(_) => "audio",
            HostEffect::Config(_) => "config",
            HostEffect::Effect(_) => "effect",
            HostEffect::Serial(_) => "serial",
            HostEffect::JsonExport(_) => "json_export",
            HostEffect::Diagnostic(_) => "diagnostic",
        }
    }

    /// The effect payload. Conformance carries this in `data-payload`.
    pub fn payload(&self) -> &str {
        match self {
            HostEffect::Print(s)
            | HostEffect::Audio(s)
            | HostEffect::Config(s)
            | HostEffect::Effect(s)
            | HostEffect::Serial(s)
            | HostEffect::JsonExport(s)
            | HostEffect::Diagnostic(s) => s,
        }
    }
}

/// Runtime boundary supplied by the embedding host.
///
/// The interpreter owns no direct clock, entropy source, or effect sink. The
/// default implementation delegates to the current platform, while tests and
/// conformance runners can inject a deterministic implementation.
pub trait HostEnv: Send + Sync {
    fn now_millis(&self) -> i64;
    fn fill_random(&self, buf: &mut [u8]) -> std::result::Result<(), String>;
    fn emit_effect(&self, _effect: &HostEffect) {}
    fn has_capability(&self, capability: HostCapability) -> bool;
}

/// Default host: exposes every currently modeled capability and delegates clock
/// and entropy access to the platform-specific boundary functions.
#[derive(Debug, Default)]
pub struct DefaultHostEnv;

impl HostEnv for DefaultHostEnv {
    fn now_millis(&self) -> i64 {
        crate::interpreter::datetime::default_now_millis()
    }

    fn fill_random(&self, buf: &mut [u8]) -> std::result::Result<(), String> {
        crate::interpreter::random::default_fill_random(buf)
    }

    fn has_capability(&self, _capability: HostCapability) -> bool {
        true
    }
}

pub fn default_host_env() -> Arc<dyn HostEnv> {
    Arc::new(DefaultHostEnv)
}

/// Deterministic host used by conformance tests for otherwise non-deterministic
/// Hosted words such as `TIME@NOW` and `CRYPTO@CSPRNG`.
#[derive(Debug)]
pub struct DeterministicHostEnv {
    now_millis: i64,
    random_bytes: Mutex<Vec<u8>>,
    capabilities: Vec<HostCapability>,
    emitted_effects: Mutex<Vec<HostEffect>>,
}

impl DeterministicHostEnv {
    pub fn new(now_millis: i64, random_bytes: Vec<u8>, capabilities: Vec<HostCapability>) -> Self {
        Self {
            now_millis,
            random_bytes: Mutex::new(random_bytes),
            capabilities,
            emitted_effects: Mutex::new(Vec::new()),
        }
    }

    pub fn all_capabilities(now_millis: i64, random_bytes: Vec<u8>) -> Self {
        Self::new(
            now_millis,
            random_bytes,
            vec![
                HostCapability::Clock,
                HostCapability::SecureRandom,
                HostCapability::Serial,
                HostCapability::Audio,
                HostCapability::JsonExport,
                HostCapability::Config,
                HostCapability::Effect,
            ],
        )
    }

    pub fn emitted_effects(&self) -> Vec<HostEffect> {
        self.emitted_effects.lock().unwrap().clone()
    }
}

impl HostEnv for DeterministicHostEnv {
    fn now_millis(&self) -> i64 {
        self.now_millis
    }

    fn fill_random(&self, buf: &mut [u8]) -> std::result::Result<(), String> {
        let mut bytes = self.random_bytes.lock().unwrap();
        if bytes.len() < buf.len() {
            return Err(format!(
                "deterministic host exhausted: requested {} bytes, {} remain",
                buf.len(),
                bytes.len()
            ));
        }
        let requested = buf.len();
        for (dst, src) in buf.iter_mut().zip(bytes.drain(..requested)) {
            *dst = src;
        }
        Ok(())
    }

    fn emit_effect(&self, effect: &HostEffect) {
        self.emitted_effects.lock().unwrap().push(effect.clone());
    }

    fn has_capability(&self, capability: HostCapability) -> bool {
        self.capabilities.contains(&capability)
    }
}

pub(crate) fn missing_capability_payload(word: &str, capability: HostCapability) -> String {
    format!(
        r#"{{"capability":"{}","op":"missingCapability","word":"{}"}}"#,
        capability.as_protocol_str(),
        word
    )
}

pub(crate) fn missing_capability_error(word: &str, capability: HostCapability) -> AjisaiError {
    AjisaiError::from(format!(
        "{} requires missing host capability {}",
        word,
        capability.as_protocol_str()
    ))
}
