mod all;
mod any;
mod common;
mod count;
mod fast_kernels;
mod filter;
mod hedged;
mod map;
mod memo;
#[cfg(test)]
mod memo_tests;
mod runners;

pub(crate) use common::{execute_executable_code, extract_executable_code, ExecutableCode};
pub(crate) use hedged::execute_hedged_fold_kernel;
#[cfg(test)]
pub(crate) use hedged::{execute_hedged_map_kernel, execute_hedged_predicate_kernel};

pub use all::op_all;
pub use any::op_any;
pub use count::op_count;
pub use filter::op_filter;
pub use map::op_map;

use crate::error::Result;
use crate::interpreter::quantized_block::QuantizedBlock;
use crate::interpreter::Interpreter;
use crate::types::Value;

/// Returns true when the interpreter is in a hedged execution mode where the
/// HOF bulk fast paths should be skipped to keep race-validation events
/// observable.
#[inline]
pub(crate) fn hedged_mode_active(interp: &Interpreter) -> bool {
    hedged::hedged_mode(interp.elastic_mode())
}

/// Public-to-the-crate wrapper around the fast-kernel bulk fold so the
/// `higher_order_fold` module (sibling, not child) can reach it.
pub(crate) fn try_bulk_quantized_fold_pub(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    init: &Value,
    target: &Value,
) -> Option<Result<Value>> {
    fast_kernels::try_bulk_quantized_fold(interp, qb, init, target)
}

/// Public-to-the-crate wrapper around the fast-kernel bulk predicate.
pub(crate) fn try_bulk_quantized_predicate_pub(
    interp: &mut Interpreter,
    qb: &QuantizedBlock,
    target: &Value,
) -> Option<fast_kernels::BulkPredicateResult> {
    fast_kernels::try_bulk_quantized_predicate(interp, qb, target)
}
