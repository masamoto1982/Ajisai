pub mod arithmetic;
pub mod audio;
pub mod cast;
pub mod comparison;
pub mod control;
#[path = "control-cond.rs"]
pub mod control_cond;
pub mod datetime;
#[path = "execute-def.rs"]
pub mod execute_def;
#[path = "execute-del.rs"]
pub mod execute_del;
#[path = "execute-lookup.rs"]
pub mod execute_lookup;
pub mod hash;
#[path = "naming-convention-checker.rs"]
pub(crate) mod naming_convention_checker;
#[path = "value-extraction-helpers.rs"]
pub(crate) mod value_extraction_helpers;
#[path = "higher-order-operations.rs"]
pub mod higher_order;
#[path = "higher-order-fold-operations.rs"]
pub mod higher_order_fold;
pub mod io;
pub mod json;
pub mod logic;
pub mod modules;
pub mod random;
#[path = "simd-vector-operations.rs"]
pub(crate) mod simd_ops;
pub mod sort;
#[path = "tensor-shape-operations.rs"]
pub mod tensor_ops;
#[path = "tensor-shape-commands.rs"]
pub mod tensor_cmds;
#[path = "vector-execution-operations.rs"]
pub mod vector_exec;
pub mod vector_ops;

// ── Interpreter core (struct, enums, initialization) ────────────
#[path = "interpreter-core.rs"]
pub mod interpreter_core;

// ── Word resolution and dependency management ───────────────────
#[path = "resolve-word.rs"]
mod resolve_word;

// ── Main execution loop and guard structures ────────────────────
#[path = "execution-loop.rs"]
mod execution_loop;

// ── Builtin word dispatch ───────────────────────────────────────
#[path = "execute-builtin.rs"]
mod execute_builtin;

// ── Tests ───────────────────────────────────────────────────────
#[cfg(test)]
#[path = "interpreter-execution-tests.rs"]
mod interpreter_execution_tests;
#[cfg(test)]
#[path = "interpreter-definition-tests.rs"]
mod interpreter_definition_tests;
#[cfg(test)]
#[path = "interpreter-mode-tests.rs"]
mod interpreter_mode_tests;
#[cfg(test)]
#[path = "dictionary-operation-tests.rs"]
mod dictionary_operation_tests;
#[cfg(test)]
#[path = "dictionary-resolution-tests.rs"]
mod dictionary_resolution_tests;
#[cfg(test)]
#[path = "control-exec-eval-tests.rs"]
mod control_exec_eval_tests;
#[cfg(test)]
#[path = "control-cond-tests.rs"]
mod control_cond_tests;
#[cfg(test)]
#[path = "datetime-tests.rs"]
mod datetime_tests;
#[cfg(test)]
#[path = "hash-tests.rs"]
mod hash_tests;
#[cfg(test)]
#[path = "higher-order-fold-tests.rs"]
mod higher_order_fold_tests;

pub use interpreter_core::*;

// Re-export types that submodules import via `crate::interpreter::`
pub use crate::types::WordDefinition;
