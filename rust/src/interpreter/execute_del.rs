use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_word_name_from_value;
use crate::interpreter::Interpreter;

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::stack_underflow())?;

    let name = extract_word_name_from_value(&val)?;

    let word_name = name.to_uppercase();

    if interp.core_vocabulary.contains_key(&word_name) {
        return Err(AjisaiError::declared(
            "protectedWord",
            format!("Cannot delete Core Word '{}'", word_name),
        ));
    }

    // A name that no User Word holds is `wordNotFound`. DEL used to reach this
    // through a dictionary-owner search that also accepted `DICT@WORD`; with
    // two tiers the name is the whole address, so the question is simply
    // whether the User tier holds it.
    if !interp.user_words.contains_key(&word_name) {
        return Err(AjisaiError::declared(
            "wordNotFound",
            format!("Word '{}' is not defined", word_name),
        ));
    }

    // DEL used to also delete a whole named dictionary when the name matched
    // one. There are no named dictionaries to delete now.
    // A word's own self-reference does not lock it: see
    // `collect_external_dependents`.
    let dependents = interp.collect_external_dependents(&word_name);

    // A referenced word is not deletable. There is no force modifier: the
    // vocabulary has no Word that overrides this, so the refusal is final and
    // the caller's only route is to delete the dependents first.
    if !dependents.is_empty() {
        // The same dependency-graph rule DEF declares as `definitionConflict`:
        // the dictionary's existing bindings refuse this change.
        let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
        return Err(AjisaiError::declared(
            "definitionConflict",
            format!(
                "Cannot delete '{}': referenced by {}. Delete those words first.",
                word_name, dep_list
            ),
        ));
    }

    let removed_def = interp.user_words.remove(&word_name);

    if let Some(removed_def) = removed_def {
        for dep_name in &removed_def.dependencies {
            if let Some(deps) = interp.dependents.get_mut(dep_name) {
                deps.remove(&word_name);
            }
        }
        interp.dependents.remove(&word_name);
        for deps in interp.dependents.values_mut() {
            deps.remove(&word_name);
        }
    }

    interp
        .output_buffer
        .push_str(&format!("Deleted word: {}\n", word_name));
    interp
        .dictionary_changes_this_run
        .push(word_name.to_string());

    interp.recompute_word_identities();
    interp.gc_body_store();
    interp.bump_dictionary_epoch();
    Ok(())
}
