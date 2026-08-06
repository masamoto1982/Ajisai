use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_word_name_from_value;
use crate::interpreter::{Interpreter, WordDefinition};
use crate::types::{Capabilities, ExecutionLine, Stability, Tier, Token, ValueData};
use std::collections::HashSet;
use std::sync::Arc;

/// DEF is strictly two positional arguments: `{ body } 'NAME' DEF`.
///
/// The top of the stack is the name (a string), and directly below it is the
/// body (a code block `{ }`). No value types are inspected to *guess* roles —
/// position alone determines them — which is why a leftover string-like value
/// on the stack can no longer shift argument interpretation. Definitions from
/// data arrays are intentionally not accepted here; canonical code data must
/// first cross the explicit `REFLECT` boundary.
pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let name_str = extract_word_name_from_value(&name_val)?;

    let def_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let tokens = match &def_val.data {
        ValueData::CodeBlock(tokens) => tokens.clone(),
        _ => {
            return Err(AjisaiError::from(
                "DEF requires a code block { ... } as the definition body",
            ));
        }
    };

    op_def_inner(interp, &name_str, &tokens)
}

pub(crate) fn op_def_inner(interp: &mut Interpreter, name: &str, tokens: &[Token]) -> Result<()> {
    crate::tokenizer::validate_code_tokens(tokens).map_err(AjisaiError::from)?;
    interp.check_source_numeric_literals(tokens)?;
    if let Some(message) =
        crate::interpreter::naming_convention_checker::check_reserved_word_name(name)
    {
        return Err(AjisaiError::from(message));
    }

    let upper_name = name.to_uppercase();

    if interp.core_vocabulary.contains_key(&upper_name) {
        return Err(AjisaiError::BuiltinProtection {
            word: upper_name,
            operation: "redefine".into(),
        });
    }

    if let Some(warning) =
        crate::interpreter::naming_convention_checker::check_word_name_convention(name)
    {
        interp.output_buffer.push_str(&format!("{}\n", warning));
    }

    // One User tier (LANG.DICTIONARY.RESOLUTION), so a Word's name is its
    // whole address: no active dictionary to pick, no `DICT@WORD` to build.
    if let Some(existing) = interp.user_words.get(&upper_name) {
        // A word's own self-reference does not lock it: see
        // `collect_external_dependents`.
        let dependents = interp.collect_external_dependents(&upper_name);

        // A referenced word is not redefinable. There is no force modifier: the
        // vocabulary has no Word that overrides this, so the refusal is final
        // and the caller's only route is to delete the dependents first.
        if !dependents.is_empty() {
            let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
            return Err(AjisaiError::from(format!(
                "Cannot redefine '{}': referenced by {}. Delete those words first.",
                upper_name, dep_list
            )));
        }

        for dep_name in &existing.dependencies {
            if let Some(dependents) = interp.dependents.get_mut(dep_name) {
                dependents.remove(&upper_name);
            }
        }
    }

    let staged_tokens = tokens.to_vec();
    let lines = parse_definition_body(&staged_tokens)?;

    // Content store (Section 8.6): share one stored body across textually
    // identical definitions so copying or re-importing a word group does not
    // duplicate its code.
    let body_key = crate::interpreter::word_identity::body_content_key(&lines);
    let lines: Arc<[ExecutionLine]> = match interp.body_store.get(&body_key) {
        Some(shared) => shared.clone(),
        None => {
            let arc: Arc<[ExecutionLine]> = lines.into();
            interp.body_store.insert(body_key, arc.clone());
            arc
        }
    };

    // Section 8.6: resolve this word's references through its own dictionary
    let mut new_dependencies = HashSet::new();
    for line in lines.iter() {
        for token in line.body_tokens.iter() {
            if let Token::Symbol(s) = token {
                let upper_s = crate::core_word_aliases::canonicalize_core_word_name(s);
                if let Some((resolved_name, resolved_def)) = interp.resolve_word_entry(&upper_s) {
                    if !resolved_def.is_builtin || resolved_name.contains('@') {
                        new_dependencies.insert(resolved_name);
                    }
                }
            }
        }
    }

    for dep_name in &new_dependencies {
        interp
            .dependents
            .entry(dep_name.clone())
            .or_default()
            .insert(upper_name.clone());
    }

    let new_def = WordDefinition {
        lines,
        is_builtin: false,
        tier: Tier::Contrib,
        stability: Stability::Stable,
        capabilities: Capabilities::PURE,
        description: None,
        dependencies: new_dependencies,
        original_source: None,
        namespace: None,
        registration_order: interp.next_registration_order(),
        execution_plans: None,
    };

    interp
        .user_words
        .insert(upper_name.clone(), Arc::new(new_def));
    interp.recompute_word_identities();
    interp.gc_body_store();
    interp
        .output_buffer
        .push_str(&format!("Defined word: {}\n", name));
    interp.dictionary_changes_this_run.push(name.to_string());

    interp.bump_dictionary_epoch();
    Ok(())
}

pub(crate) fn parse_definition_body(tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
    let mut lines = Vec::new();
    let mut processed_tokens = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::LineBreak => {
                if !processed_tokens.is_empty() {
                    let execution_line = ExecutionLine {
                        body_tokens: processed_tokens.clone().into(),
                    };
                    lines.push(execution_line);
                    processed_tokens.clear();
                }
            }
            _ => {
                processed_tokens.push(tokens[i].clone());
            }
        }
        i += 1;
    }

    if !processed_tokens.is_empty() {
        let execution_line = ExecutionLine {
            body_tokens: processed_tokens.into(),
        };
        lines.push(execution_line);
    }

    if lines.is_empty() {
        return Err(AjisaiError::from("Word definition cannot be empty"));
    }

    Ok(lines)
}
