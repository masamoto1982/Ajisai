

pub mod position;
pub mod quantity;
pub mod structure;
mod targeting;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_modes;

pub use position::{op_get, op_insert, op_replace, op_remove};
pub use quantity::{op_length, op_take, op_split};
pub use structure::{op_concat, op_reverse, op_range, op_reorder, op_collect};

use crate::types::Value;

/// Materialize the children of an iterable `Value` (Vector / Record / Tensor)
/// into an owned `Vec<Value>`. Non-iterable values produce an empty `Vec`.
///
/// Implemented on top of [`Value::as_vector_view`], which keeps the borrow vs.
/// owned distinction explicit at the helper level.
pub(crate) fn extract_vector_elements(val: &Value) -> Vec<Value> {
    val.as_vector_view()
        .map(|cow| cow.into_owned())
        .unwrap_or_default()
}
