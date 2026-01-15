// rust/src/wasm_api.rs
//
// 統一分数アーキテクチャ版のWebAssembly API
//
// すべての値は Vec<Fraction> として表現される。
// DisplayHint は表示目的のみに使用される。

use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
use crate::interpreter::Interpreter;
use crate::types::{Value, DisplayHint, Token, ExecutionLine};
use crate::types::fraction::Fraction;
use num_bigint::BigInt;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use crate::interpreter;
use crate::tokenizer;
use crate::builtins;

// ============================================================================
// ヘルパー関数
// ============================================================================

/// 値が文字列として扱えるかチェック
fn is_string_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::String && !val.data.is_empty()
}

/// 値が真偽値として扱えるかチェック
fn is_boolean_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::Boolean && val.data.len() == 1
}

/// 値が数値として扱えるかチェック
fn is_number_value(val: &Value) -> bool {
    matches!(val.display_hint, DisplayHint::Number | DisplayHint::Auto) && val.data.len() == 1
}

/// 値がDateTimeとして扱えるかチェック
fn is_datetime_value(val: &Value) -> bool {
    val.display_hint == DisplayHint::DateTime && val.data.len() == 1
}

/// 値がベクタ（複数要素）かチェック
fn is_vector_value(val: &Value) -> bool {
    val.data.len() > 1 || !val.shape.is_empty()
}

/// 値を文字列として解釈する
///
/// UTF-8バイト列として保存されたデータを文字列に復元する。
/// 各Fractionは0-255のバイト値として解釈される。
fn value_as_string(val: &Value) -> String {
    let bytes: Vec<u8> = val.data.iter()
        .filter_map(|f| {
            f.to_i64().and_then(|n| {
                if n >= 0 && n <= 255 {
                    Some(n as u8)
                } else {
                    None
                }
            })
        })
        .collect();

    String::from_utf8_lossy(&bytes).into_owned()
}

/// 形状ベクタからブラケット構造を生成
/// shape: [dim1] → { } × dim1
/// shape: [dim1, dim2] → { ( ) × dim2 } × dim1
/// shape: [dim1, dim2, dim3] → { ( [ ] × dim3 ) × dim2 } × dim1
fn generate_bracket_structure_from_shape(shape: &[usize]) -> String {
    match shape.len() {
        1 => {
            // 1D: { } を dim1 個生成（スタックに別々に配置）
            (0..shape[0]).map(|_| "{ }").collect::<Vec<_>>().join(" ")
        }
        2 => {
            // 2D: { ( ) × dim2 } を dim1 個生成
            let inner = (0..shape[1]).map(|_| "( )").collect::<Vec<_>>().join(" ");
            let one_element = format!("{{ {} }}", inner);
            (0..shape[0]).map(|_| one_element.as_str()).collect::<Vec<_>>().join(" ")
        }
        3 => {
            // 3D: { ( [ ] × dim3 ) × dim2 } を dim1 個生成
            let innermost = (0..shape[2]).map(|_| "[ ]").collect::<Vec<_>>().join(" ");
            let middle = (0..shape[1]).map(|_| format!("( {} )", innermost)).collect::<Vec<_>>().join(" ");
            let one_element = format!("{{ {} }}", middle);
            (0..shape[0]).map(|_| one_element.as_str()).collect::<Vec<_>>().join(" ")
        }
        _ => "{ }".to_string() // フォールバック
    }
}

#[derive(Serialize, Deserialize)]
struct CustomWordData {
    name: String,
    definition: String,
    description: Option<String>,
}

#[wasm_bindgen]
pub struct AjisaiInterpreter {
    interpreter: Interpreter,
    step_tokens: Vec<Token>,
    step_position: usize,
    step_mode: bool,
    current_step_code: String,
}

