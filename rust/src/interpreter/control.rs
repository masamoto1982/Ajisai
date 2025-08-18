// rust/src/interpreter/control.rs (簡素化版)

use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::{ValueType};

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop()
        .ok_or(AjisaiError::StackUnderflow)?;
    
    match val.val_type {
        ValueType::String(name) => {
            let name = name.to_uppercase();
            
            // ビルトインワードは削除不可
            if let Some(def) = interp.dictionary.get(&name) {
                if def.is_builtin {
                    return Err(AjisaiError::from(format!("Cannot delete builtin word: {}", name)));
                }
            }
            
            // 辞書から削除
            interp.dictionary.remove(&name);
            interp.append_output(&format!("Deleted: {}\n", name));
            
            Ok(())
        },
        _ => Err(AjisaiError::type_error("string", "other type")),
    }
}
