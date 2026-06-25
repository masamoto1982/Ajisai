pub mod algo_ops;
pub mod arithmetic;
pub mod audio;
pub mod cast;
pub mod child_runtime;
pub mod comparison;
pub mod compiled_plan;
pub mod comptime;
pub mod control;
pub mod control_cond;
pub mod datetime;
pub mod debug_diagnosis;
pub mod energy_proxy;
pub mod epoch;
pub mod error_flow_trace;
pub mod execute_def;
pub mod execute_del;
pub mod execute_lookup;
pub mod execution_plan_set;
pub mod hash;
pub mod higher_order;
pub mod higher_order_fold;
pub mod host;
pub mod interval_ops;
pub mod io;
pub mod json;
pub mod logic;
pub mod logic_kleene;
pub mod mass_conservation;
pub mod math_ops;
pub mod modules;
pub(crate) mod naming_convention_checker;
pub mod parallel;
pub mod quantized_block;
pub mod random;
mod resolve_cache;
pub mod serial;
mod shadow_validation;
pub(crate) mod simd_ops;
pub mod sort;
pub mod tensor_cmds;
pub mod tensor_ops;
pub mod time_calendar;
pub mod time_ops;
pub(crate) mod value_extraction_helpers;
pub mod vector_exec;
pub mod vector_ops;
mod word_identity;

pub mod interpreter_core;

mod resolve_word;

mod execution_loop;

mod execute_builtin;

#[cfg(test)]
mod algo_ops_tests;
#[cfg(test)]
mod arithmetic_exact_div_tests;
#[cfg(test)]
mod child_runtime_tests;
#[cfg(test)]
mod control_cond_tests;
#[cfg(test)]
mod control_exec_eval_tests;
#[cfg(test)]
mod datetime_tests;
#[cfg(test)]
mod dependents_index_tests;
#[cfg(test)]
mod dictionary_operation_tests;
#[cfg(test)]
mod dictionary_resolution_tests;
#[cfg(test)]
mod dictionary_tier_tests;
#[cfg(test)]
mod energy_proxy_regression_tests;
#[cfg(test)]
mod error_flow_trace_tests;
#[cfg(test)]
mod hash_tests;
#[cfg(test)]
mod higher_order_fold_tests;
#[cfg(test)]
mod higher_order_operations_mcdc_tests;
#[cfg(test)]
mod interpreter_definition_tests;
#[cfg(test)]
mod interpreter_execution_tests;
#[cfg(test)]
mod interpreter_mode_tests;
#[cfg(test)]
mod math_ops_tests;
#[cfg(test)]
mod module_catalog_tests;
#[cfg(test)]
mod module_unimport_tests;
#[cfg(test)]
mod nil_conformance_tests;
#[cfg(test)]
mod nil_reason_tests;
#[cfg(test)]
mod nil_unknown_firewall_tests;

pub use interpreter_core::*;

pub use host::{
    default_host_env, DefaultHostEnv, DeterministicHostEnv, HostCapability, HostEffect, HostEnv,
};

pub use crate::types::WordDefinition;

pub use compiled_plan::{
    compile_word_definition, execute_compiled_plan, is_plan_valid, CompiledLine, CompiledOp,
    CompiledPlan,
};
pub use epoch::EpochSnapshot;
pub use quantized_block::{
    is_quantizable_block, quantize_code_block, QuantizedArity, QuantizedBlock, QuantizedPurity,
};

#[cfg(test)]
mod compiled_clause_tests;
#[cfg(test)]
mod compiled_plan_tests;
#[cfg(test)]
mod cond_dispatch_tests;
#[cfg(test)]
mod core_word_canonicalization_tests;
#[cfg(test)]
mod differential_tests;
#[cfg(test)]
mod fast_guarded_tests;
#[cfg(test)]
mod perf_regression_tests;
#[cfg(test)]
mod quantized_block_tests;
#[cfg(test)]
mod scalar_fastpath_tests;
#[cfg(test)]
mod tail_call_tests;
#[cfg(test)]
mod vector_literal_tests;