#[wasm_bindgen]
impl AjisaiInterpreter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        AjisaiInterpreter {
            interpreter: Interpreter::new(),
            step_tokens: Vec::new(),
            step_position: 0,
            step_mode: false,
            current_step_code: String::new(),
        }
    }

    #[wasm_bindgen]
    pub async fn execute(&mut self, code: &str) -> Result<JsValue, JsValue> {
        self.interpreter.definition_to_load = None;
        let obj = js_sys::Object::new();

        // 入力支援ワードの検出（末尾にあるかチェック）
        let trimmed = code.trim();
        let upper_code = trimmed.to_uppercase();

        // FRAME ワードの検出
        if upper_code.ends_with("FRAME") {
            // ワードの前に空白があるか、完全一致かを確認
            let prefix_len = upper_code.len() - 5; // "FRAME".len() == 5
            let is_valid = if prefix_len == 0 {
                true // 完全一致
            } else {
                // ワードの前が空白文字であることを確認
                upper_code.chars().nth(prefix_len - 1).map_or(false, |c| c.is_whitespace())
            };

            if is_valid {
                // 入力支援ワードより前の部分があれば先に実行
                if prefix_len > 0 {
                    let prefix_code = &trimmed[..prefix_len].trim();
                    if !prefix_code.is_empty() {
                        if let Err(e) = self.interpreter.execute(prefix_code).await {
                            js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                            js_sys::Reflect::set(&obj, &"message".into(), &e.to_string().into()).unwrap();
                            js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
                            return Ok(obj.into());
                        }
                    }
                }

                // スタックトップから形状ベクタを取得（統一分数アーキテクチャ）
                let shape = if let Some(top) = self.interpreter.stack.last() {
                    if is_vector_value(top) && !top.is_nil() {
                        // ベクタ内の全要素が正の整数（1〜100）かチェック
                        let mut dims = Vec::new();
                        let mut valid = top.data.len() >= 1 && top.data.len() <= 3;
                        if valid {
                            for f in &top.data {
                                if let Some(val) = f.as_usize() {
                                    if val >= 1 && val <= 100 {
                                        dims.push(val);
                                    } else {
                                        valid = false;
                                        break;
                                    }
                                } else {
                                    valid = false;
                                    break;
                                }
                            }
                        }
                        if valid { Some(dims) } else { None }
                    } else {
                        None
                    }
                } else {
                    None
                };

                // 形状ベクタが指定されていた場合、スタックから消費してブラケット構造を生成
                if let Some(shape_vec) = shape {
                    self.interpreter.stack.pop();
                    let helper_text = generate_bracket_structure_from_shape(&shape_vec);
                    js_sys::Reflect::set(&obj, &"inputHelper".into(), &helper_text.into()).unwrap();
                    js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                    js_sys::Reflect::set(&obj, &"stack".into(), &self.get_stack()).unwrap();
                    js_sys::Reflect::set(&obj, &"customWords".into(), &self.get_custom_words_for_state()).unwrap();
                    return Ok(obj.into());
                } else {
                    // 形状ベクタがない場合はエラー
                    js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                    js_sys::Reflect::set(&obj, &"message".into(), &"FRAME requires a shape vector [ dim1 dim2 ... ] (1-3 dimensions, values 1-100)".into()).unwrap();
                    js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
                    return Ok(obj.into());
                }
            }
        }

        match self.interpreter.execute(code).await {
            Ok(()) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                let output = self.interpreter.get_output();
                js_sys::Reflect::set(&obj, &"output".into(), &output.clone().into()).unwrap();
                js_sys::Reflect::set(&obj, &"stack".into(), &self.get_stack()).unwrap();
                js_sys::Reflect::set(&obj, &"customWords".into(), &self.get_custom_words_for_state()).unwrap();

                if let Some(def_str) = self.interpreter.definition_to_load.take() {
                    js_sys::Reflect::set(&obj, &"definition_to_load".into(), &def_str.into()).unwrap();
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &error_msg.into()).unwrap();
                js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
            }
        }
        Ok(obj.into())
    }

    #[wasm_bindgen]
    pub fn execute_step(&mut self, code: &str) -> JsValue {
        let obj = js_sys::Object::new();

        if !self.step_mode || code != self.current_step_code {
            self.step_mode = true;
            self.step_position = 0;
            self.current_step_code = code.to_string();

            let custom_word_names: std::collections::HashSet<String> = self.interpreter.dictionary.iter()
                .filter(|(_, def)| !def.is_builtin)
                .map(|(name, _)| name.clone())
                .collect();

            match tokenizer::tokenize_with_custom_words(code, &custom_word_names) {
                Ok(tokens) => { self.step_tokens = tokens; }
                Err(e) => {
                    self.step_mode = false;
                    js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                    js_sys::Reflect::set(&obj, &"message".into(), &format!("Tokenization error: {}", e).into()).unwrap();
                    js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
                    return obj.into();
                }
            }
        }

        if self.step_position >= self.step_tokens.len() {
            self.step_mode = false;
            js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
            js_sys::Reflect::set(&obj, &"output".into(), &"Step execution completed".into()).unwrap();
            js_sys::Reflect::set(&obj, &"hasMore".into(), &false.into()).unwrap();
            return obj.into();
        }

        let token = self.step_tokens[self.step_position].clone();

        // トークンを1つの行として実行
        let line = ExecutionLine {
            body_tokens: vec![token],
        };
        let result = self.interpreter.execute_guard_structure_sync(&[line]);

        match result {
            Ok(()) => {
                let output = self.interpreter.get_output();
                self.step_position += 1;
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &output.into()).unwrap();
                js_sys::Reflect::set(&obj, &"hasMore".into(), &(self.step_position < self.step_tokens.len()).into()).unwrap();
                js_sys::Reflect::set(&obj, &"position".into(), &(self.step_position as u32).into()).unwrap();
                js_sys::Reflect::set(&obj, &"total".into(), &(self.step_tokens.len() as u32).into()).unwrap();
                js_sys::Reflect::set(&obj, &"stack".into(), &self.get_stack()).unwrap();
                js_sys::Reflect::set(&obj, &"customWords".into(), &self.get_custom_words_for_state()).unwrap();
            }
            Err(e) => {
                self.step_mode = false;
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &e.to_string().into()).unwrap();
                js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
                js_sys::Reflect::set(&obj, &"hasMore".into(), &false.into()).unwrap();
            }
        }

        obj.into()
    }

    #[wasm_bindgen]
    pub fn reset(&mut self) -> JsValue {
        let obj = js_sys::Object::new();

        self.step_mode = false;
        self.step_tokens.clear();
        self.step_position = 0;
        self.current_step_code.clear();

        match self.interpreter.execute_reset() {
            Ok(()) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"OK".into()).unwrap();
                js_sys::Reflect::set(&obj, &"output".into(), &"System reinitialized.".into()).unwrap();
                js_sys::Reflect::set(&obj, &"stack".into(), &self.get_stack()).unwrap();
                js_sys::Reflect::set(&obj, &"customWords".into(), &self.get_custom_words_for_state()).unwrap();
            }
            Err(e) => {
                js_sys::Reflect::set(&obj, &"status".into(), &"ERROR".into()).unwrap();
                js_sys::Reflect::set(&obj, &"message".into(), &e.to_string().into()).unwrap();
                js_sys::Reflect::set(&obj, &"error".into(), &true.into()).unwrap();
            }
        }
        obj.into()
    }

    #[wasm_bindgen]
    pub fn get_stack(&self) -> JsValue {
        let js_array = js_sys::Array::new();
        for value in self.interpreter.get_stack() {
            js_array.push(&value_to_js_value(value));
        }
        js_array.into()
    }

    #[wasm_bindgen]
    pub fn get_custom_words_info(&self) -> JsValue {
        let js_array = js_sys::Array::new();

        for (name, def) in self.interpreter.dictionary.iter() {
            if def.is_builtin { continue; }

            let is_protected = self.interpreter.dependents.get(name)
                .map_or(false, |deps| !deps.is_empty());

            let item = js_sys::Array::new();
            item.push(&name.clone().into());
            item.push(&def.description.clone().map(JsValue::from).unwrap_or(JsValue::NULL));
            item.push(&is_protected.into());

            js_array.push(&item);
        }

        js_array.into()
    }

    fn get_custom_words_for_state(&self) -> JsValue {
        let words_info: Vec<CustomWordData> = self.interpreter.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| {
                CustomWordData {
                    name: name.clone(),
                    definition: self.interpreter.get_word_definition_tokens(name).unwrap_or_default(),
                    description: def.description.clone(),
                }
            })
            .collect();
        to_value(&words_info).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn get_builtin_words_info(&self) -> JsValue {
        to_value(&builtins::get_builtin_definitions()).unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn get_word_definition(&self, name: &str) -> JsValue {
        let upper_name = name.to_uppercase();
        self.interpreter.get_word_definition_tokens(&upper_name)
            .map(|def| JsValue::from_str(&def))
            .unwrap_or(JsValue::NULL)
    }

    #[wasm_bindgen]
    pub fn restore_stack(&mut self, stack_js: JsValue) -> Result<(), String> {
        let js_array = js_sys::Array::from(&stack_js);
        let mut stack = Vec::new();
        for i in 0..js_array.length() {
            stack.push(js_value_to_value(js_array.get(i))?);
        }
        self.interpreter.set_stack(stack);
        Ok(())
    }

    #[wasm_bindgen]
    pub fn restore_custom_words(&mut self, words_js: JsValue) -> Result<(), String> {
        let words: Vec<CustomWordData> = serde_wasm_bindgen::from_value(words_js)
            .map_err(|e| format!("Failed to deserialize words: {}", e))?;

        let custom_word_names: std::collections::HashSet<String> = words.iter()
            .map(|w| w.name.to_uppercase())
            .collect();

        for word in words {
            let tokens = tokenizer::tokenize_with_custom_words(&word.definition, &custom_word_names)
                .map_err(|e| format!("Failed to tokenize definition for {}: {}", word.name, e))?;

            interpreter::dictionary::op_def_inner(&mut self.interpreter, &word.name, &tokens, word.description.clone())
                .map_err(|e| format!("Failed to restore word {}: {}", word.name, e))?;
        }

        self.interpreter.rebuild_dependencies().map_err(|e| e.to_string())?;

        // 復元操作中に溜まった "Defined word: ..." メッセージをクリア
        // これらは裏方の処理であり、ユーザーに表示する必要がない
        let _ = self.interpreter.get_output();

        Ok(())
    }
}

