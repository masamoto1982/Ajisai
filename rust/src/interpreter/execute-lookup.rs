use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_word_name_from_value;
use crate::interpreter::{Interpreter, OperationTargetMode};

pub fn op_lookup(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "? (LOOKUP)".into(),
            mode: "Stack".into(),
        });
    }

    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let name_str = extract_word_name_from_value(&name_val)?;

    let upper_name = name_str.to_uppercase();

    if let Some(def) = interp.resolve_word(&upper_name) {
        if def.is_builtin {
            let detailed_info = crate::builtins::lookup_builtin_detail(&upper_name);
            interp.definition_to_load = Some(detailed_info);
            return Ok(());
        }

        if let Some(original_source) = &def.original_source {
            interp.definition_to_load = Some(original_source.clone());
        } else {
            let definition = interp
                .lookup_word_definition_tokens(&upper_name)
                .unwrap_or_default();
            let full_definition = if definition.is_empty() {
                format!("[ NIL ] '{}' DEF", name_str)
            } else {
                if let Some(desc) = &def.description {
                    format!("[ {} ] '{}' '{}' DEF", definition, name_str, desc)
                } else {
                    format!("[ {} ] '{}' DEF", definition, name_str)
                }
            };
            interp.definition_to_load = Some(full_definition);
        }
        Ok(())
    } else {
        Err(AjisaiError::UnknownWord(name_str))
    }
}
