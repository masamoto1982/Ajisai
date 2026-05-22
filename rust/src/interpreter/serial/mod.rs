//! `SERIAL` module — Web Serial / native serial port I/O.
//!
//! Phase 1 scope: outbound (send-side) words only. Each word emits a single
//! `SERIAL:{json}` command line into the interpreter output buffer, mirroring
//! how the `MUSIC` module emits `AUDIO:` commands. The Rust core never touches
//! a real port; the main-thread platform adapter consumes these commands and
//! drives `navigator.serial` (web) or a native serial backend (Tauri).
//!
//! Inbound (read-side) words and the received-byte inbox are Phase 2 and are
//! intentionally absent here. See `docs/dev/web-serial-module-design.md`.

mod execute_serial_commands;

pub use execute_serial_commands::{
    op_close, op_configure, op_flush, op_list_ports, op_open, op_write,
};

#[cfg(test)]
mod serial_command_tests;
