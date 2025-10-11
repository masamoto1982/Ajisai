// rust/src/lib.rs

use wasm_bindgen::prelude::*;

mod types;
mod tokenizer;
mod interpreter;
mod builtins;
mod wasm_api;

#[wasm_bindgen]
pub use wasm_api::AjisaiInterpreter;