fn js_value_to_value(js_val: JsValue) -> Result<Value, String> {
    let obj = js_sys::Object::from(js_val);
    let type_str = js_sys::Reflect::get(&obj, &"type".into())
        .map_err(|_| "Failed to get 'type' property".to_string())?
        .as_string().ok_or("Type not string")?;
    let value_js = js_sys::Reflect::get(&obj, &"value".into())
        .map_err(|_| "Failed to get 'value' property".to_string())?;

    match type_str.as_str() {
        "number" => {
            let num_obj = js_sys::Object::from(value_js);
            let num_str = js_sys::Reflect::get(&num_obj, &"numerator".into()).map_err(|_| "No numerator".to_string())?.as_string().ok_or("Numerator not string")?;
            let den_str = js_sys::Reflect::get(&num_obj, &"denominator".into()).map_err(|_| "No denominator".to_string())?.as_string().ok_or("Denominator not string")?;
            let fraction = Fraction::new(
                BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                BigInt::from_str(&den_str).map_err(|e| e.to_string())?
            );
            Ok(Value::from_fraction(fraction))
        },
        "datetime" => {
            // DateTime型もNumber型と同じ構造（分数）だが、タイプが異なる
            let num_obj = js_sys::Object::from(value_js);
            let num_str = js_sys::Reflect::get(&num_obj, &"numerator".into()).map_err(|_| "No numerator".to_string())?.as_string().ok_or("Numerator not string")?;
            let den_str = js_sys::Reflect::get(&num_obj, &"denominator".into()).map_err(|_| "No denominator".to_string())?.as_string().ok_or("Denominator not string")?;
            let fraction = Fraction::new(
                BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                BigInt::from_str(&den_str).map_err(|e| e.to_string())?
            );
            Ok(Value::from_datetime(fraction))
        },
        "string" => {
            let s = value_js.as_string().ok_or("Value not string")?;
            Ok(Value::from_string(&s))
        },
        "boolean" => {
            let b = value_js.as_bool().ok_or("Value not boolean")?;
            Ok(Value::from_bool(b))
        },
        "symbol" => {
            let s = value_js.as_string().ok_or("Value not string")?;
            Ok(Value::from_symbol(&s))
        },
        "vector" => {
            // bracketType は表示層で深さから計算されるため、ここでは無視
            let js_array = js_sys::Array::from(&value_js);
            let mut vec = Vec::new();
            for i in 0..js_array.length() {
                vec.push(js_value_to_value(js_array.get(i))?);
            }
            Ok(Value::from_vector(vec))
        },
        "tensor" => {
            // 多次元配列: shape と data から Value を復元
            let tensor_obj = js_sys::Object::from(value_js);

            // shape を取得
            let shape_js = js_sys::Reflect::get(&tensor_obj, &"shape".into())
                .map_err(|_| "No shape in tensor".to_string())?;
            let shape_array = js_sys::Array::from(&shape_js);
            let mut shape = Vec::new();
            for i in 0..shape_array.length() {
                let dim = shape_array.get(i).as_f64().ok_or("Shape element not number")? as usize;
                shape.push(dim);
            }

            // data を取得
            let data_js = js_sys::Reflect::get(&tensor_obj, &"data".into())
                .map_err(|_| "No data in tensor".to_string())?;
            let data_array = js_sys::Array::from(&data_js);
            let mut data = Vec::new();
            for i in 0..data_array.length() {
                let frac_obj = js_sys::Object::from(data_array.get(i));
                let num_str = js_sys::Reflect::get(&frac_obj, &"numerator".into())
                    .map_err(|_| "No numerator in tensor data".to_string())?
                    .as_string().ok_or("Numerator not string")?;
                let den_str = js_sys::Reflect::get(&frac_obj, &"denominator".into())
                    .map_err(|_| "No denominator in tensor data".to_string())?
                    .as_string().ok_or("Denominator not string")?;
                let fraction = Fraction::new(
                    BigInt::from_str(&num_str).map_err(|e| e.to_string())?,
                    BigInt::from_str(&den_str).map_err(|e| e.to_string())?
                );
                data.push(fraction);
            }

            Ok(Value {
                data,
                display_hint: DisplayHint::Auto,
                shape,
            })
        },
        "nil" => Ok(Value::nil()),
        _ => Err(format!("Unknown type: {}", type_str)),
    }
}

