use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_word_name_from_value;
use crate::interpreter::{Interpreter, OperationTargetMode};

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "DEL".into(),
            mode: "Stack".into(),
        });
    }

    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let name = extract_word_name_from_value(&val)?;

    let upper_name = name.to_uppercase();


    let (target_dict, word_name) = if let Some((ns, w)) = interp.split_qualified_name(&upper_name) {
        (Some(ns), w)
    } else {
        (None, upper_name.clone())
    };

    if interp.core_vocabulary.contains_key(&word_name) {
        interp.force_flag = false;
        return Err(AjisaiError::BuiltinProtection {
            word: word_name,
            operation: "delete".into(),
        });
    }


    if target_dict.is_none() {
        if interp.user_dictionaries.contains_key(&word_name) {
            interp.user_dictionaries.remove(&word_name);
            interp.sync_user_words_cache();
            interp.rebuild_dependencies()?;
            interp
                .output_buffer
                .push_str(&format!("Deleted dictionary: {}\n", word_name));
            interp.force_flag = false;
            return Ok(());
        }

        if interp.module_vocabulary.contains_key(&word_name) {
            interp.module_vocabulary.remove(&word_name);
            interp.import_table.modules.remove(&word_name);
            interp.sync_user_words_cache();
            interp.rebuild_dependencies()?;
            interp
                .output_buffer
                .push_str(&format!("Deleted dictionary: {}\n", word_name));
            interp.force_flag = false;
            return Ok(());
        }
    }


    let (owner_name, is_module) = find_word_owner(interp, target_dict.as_deref(), &word_name)?;


    if is_module && !interp.force_flag {
        interp.force_flag = false;
        return Err(AjisaiError::from(format!(
            "Word '{}' is a module sample word. Use ! '{}' DEL to force delete.",
            word_name, word_name
        )));
    }

    let fq_name = format!("{}@{}", owner_name, word_name);
    let dependents = interp.collect_dependents(&fq_name);

    if !dependents.is_empty() && !interp.force_flag {
        let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
        return Err(AjisaiError::from(format!(
            "Cannot delete '{}': referenced by {}. Use ! '{}' DEL to force.",
            word_name, dep_list, word_name
        )));
    }


    let removed_def = if is_module {
        interp
            .module_vocabulary
            .get_mut(&owner_name)
            .and_then(|dict| dict.sample_words.remove(&word_name))
    } else {
        interp
            .user_dictionaries
            .get_mut(&owner_name)
            .and_then(|dict| dict.words.remove(&word_name))
    };

    if let Some(removed_def) = removed_def {
        interp.sync_user_words_cache();
        for dep_name in &removed_def.dependencies {
            if let Some(deps) = interp.dependents.get_mut(dep_name) {
                deps.remove(&fq_name);
            }
        }
        interp.dependents.remove(&fq_name);
        for deps in interp.dependents.values_mut() {
            deps.remove(&fq_name);
        }
    }

    if !dependents.is_empty() {
        let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
        interp.output_buffer.push_str(&format!(
            "Warning: '{}' was deleted. Affected words: {}\n",
            word_name, dep_list
        ));
    }

    interp
        .output_buffer
        .push_str(&format!("Deleted word: {}\n", fq_name));

    interp.force_flag = false;
    Ok(())
}




fn find_word_owner(
    interp: &Interpreter,
    target_dict: Option<&str>,
    word_name: &str,
) -> Result<(String, bool)> {
    if let Some(dict_name) = target_dict {

        if let Some(dict) = interp.user_dictionaries.get(dict_name) {
            if dict.words.contains_key(word_name) {
                return Ok((dict_name.to_string(), false));
            }
        }
        if let Some(module) = interp.module_vocabulary.get(dict_name) {
            if module.sample_words.contains_key(word_name) {
                return Ok((dict_name.to_string(), true));
            }
        }
        Err(AjisaiError::from(format!(
            "Word '{}@{}' is not defined",
            dict_name, word_name
        )))
    } else {

        for (dict_name, dict) in &interp.user_dictionaries {
            if dict.words.contains_key(word_name) {
                return Ok((dict_name.clone(), false));
            }
        }
        for (module_name, module) in &interp.module_vocabulary {
            if module.sample_words.contains_key(word_name) {
                return Ok((module_name.clone(), true));
            }
        }
        Err(AjisaiError::from(format!(
            "Word '{}' is not defined",
            word_name
        )))
    }
}
