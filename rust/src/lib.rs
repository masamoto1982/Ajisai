mod builtins;
pub mod core_word_aliases;
pub mod coreword_registry;
pub mod elastic;
mod error;
pub use error::{AjisaiError, ErrorCategory, NilReason};
pub mod interpreter;
pub mod semantic;
pub mod surface_forms;
mod tokenizer;
pub mod types;

// Headless agent-facing CLI (the `ajisai` bin target). Native-only: it is
// host-adapter plumbing over the same interpreter the WASM bindings wrap.
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
mod tensor_operation_tests;

#[cfg(test)]
mod json_io_tests;

#[cfg(test)]
mod json_semantic_role_tests;

#[cfg(test)]
mod conformance_tests;

#[cfg(test)]
mod vent_flow_canonical_tests;

#[cfg(test)]
mod stack_render_tests;

#[cfg(test)]
mod role_ownership_tests;
