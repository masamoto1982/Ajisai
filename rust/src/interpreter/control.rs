// rust/src/interpreter/control.rs - 新しい制御構造

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType, Value};
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
    
    let val = interp.workspace.pop().ok_or(AjisaiError::WorkspaceUnderflow)?;
    console::log_1(&JsValue::from_str(&format!("Deleting word from value: {:?}", val)));
    
    let name = match val.val_type {
        ValueType::Vector(v) if v.len() == 1 => match &v[0].val_type {
            ValueType::String(s) => s.to_uppercase(),
            ValueType::Symbol(s) => s.to_uppercase(),
            _ => return Err(AjisaiError::type_error("string or symbol", "other type"))
        },
        ValueType::Symbol(s) => s.to_uppercase(),
        ValueType::String(s) => s.to_uppercase(),
        _ => return Err(AjisaiError::type_error("vector with string/symbol, string, or symbol", "other type")),
    };

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
