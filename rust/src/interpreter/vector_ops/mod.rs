









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

use crate::types::{Value, ValueData};

pub(crate) fn extract_vector_elements(val: &Value) -> &[Value] {
    match &val.data {
        ValueData::Vector(children) | ValueData::Record { pairs: children, .. } => children,
        _ => &[],
    }
}
