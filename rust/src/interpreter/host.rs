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

/// A capability the host must provide for a Hosted-profile word to run.
///
/// Core-profile words never require a `HostCapability`. Hosted-profile words
/// declare exactly one. This enum is the vocabulary used to describe those
/// requirements; routing words through it (and failing in a specified way when
/// a capability is absent) is future work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostCapability {
    Clock,
    SecureRandom,
    Serial,
    Audio,
    JsonExport,
    Config,
    Effect,
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
