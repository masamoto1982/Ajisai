// rust/src/lib.rs

mod error;
mod types;
mod tokenizer;
mod interpreter;
mod builtins;
mod wasm_api;

// `pub use` に `#[wasm_bindgen]` は適用できないため削除。
// `AjisaiInterpreter` 構造体自体が `wasm_api.rs` の中で `#[wasm_bindgen]` されているため、
// この `use` を介して正しくエクスポートされます。
pub use wasm_api::AjisaiInterpreter;

#[cfg(test)]
mod test_tokenizer;
