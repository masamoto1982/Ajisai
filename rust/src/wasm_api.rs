use crate::interpreter::Interpreter;
use serde::Serialize;
use wasm_bindgen::prelude::*;

// --- Wasm <-> JS 間のデータ構造 ---

/// オペレータ（ワード）の情報をJSに渡すための構造体
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OperatorInfo {
    name: String,
    signature: String,
}

/// オペランド（VStack）の情報をJSに渡すための構造体
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OperandInfo {
    name: String,
    content: String,
}

/// 実行結果をJSに渡すための構造体
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EvalResult {
    output: Vec<String>,
    error: Option<String>,
    // GUI更新のために、最新のOperand状態も一緒に返す
    operands: Vec<OperandInfo>,
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

    /// Ajisaiコードを1行実行します
    #[wasm_bindgen]
    pub fn eval(&mut self, code: &str) -> Result<JsValue, JsValue> {
        let result = self.interpreter.eval(code);
        
        let (output, error_str) = match result {
            Ok(output_lines) => (output_lines, None),
            Err(e) => (Vec::new(), Some(e.to_string())),
        };

        // 実行結果に関わらず、現在のOperand状態を取得してGUIに返す
        let operands = self.get_operands_internal();

        let eval_result = EvalResult {
            output,
            error: error_str,
            operands,
        };

        serde_wasm_bindgen::to_value(&eval_result).map_err(|e| e.to_string().into())
    }

    /// GUI表示用のOperator（辞書）リストを取得します
    #[wasm_bindgen(js_name = getOperators)]
    pub fn get_operators(&self) -> Result<JsValue, JsValue> {
        let op_store = self.interpreter.get_operators_store();
        let ops = op_store.read().unwrap().get_all_pairs(); // (String, Arc<Operator>)

        let op_info: Vec<OperatorInfo> = ops
            .into_iter()
            .map(|(name, op)| OperatorInfo {
                name,
                signature: op.signature(),
            })
            .collect();

        serde_wasm_bindgen::to_value(&op_info).map_err(|e| e.to_string().into())
    }

    /// GUI表示用のOperand（領域）リストを取得します
    #[wasm_bindgen(js_name = getOperands)]
    pub fn get_operands(&self) -> Result<JsValue, JsValue> {
        let operands = self.get_operands_internal();
        serde_wasm_bindgen::to_value(&operands).map_err(|e| e.to_string().into())
    }
    
    /// `get_operands` の内部実装
    fn get_operands_internal(&self) -> Vec<OperandInfo> {
        let operand_store = self.interpreter.get_operands_store();
        let ops = operand_store.read().unwrap().get_all_pairs(); // (String, Arc<RwLock<VStack>>)

        ops.into_iter()
            .map(|(name, v_stack)| OperandInfo {
                name,
                content: v_stack.read().unwrap().to_string(),
            })
            .collect()
    }
}

// wasm_api.rs のデフォルト実装
impl Default for WasmApi {
    fn default() -> Self {
        Self::new()
    }
}
