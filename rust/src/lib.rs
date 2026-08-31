// Test files follow the convention `mod <file_name> { … }` inside
// `<file_name>.rs` (e.g. `mod runtime_limits_tests` in
// `runtime_limits_tests.rs`), which clippy flags as `module_inception`. The
// nesting is a deliberate test-organization convention, not an accident, and
// there are no production inception cases, so allow it crate-wide.
#![allow(clippy::module_inception)]
// The crate is `unsafe`-free, enforced by the compiler. `deny` rather than
// `forbid` because the `wasm`-gated bindings module must re-permit it:
// `wasm-bindgen` expands to generated glue that contains `unsafe`.
#![deny(unsafe_code)]

mod builtins;
pub mod core_word_aliases;
pub mod coreword_registry;
mod error;
pub use error::{AjisaiError, ErrorCategory, NilReason};
pub mod interpreter;
pub mod kernel;
pub mod semantic;
pub mod surface_forms;
mod tokenizer;
pub mod types;

// Host-neutral agent boundary (pure computation, no filesystem/terminal I/O):
// shared by the native CLI below and the WASM one-shot entry point in
// `wasm_interpreter_bindings`, so every host renders the identical schema-1
// envelope (`docs/dev/agent-cli-output-contract.md`).
#[cfg(feature = "std")]
pub mod agent;

// Headless agent-facing CLI (the `ajisai` bin target). Native-only: it is
// host-adapter plumbing (file I/O, terminal rendering, REPL) over
// `crate::agent`.
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
pub mod cli;

#[cfg(feature = "wasm")]
mod wasm_interpreter_bindings;

#[cfg(feature = "wasm")]
pub use wasm_interpreter_bindings::AjisaiInterpreter;

#[cfg(test)]
mod tokenizer_regression_tests;

#[cfg(test)]
mod tokenizer_regression_tests_2;

#[cfg(test)]
mod tokenizer_mcdc_tests;

#[cfg(test)]
mod arithmetic_operation_tests;

#[cfg(test)]
mod dimension_limit_tests;

#[cfg(test)]
mod materialization_limit_tests;

#[cfg(test)]
mod runtime_limits_tests;

#[cfg(test)]
mod extreme_index_tests;

#[cfg(test)]
mod conformance_tests;

#[cfg(test)]
mod or_nil_canonical_tests;

#[cfg(test)]
mod stack_render_tests;

#[cfg(test)]
mod role_ownership_tests;
