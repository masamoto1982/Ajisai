use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_word_name_from_value;
use crate::interpreter::Interpreter;

pub fn op_lookup(interp: &mut Interpreter) -> Result<()> {
    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let name_str = extract_word_name_from_value(&name_val)?;

    let canonical_name = crate::core_word_aliases::canonicalize_core_word_name(&name_str);

    if let Some(def) = interp.resolve_word(&canonical_name) {
        if def.is_builtin {
            let detailed_info = crate::builtins::lookup_builtin_detail(&name_str);
            interp.definition_to_load = Some(detailed_info);
            return Ok(());
        }

        if let Some(original_source) = &def.original_source {
            interp.definition_to_load = Some(original_source.clone());
        } else {
            let definition = interp
                .lookup_word_definition_tokens(&canonical_name)
                .unwrap_or_default();
            interp.definition_to_load = Some(render_def_source(
                &definition,
                &name_str,
                def.description.as_deref(),
            ));
        }
        Ok(())
    } else {
        Err(AjisaiError::UnknownWord(name_str))
    }
}

/// Reconstruct the `DEF` source of a User Word so that running it again defines
/// the same word — the point of `LOOKUP` on a user word being to load an
/// existing definition, edit it, and define it once more.
///
/// The body is wrapped in a **code block**. It used to be wrapped in `[ ]`, and
/// the result did not round-trip in either direction: `DEF` refuses a Vector
/// body ("DEF requires a code block { ... } as the definition body"), and even
/// as a value the meaning differed, since LANG.VALUES.VECTOR makes a bare name
/// inside `[ ]` its own text rather than a call. For a word whose body holds
/// `|` clauses the loaded text did not even tokenize, because `|` is COND-clause
/// sugar and is meaningless inside a Vector.
///
/// A multi-line body keeps its line structure: COND requires one `|` clause per
/// line, so the closing `}` and the name go on the line after the last body
/// line rather than being folded onto it.
///
/// A description is emitted as a leading `#` comment. `DEF` takes exactly two
/// positional arguments, so a third string on the line would be read as the
/// word's name; the comment carries the text without changing what runs.
fn render_def_source(definition: &str, name: &str, description: Option<&str>) -> String {
    let body = if definition.is_empty() {
        "{ NIL }".to_string()
    } else if definition.contains('\n') {
        format!("{{\n{}\n}}", definition)
    } else {
        format!("{{ {} }}", definition)
    };
    let source = format!("{} '{}' DEF", body, name);
    match description {
        Some(desc) if !desc.is_empty() => format!("# {}\n{}", desc.replace('\n', " "), source),
        _ => source,
    }
}
