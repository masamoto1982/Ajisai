//! Host adapter for the headless `ajisai` CLI.
//!
//! The CLI is a terminal process: it has a clock and an OS entropy source,
//! and PRINT-style effects are observable through the collected effect log,
//! but there is no audio device, no serial port, and no GUI. Those
//! capabilities are therefore *not* advertised, so Hosted words that need
//! them fail through the structured `missing_capability_error` path
//! (capability gate before any stack consumption) instead of panicking or
//! silently pretending a device exists.

use crate::interpreter::{HostCapability, HostEnv};

#[derive(Debug, Default)]
pub(crate) struct CliHostEnv;

impl HostEnv for CliHostEnv {
    fn now_millis(&self) -> i64 {
        crate::interpreter::datetime::default_now_millis()
    }

    fn fill_random(&self, buf: &mut [u8]) -> std::result::Result<(), String> {
        crate::interpreter::random::default_fill_random(buf)
    }

    fn has_capability(&self, capability: HostCapability) -> bool {
        match capability {
            HostCapability::Clock
            | HostCapability::SecureRandom
            | HostCapability::JsonExport
            | HostCapability::Config
            | HostCapability::Effect => true,
            // No audio device or serial port exists in a headless terminal
            // process. MUSIC@* / SERIAL@* words fail with a structured
            // missing-capability diagnosis (why: environment).
            HostCapability::Audio | HostCapability::Serial => false,
        }
    }
}
