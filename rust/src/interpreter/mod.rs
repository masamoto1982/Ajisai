pub mod arithmetic;
pub mod audio;
pub mod cast;
#[path = "child-runtime.rs"]
pub mod child_runtime;
pub mod comparison;
#[path = "compiled-plan.rs"]
pub mod compiled_plan;
pub mod control;
#[path = "control-cond.rs"]
pub mod control_cond;
pub mod datetime;
pub mod epoch;
#[path = "execute-def.rs"]
pub mod execute_def;
#[path = "execute-del.rs"]
pub mod execute_del;
#[path = "execute-lookup.rs"]
pub mod execute_lookup;
#[path = "execution_plan_set.rs"]
pub mod execution_plan_set;
pub mod hash;
#[path = "higher-order-operations.rs"]
pub mod higher_order;
#[path = "higher-order-fold-operations.rs"]
pub mod higher_order_fold;
pub mod interval_ops;
pub mod io;
pub mod json;
pub mod logic;
pub mod modules;
#[path = "naming-convention-checker.rs"]
pub(crate) mod naming_convention_checker;
#[path = "optimization-hooks.rs"]
pub(crate) mod optimization_hooks;
#[path = "quantized-block.rs"]
pub mod quantized_block;
pub mod random;
#[path = "redundancy-budget.rs"]
pub mod redundancy_budget;
#[path = "redundancy-layer.rs"]
pub mod redundancy_layer;
#[path = "resolve-cache.rs"]
mod resolve_cache;
#[path = "shadow-validation.rs"]
mod shadow_validation;
#[path = "simd-vector-operations.rs"]
pub(crate) mod simd_ops;
pub mod sort;
#[path = "tensor-shape-commands.rs"]
pub mod tensor_cmds;
#[path = "tensor-shape-operations.rs"]
pub mod tensor_ops;
#[path = "value-extraction-helpers.rs"]
pub(crate) mod value_extraction_helpers;
#[path = "vector-execution-operations.rs"]
pub mod vector_exec;
pub mod vector_ops;

#[path = "interpreter-core.rs"]
pub mod interpreter_core;

#[path = "resolve-word.rs"]
mod resolve_word;

#[path = "execution-loop.rs"]
mod execution_loop;

#[path = "deprecated-core-aliases.rs"]
mod deprecated_core_aliases;
#[path = "execute-builtin.rs"]
mod execute_builtin;

#[cfg(test)]
#[path = "child-runtime-tests.rs"]
mod child_runtime_tests;
#[cfg(test)]
#[path = "control-cond-tests.rs"]
mod control_cond_tests;
#[cfg(test)]
#[path = "control-exec-eval-tests.rs"]
mod control_exec_eval_tests;
#[cfg(test)]
#[path = "datetime-tests.rs"]
mod datetime_tests;
#[cfg(test)]
#[path = "dictionary-operation-tests.rs"]
mod dictionary_operation_tests;
#[cfg(test)]
#[path = "dictionary-resolution-tests.rs"]
mod dictionary_resolution_tests;
#[cfg(test)]
#[path = "dictionary-tier-tests.rs"]
mod dictionary_tier_tests;
#[cfg(test)]
#[path = "hash-tests.rs"]
mod hash_tests;
#[cfg(test)]
#[path = "higher-order-fold-tests.rs"]
mod higher_order_fold_tests;
#[cfg(test)]
#[path = "higher-order-operations-mcdc-tests.rs"]
mod higher_order_operations_mcdc_tests;
#[cfg(test)]
#[path = "interpreter-definition-tests.rs"]
mod interpreter_definition_tests;
#[cfg(test)]
#[path = "interpreter-execution-tests.rs"]
mod interpreter_execution_tests;
#[cfg(test)]
#[path = "interpreter-mode-tests.rs"]
mod interpreter_mode_tests;

pub use interpreter_core::*;

pub use crate::types::WordDefinition;

pub use compiled_plan::{
    compile_word_definition, execute_compiled_plan, is_plan_valid, CompiledLine, CompiledOp,
    CompiledPlan,
};
pub use epoch::EpochSnapshot;
pub use quantized_block::{
    is_quantizable_block, quantize_code_block, QuantizedArity, QuantizedBlock, QuantizedPurity,
};
pub use redundancy_budget::{DegradationPolicy, FailureHistory, RedundancyBudget};
pub use redundancy_layer::{select_degradation_policy, RedundancyCheckpoint};

#[cfg(test)]
#[path = "compiled-plan-tests.rs"]
mod compiled_plan_tests;
#[cfg(test)]
#[path = "core-word-canonicalization-tests.rs"]
mod core_word_canonicalization_tests;
#[cfg(test)]
#[path = "differential-tests.rs"]
mod differential_tests;
#[cfg(test)]
#[path = "fast-guarded-tests.rs"]
mod fast_guarded_tests;
#[cfg(test)]
#[path = "perf-regression-tests.rs"]
mod perf_regression_tests;
#[cfg(test)]
#[path = "quantized-block-tests.rs"]
mod quantized_block_tests;

#[cfg(test)]
#[path = "redundancy-layer-tests.rs"]
mod redundancy_layer_tests;