fn value_to_js_value(value: &Value) -> JsValue {
    let obj = js_sys::Object::new();

    // 統一分数アーキテクチャ: 直接データアクセス

    // NILチェック
    if value.is_nil() {
        js_sys::Reflect::set(&obj, &"type".into(), &"nil".into()).unwrap();
        js_sys::Reflect::set(&obj, &"value".into(), &JsValue::NULL).unwrap();
        return obj.into();
    }

    // 文字列は1次元の場合のみ直接文字列として処理
    // 多次元の場合（[ 'てすと' ] など）はtensorとして処理し、ネスト構造を保持
    if value.display_hint == DisplayHint::String && !value.data.is_empty() && value.shape.len() <= 1 {
        js_sys::Reflect::set(&obj, &"type".into(), &"string".into()).unwrap();
        let s = value_as_string(value);
        js_sys::Reflect::set(&obj, &"value".into(), &s.into()).unwrap();
        return obj.into();
    }

    // 多次元配列（shape.len() > 1）の場合はtensorとして送信
    // これにより shape [2, 3, 1] などのネスト構造が保持される
    if value.shape.len() > 1 {
        js_sys::Reflect::set(&obj, &"type".into(), &"tensor".into()).unwrap();

        let tensor_obj = js_sys::Object::new();

        // shape配列を作成
        let shape_array = js_sys::Array::new();
        for &dim in &value.shape {
            shape_array.push(&(dim as u32).into());
        }
        js_sys::Reflect::set(&tensor_obj, &"shape".into(), &shape_array).unwrap();

        // data配列を作成（各分数をオブジェクトとして）
        let data_array = js_sys::Array::new();
        for frac in &value.data {
            let num_obj = js_sys::Object::new();
            js_sys::Reflect::set(&num_obj, &"numerator".into(), &frac.numerator.to_string().into()).unwrap();
            js_sys::Reflect::set(&num_obj, &"denominator".into(), &frac.denominator.to_string().into()).unwrap();
            data_array.push(&num_obj);
        }
        js_sys::Reflect::set(&tensor_obj, &"data".into(), &data_array).unwrap();

        // display_hintを追加（文字列の場合、JavaScript側で文字列として表示するため）
        let hint_str = match value.display_hint {
            DisplayHint::Nil => "nil",
            DisplayHint::String => "string",
            DisplayHint::Boolean => "boolean",
            DisplayHint::DateTime => "datetime",
            DisplayHint::Number => "number",
            DisplayHint::Auto => "auto",
        };
        js_sys::Reflect::set(&tensor_obj, &"displayHint".into(), &hint_str.into()).unwrap();

        js_sys::Reflect::set(&obj, &"value".into(), &tensor_obj).unwrap();
        return obj.into();
    }

    // DisplayHintに基づいて型を決定
    let type_str = if is_datetime_value(value) {
        "datetime"
    } else if is_boolean_value(value) {
        "boolean"
    } else if is_string_value(value) {
        "string"
    } else if is_number_value(value) {
        "number"
    } else if is_vector_value(value) {
        "vector"
    } else {
        "number" // フォールバック
    };

    js_sys::Reflect::set(&obj, &"type".into(), &type_str.into()).unwrap();

    match type_str {
        "number" | "datetime" => {
            let num_obj = js_sys::Object::new();
            js_sys::Reflect::set(&num_obj, &"numerator".into(), &value.data[0].numerator.to_string().into()).unwrap();
            js_sys::Reflect::set(&num_obj, &"denominator".into(), &value.data[0].denominator.to_string().into()).unwrap();
            js_sys::Reflect::set(&obj, &"value".into(), &num_obj).unwrap();
        },
        "string" => {
            let s = value_as_string(value);
            js_sys::Reflect::set(&obj, &"value".into(), &s.into()).unwrap();
        },
        "boolean" => {
            let b = !value.data[0].is_zero();
            js_sys::Reflect::set(&obj, &"value".into(), &b.into()).unwrap();
        },
        "vector" => {
            // 1次元ベクタの場合、各要素を個別のValueとして返す
            let js_array = js_sys::Array::new();
            for frac in &value.data {
                // 各要素のdisplay_hintを継承しつつ、単一要素のValueを作成
                let elem = Value {
                    data: vec![frac.clone()],
                    display_hint: value.display_hint,
                    shape: vec![],
                };
                js_array.push(&value_to_js_value(&elem));
            }
            js_sys::Reflect::set(&obj, &"value".into(), &js_array).unwrap();
        },
        _ => {}
    };

    obj.into()
}

