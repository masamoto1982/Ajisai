//! `SERIAL` module — Web Serial / native serial port I/O.
//!
//! Outbound (send-side) words emit a single `SERIAL:{json}` command line into
//! the interpreter output buffer, mirroring how the `MUSIC` module emits
//! `AUDIO:` commands. The Rust core never touches a real port; the main-thread
//! platform adapter consumes these commands and drives `navigator.serial`
//! (web) or a native serial backend (Tauri).
//!
//! Inbound: `READ` drains the host-injected receive buffer (`serial_inbox` on
//! the interpreter), returning a byte vector or a reasoned Bubble/NIL when no
//! data is available. See `docs/dev/web-serial-module-design.md` and
//! SPECIFICATION.html §9.4.

mod execute_serial_commands;

pub use execute_serial_commands::{
    op_close, op_configure, op_flush, op_list_ports, op_open, op_read, op_write,
};

#[cfg(test)]
mod serial_command_tests;
