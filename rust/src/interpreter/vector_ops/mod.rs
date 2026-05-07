

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

pub(crate) fn extract_vector_elements(val: &Value) -> Vec<Value> {
    match &val.data {
        ValueData::Vector(children) | ValueData::Record { pairs: children, .. } => {
            children.as_ref().clone()
        }
        ValueData::Tensor { .. } => {
            let n = val.len();
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                out.push(val.child(i).expect("Tensor child index in 0..len must be valid"));
            }
            out
        }
        _ => Vec::new(),
    }
}
