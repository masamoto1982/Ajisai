//! Shared helpers for the conformance tests.
//!
//! Each integration test binary includes this module, and none of them uses
//! every helper.
#![allow(dead_code)]

use ajisai_core::{Error, Interpreter};

/// Run a program on a fresh interpreter and render the resulting flow.
pub fn flow(source: &str) -> Vec<String> {
    let mut interpreter = Interpreter::new();
    interpreter
        .execute(source)
        .unwrap_or_else(|error| panic!("`{source}` failed unexpectedly: {error}"));
    ajisai_core::render_stack(&interpreter)
}

/// Run a program and render the flow as one space-separated line.
pub fn line(source: &str) -> String {
    flow(source).join(" ")
}

/// Run a program that is expected to fail, and return the error.
pub fn failure(source: &str) -> Error {
    let mut interpreter = Interpreter::new();
    match interpreter.execute(source) {
        Ok(()) => panic!(
            "`{source}` unexpectedly succeeded with flow {:?}",
            ajisai_core::render_stack(&interpreter)
        ),
        Err(error) => error,
    }
}
