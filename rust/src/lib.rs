mod builtins;
mod error;
pub mod interpreter;
mod tokenizer;
pub mod types;
#[path = "wasm-interpreter-bindings.rs"]
mod wasm_interpreter_bindings;

pub use wasm_interpreter_bindings::AjisaiInterpreter;

#[cfg(test)]
#[path = "tokenizer-regression-tests.rs"]
mod tokenizer_regression_tests;

#[cfg(test)]
#[path = "tokenizer-regression-tests-2.rs"]
mod tokenizer_regression_tests_2;

#[cfg(test)]
#[path = "arithmetic-operation-tests.rs"]
mod arithmetic_operation_tests;

#[cfg(test)]
#[path = "dimension-limit-tests.rs"]
mod dimension_limit_tests;

#[cfg(test)]
#[path = "tensor-operation-tests.rs"]
mod tensor_operation_tests;

#[cfg(test)]
#[path = "json-io-tests.rs"]
mod json_io_tests;
