// rust/src/interpreter/vector_ops/mod.rs
//
// 【責務】
// ベクタおよびスタックに対する位置・構造操作を実装する。
// 0オリジンの位置指定操作（GET/INSERT/REPLACE/REMOVE）、
// 1オリジンの量指定操作（LENGTH/TAKE/SPLIT）、
// およびベクタ構造操作（CONCAT/REVERSE/LEVEL）を提供する。
//
// 統一Value宇宙アーキテクチャ版

pub mod position;
pub mod quantity;
pub mod structure;

#[cfg(test)]
mod tests;

pub use position::{op_get, op_insert, op_replace, op_remove};
pub use quantity::{op_length, op_take, op_split};
pub use structure::{op_concat, op_reverse, op_range, op_reorder, op_collect};

use crate::types::{Value, ValueData};

pub(crate) fn reconstruct_vector_elements(val: &Value) -> Vec<Value> {
    match &val.data {
        ValueData::Vector(children) => children.clone(),
        ValueData::Scalar(_) => vec![val.clone()],
        ValueData::Nil => vec![],
        ValueData::CodeBlock(_) => vec![val.clone()],
    }
}
