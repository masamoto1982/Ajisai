mod builtins;
pub mod core_word_aliases;
pub mod coreword_registry;
pub mod elastic;
mod error;
pub mod interpreter;
pub mod semantic;
pub mod surface_forms;
mod tokenizer;
pub mod types;
mod wasm_interpreter_bindings;

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
mod tensor_operation_tests;

#[cfg(test)]
mod json_io_tests;
