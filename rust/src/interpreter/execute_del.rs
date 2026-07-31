use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_word_name_from_value;
use crate::interpreter::Interpreter;

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let name = extract_word_name_from_value(&val)?;

    let upper_name = name.to_uppercase();

    let (target_dict, word_name) = if let Some((ns, w)) = interp.split_qualified_name(&upper_name) {
        (Some(ns), w)
    } else {
        (None, upper_name.clone())
    };

    if interp.core_vocabulary.contains_key(&word_name) {
        return Err(AjisaiError::BuiltinProtection {
            word: word_name,
            operation: "delete".into(),
        });
    }

    if target_dict.is_none() && interp.user_dictionaries.contains_key(&word_name) {
        interp.user_dictionaries.remove(&word_name);
        interp.sync_user_words_cache();
        interp.rebuild_dependencies()?;
        interp
            .output_buffer
            .push_str(&format!("Deleted dictionary: {}\n", word_name));
        interp.bump_dictionary_epoch();
        return Ok(());
    }

    let owner_name = find_word_owner(interp, target_dict.as_deref(), &word_name)?;

    let fq_name = format!("{}@{}", owner_name, word_name);
    let dependents = interp.collect_dependents(&fq_name);

    // A referenced word is not deletable. There is no force modifier: the
    // vocabulary has no Word that overrides this, so the refusal is final and
    // the caller's only route is to delete the dependents first.
    if !dependents.is_empty() {
        let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
        return Err(AjisaiError::from(format!(
            "Cannot delete '{}': referenced by {}. Delete those words first.",
            word_name, dep_list
        )));
    }

    let removed_def = interp
        .user_dictionaries
        .get_mut(&owner_name)
        .and_then(|dict| dict.words.remove(&word_name));

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

    interp
        .output_buffer
        .push_str(&format!("Deleted word: {}\n", fq_name));

    interp.recompute_word_identities();
    interp.gc_body_store();
    interp.bump_dictionary_epoch();
    Ok(())
}

fn find_word_owner(
    interp: &Interpreter,
    target_dict: Option<&str>,
    word_name: &str,
) -> Result<String> {
    if let Some(dict_name) = target_dict {
        if let Some(dict) = interp.user_dictionaries.get(dict_name) {
            if dict.words.contains_key(word_name) {
                return Ok(dict_name.to_string());
            }
        }
        Err(AjisaiError::from(format!(
            "Word '{}@{}' is not defined",
            dict_name, word_name
        )))
    } else {
        for (dict_name, dict) in &interp.user_dictionaries {
            if dict.words.contains_key(word_name) {
                return Ok(dict_name.clone());
            }
        }
        Err(AjisaiError::from(format!(
            "Word '{}' is not defined",
            word_name
        )))
    }
}
