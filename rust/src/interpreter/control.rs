// rust/src/interpreter/control.rs - DEL構文対応版

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType};
use web_sys::console;
use wasm_bindgen::JsValue;

pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_def ==="));
    console::log_1(&JsValue::from_str(&format!("Workspace size: {}", interp.workspace.len())));
    
    // 新しい構文では DEF は単体で呼ばれることはない
    // ワード定義は parse_word_definition で処理される
    Err(AjisaiError::from("DEF should be used in word definition context"))
}

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    console::log_1(&JsValue::from_str("=== op_del ==="));
    console::log_1(&JsValue::from_str(&format!("Workspace size: {}", interp.workspace.len())));
    
    // 新しい構文では DEL は単体で呼ばれることはない
    // [ DEL [ WORD_NAME ] ] の形式で処理される
    Err(AjisaiError::from("DEL should be used in [ DEL [ WORD_NAME ] ] format"))
}

pub fn op_del_word(interp: &mut Interpreter, word_name: &str) -> Result<()> {
    console::log_1(&JsValue::from_str(&format!("=== op_del_word: {} ===", word_name)));

    let name = word_name.to_uppercase();
    console::log_1(&JsValue::from_str(&format!("Attempting to delete word: '{}'", name)));

    if interp.dictionary.get(&name).map_or(false, |d| d.is_builtin) {
        console::log_1(&JsValue::from_str(&format!("Cannot delete builtin word: {}", name)));
        return Err(AjisaiError::from(format!("Cannot delete builtin word: {}", name)));
    }
    
    if interp.dependencies.get(&name).map_or(false, |deps| !deps.is_empty()) {
        let dependents = interp.dependencies.get(&name).unwrap().iter().cloned().collect();
        console::log_1(&JsValue::from_str(&format!("Word {} is protected by dependencies: {:?}", name, dependents)));
        return Err(AjisaiError::ProtectedWord { name, dependents });
    }
    
    if interp.dictionary.remove(&name).is_some() {
        interp.output_buffer.push_str(&format!("Deleted word: {}\n", name));
        console::log_1(&JsValue::from_str(&format!("Successfully deleted word: {}", name)));
    } else {
        console::log_1(&JsValue::from_str(&format!("Word not found: {}", name)));
        return Err(AjisaiError::UnknownWord(name));
    }
    
    Ok(())
}
