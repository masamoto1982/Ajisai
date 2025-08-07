use crate::interpreter::{Interpreter, error::{AjisaiError, Result}};
use crate::types::ValueType;

pub fn op_def(_interp: &mut Interpreter) -> Result<()> {
    // DEFは行末での特殊な構文として処理されるため、
    // 通常の実行フローでここに到達した場合はエラー
    Err(AjisaiError::from("DEF must be used at the end of a line with a string name: <words> \"NAME\" DEF"))
}

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
            
            // 依存関係チェック
            if let Some(dependents) = interp.dependencies.get(&name) {
                if !dependents.is_empty() {
                    let dependent_list: Vec<String> = dependents.iter().cloned().collect();
                    return Err(AjisaiError::ProtectedWord {
                        name: name.clone(),
                        dependents: dependent_list,
                    });
                }
            }
            
            interp.dictionary.remove(&name);
            interp.dependencies.remove(&name);
            Ok(())
        },
        _ => Err(AjisaiError::type_error("string", "other type")),
    }
}
