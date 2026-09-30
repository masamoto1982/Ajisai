use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_word_name_from_value;
use crate::interpreter::Interpreter;
use crate::types::Value;

pub fn op_del(interp: &mut Interpreter) -> Result<()> {
    let val = interp.stack.pop().ok_or(AjisaiError::stack_underflow())?;

    // A refusal puts the name back as it was written (the ERROR discipline of
    // every Core Word, LANG.STACK.CONSUMPTION): the operand is consumed only
    // once the Word is gone from the dictionary.
    let outcome = delete_named(interp, &val);
    if outcome.is_err() {
        interp.stack.push(val);
    }
    outcome
}

fn delete_named(interp: &mut Interpreter, val: &Value) -> Result<()> {
    let name = extract_word_name_from_value(val)?;

    let word_name = name.to_uppercase();

    if interp.core_vocabulary.contains_key(&word_name) {
        return Err(AjisaiError::declared(
            "protectedWord",
            format!("Cannot delete Core Word '{}'", word_name),
        ));
    }

    // A name that no User Word holds is `wordNotFound`: with two tiers the
    // name is the whole address, so the question is simply whether the User
    // tier holds it.
    if !interp.user_words.contains_key(&word_name) {
        return Err(AjisaiError::declared(
            "wordNotFound",
            format!("Word '{}' is not defined", word_name),
        ));
    }

    // A referenced word is not deletable. There is no force modifier: the
    // vocabulary has no Word that overrides this, so the refusal is final and
    // the caller's only route is to delete the dependents first. A word's own
    // self-reference does not lock it: see `collect_external_dependents`.
    let dependents = interp.collect_external_dependents(&word_name);
    if !dependents.is_empty() {
        // The same dependency-graph rule DEF declares as `definitionConflict`:
        // the dictionary's existing bindings refuse this change.
        return Err(AjisaiError::declared(
            "definitionConflict",
            format!(
                "Cannot delete '{}': referenced by {}. Delete those words first.",
                word_name,
                super::execute_def::sorted_names(&dependents)
            ),
        ));
    }

    // The index holds exactly the edges the definitions hold (`DEF` records
    // both directions, a forward reference included once its target is
    // defined), so the word's own edges are the ones to drop. Its dependents
    // entry is empty by the check above; the words that still *name* it keep
    // the name in `text_references` and will depend on it again if it is
    // defined again.
    if let Some(removed_def) = interp.user_words.remove(&word_name) {
        for dep_name in &removed_def.dependencies {
            if let Some(deps) = interp.dependents.get_mut(dep_name) {
                deps.remove(&word_name);
            }
        }
        interp.dependents.remove(&word_name);
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
