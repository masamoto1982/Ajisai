// rust/src/wasm_api.rs

use crate::interpreter::Interpreter;
use serde::Serialize;
use wasm_bindgen::prelude::*;
use crate::types::{Value, ValueType};

// --- Wasm <-> JS 間のデータ構造 ---

/// 実行結果をJSに渡すための構造体
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EvalResult {
    output: Vec<String>,
    error: Option<String>,
    stack: Vec<String>,  // スタックの状態も返す
}

// --- Wasm エントリーポイント ---

#[wasm_bindgen]
pub struct WasmApi {
    interpreter: Interpreter,
}

#[wasm_bindgen]
impl WasmApi {
    /// WasmApiを初期化します
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // パニック時にJSのconsole.error()に詳細を出力
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        WasmApi {
            interpreter: Interpreter::new(),
        }
    }

    /// Ajisaiコードを1行実行します（REPL用：自動BLOOM）
    #[wasm_bindgen]
    pub fn eval(&mut self, code: &str) -> Result<JsValue, JsValue> {
        let result = self.interpreter.eval_interactive(code);
        
        let (output, error_str) = match result {
            Ok(output_lines) => (output_lines, None),
            Err(e) => (Vec::new(), Some(e.to_string())),
        };

        // スタックの状態を取得
        let stack_state = self.get_stack_state();

        let eval_result = EvalResult {
            output,
            error: error_str,
            stack: stack_state,
        };

        serde_wasm_bindgen::to_value(&eval_result).map_err(|e| e.to_string().into())
    }

    /// スタックの状態を文字列のベクターとして取得
    fn get_stack_state(&self) -> Vec<String> {
        self.interpreter.stack
            .iter()
            .map(|v| format!("{}", v))
            .collect()
    }

    /// スタックをクリア
    #[wasm_bindgen(js_name = clearStack)]
    pub fn clear_stack(&mut self) {
        self.interpreter.stack.clear();
    }

    /// 現在のスタックを取得
    #[wasm_bindgen(js_name = getStack)]
    pub fn get_stack(&self) -> Result<JsValue, JsValue> {
        let stack_state = self.get_stack_state();
        serde_wasm_bindgen::to_value(&stack_state).map_err(|e| e.to_string().into())
    }

    /// 辞書の内容を取得
    #[wasm_bindgen(js_name = getDictionary)]
    pub fn get_dictionary(&self) -> Result<JsValue, JsValue> {
        let dict: Vec<(String, String)> = self.interpreter.dictionary
            .iter()
            .map(|(name, def)| {
                let desc = if def.is_builtin {
                    def.description.as_deref().unwrap_or("Built-in word")
                } else {
                    def.description.as_deref().unwrap_or("Custom word")
                };
                (name.clone(), desc.to_string())
            })
            .collect();
        
        serde_wasm_bindgen::to_value(&dict).map_err(|e| e.to_string().into())
    }
}

impl Default for WasmApi {
    fn default() -> Self {
        Self::new()
    }
}