#[cfg(test)]
mod test_input_helper {
    use super::generate_bracket_structure_from_shape;

    #[test]
    fn test_generate_bracket_structure_from_shape() {
        // 1D: { } を生成
        assert_eq!(generate_bracket_structure_from_shape(&[1]), "{ }");
        assert_eq!(generate_bracket_structure_from_shape(&[2]), "{ } { }");
        assert_eq!(generate_bracket_structure_from_shape(&[3]), "{ } { } { }");

        // 2D: { ( ) × dim2 } × dim1 を生成
        assert_eq!(generate_bracket_structure_from_shape(&[1, 1]), "{ ( ) }");
        assert_eq!(generate_bracket_structure_from_shape(&[1, 2]), "{ ( ) ( ) }");
        assert_eq!(generate_bracket_structure_from_shape(&[1, 3]), "{ ( ) ( ) ( ) }");
        assert_eq!(generate_bracket_structure_from_shape(&[2, 3]), "{ ( ) ( ) ( ) } { ( ) ( ) ( ) }");

        // 3D: { ( [ ] × dim3 ) × dim2 } × dim1 を生成
        assert_eq!(generate_bracket_structure_from_shape(&[1, 1, 1]), "{ ( [ ] ) }");
        assert_eq!(generate_bracket_structure_from_shape(&[1, 1, 2]), "{ ( [ ] [ ] ) }");
        assert_eq!(generate_bracket_structure_from_shape(&[1, 2, 3]), "{ ( [ ] [ ] [ ] ) ( [ ] [ ] [ ] ) }");
        assert_eq!(generate_bracket_structure_from_shape(&[2, 2, 3]), "{ ( [ ] [ ] [ ] ) ( [ ] [ ] [ ] ) } { ( [ ] [ ] [ ] ) ( [ ] [ ] [ ] ) }");
    }
}
