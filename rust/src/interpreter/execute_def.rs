use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_word_name_from_value;
use crate::interpreter::{Interpreter, OperationTargetMode, WordDefinition};
use crate::types::{Capabilities, ExecutionLine, Stability, Tier, Token, ValueData};
use std::collections::HashSet;
use std::sync::Arc;

/// Serialize a code-block token stream back into Ajisai source text.
///
/// `LineBreak` tokens become real newlines so that a multi-line `{ }` body is
/// re-tokenized into one `ExecutionLine` per source line (see
/// `parse_definition_body`). This is the only body shape DEF accepts.
fn code_block_tokens_to_source(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|t| match t {
            Token::Number(n) => n.to_string(),
            Token::String(s) => format!("'{}'", s),
            Token::Symbol(s) => s.to_string(),
            Token::VectorStart => "[".to_string(),
            Token::VectorEnd => "]".to_string(),
            Token::BlockStart => "{".to_string(),
            Token::BlockEnd => "}".to_string(),
            Token::Pipeline => "~".to_string(),
            Token::NilCoalesce => "^".to_string(),
            Token::CondClauseSep => "|".to_string(),
            Token::LineBreak => "\n".to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// DEF is strictly two positional arguments: `{ body } 'NAME' DEF`.
///
/// The top of the stack is the name (a string), and directly below it is the
/// body (a code block `{ }`). No value types are inspected to *guess* roles —
/// position alone determines them — which is why a leftover string-like value
/// on the stack can no longer shift argument interpretation. Definitions from
/// data arrays are intentionally not accepted here; that path is reserved for
/// the future `>CODE` conversion word.
pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.operation_target_mode != OperationTargetMode::StackTop {
        return Err(AjisaiError::ModeUnsupported {
            word: "DEF".into(),
            mode: "Stack".into(),
        });
    }

    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let name_str = extract_word_name_from_value(&name_val)?;

    let def_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    let definition_str = match &def_val.data {
        ValueData::CodeBlock(tokens) => code_block_tokens_to_source(tokens),
        _ => {
            return Err(AjisaiError::from(
                "DEF requires a code block { ... } as the definition body",
            ));
        }
    };

    let tokens = crate::tokenizer::tokenize(&definition_str)
        .map_err(|e| AjisaiError::from(format!("Tokenization error in DEF: {}", e)))?;

    op_def_inner(interp, &name_str, &tokens)
}

pub(crate) fn op_def_inner(interp: &mut Interpreter, name: &str, tokens: &[Token]) -> Result<()> {
    if let Some(message) =
        crate::interpreter::naming_convention_checker::check_reserved_word_name(name)
    {
        interp.force_flag = false;
        return Err(AjisaiError::from(message));
    }

    let upper_name = name.to_uppercase();

    if interp.core_vocabulary.contains_key(&upper_name) {
        interp.force_flag = false;
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

    let collision_modules: Vec<String> = Vec::new();

    let dict_name = interp.active_user_dictionary.clone();
    let fq_name = format!("{}@{}", dict_name, upper_name);

    if let Some(existing) = interp
        .user_dictionaries
        .get(&dict_name)
        .and_then(|dict| dict.words.get(&upper_name))
    {
        let dependents = interp.collect_dependents(&fq_name);

        if !dependents.is_empty() && !interp.force_flag {
            let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
            interp.force_flag = false;
            return Err(AjisaiError::from(format!(
                "Cannot redefine '{}': referenced by {}. Use ! {{ ... }} '{}' DEF to force.",
                fq_name, dep_list, upper_name
            )));
        }

        if !dependents.is_empty() {
            let dep_list = dependents.iter().cloned().collect::<Vec<_>>().join(", ");
            interp.output_buffer.push_str(&format!(
                "Warning: '{}' was redefined. Affected words: {}\n",
                fq_name, dep_list
            ));
        }

        for dep_name in &existing.dependencies {
            if let Some(dependents) = interp.dependents.get_mut(dep_name) {
                dependents.remove(&fq_name);
            }
        }
    }

    let staged_tokens = crate::interpreter::comptime::precompute_definition_tokens(interp, tokens)?;
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
    // first, so the dependency it records is its own dictionary's word rather
    // than a same-named word in another (e.g. earlier-loaded) dictionary.
    let prev_owning = interp
        .owning_dictionary_context
        .replace(dict_name.clone());
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
    interp.owning_dictionary_context = prev_owning;

    for dep_name in &new_dependencies {
        interp
            .dependents
            .entry(dep_name.clone())
            .or_default()
            .insert(fq_name.clone());
    }

    let new_def = WordDefinition {
        lines: lines.into(),
        is_builtin: false,
        tier: Tier::Contrib,
        stability: Stability::Stable,
        capabilities: Capabilities::PURE,
        description: None,
        dependencies: new_dependencies,
        original_source: None,
        namespace: Some(dict_name.clone()),
        registration_order: interp.next_registration_order(),
        execution_plans: None,
    };

    let dict_order = interp
        .user_dictionaries
        .get(&dict_name)
        .map(|dict| dict.order)
        .unwrap_or_else(|| new_def.registration_order);
    interp
        .user_dictionaries
        .entry(dict_name.clone())
        .or_insert_with(|| crate::interpreter::UserDictionary {
            order: dict_order,
            words: std::collections::HashMap::new(),
        })
        .words
        .insert(upper_name.clone(), Arc::new(new_def));
    interp.sync_user_words_cache();
    interp.recompute_word_identities();
    interp.gc_body_store();
    interp
        .output_buffer
        .push_str(&format!("Defined word: {}@{}\n", dict_name, name));

    if !collision_modules.is_empty() {
        let module_paths: Vec<String> = collision_modules
            .iter()
            .map(|m| format!("{}@{}", m, upper_name))
            .collect();
        let user_path = format!("{}@{}", dict_name, upper_name);
        let all_paths: Vec<String> = module_paths
            .iter()
            .chain(std::iter::once(&user_path))
            .cloned()
            .collect();
        interp.output_buffer.push_str(&format!(
            "Warning: '{}' now exists in both {}. Use a qualified path when calling this word.\n",
            upper_name,
            all_paths.join(" and ")
        ));
    }
    interp.bump_dictionary_epoch();
    interp.force_flag = false;
    Ok(())
}

/// Host-only DEF entry point for spreadsheet cells (Sheet view; see
/// docs/dev/ajisai-spreadsheet-app-redesign-plan.md §2.4).
///
/// Overwriting a cell that other cells reference is normal spreadsheet
/// operation, so this takes the force path through the redefinition guard —
/// the same effect as `! { ... } 'A1' DEF` — and targets `dictionary`
/// directly without disturbing the interactively selected dictionary.
/// DEF's user-facing output-buffer chatter ("Defined word: ..." and
/// redefinition warnings) is rolled back: a cell definition is host
/// bookkeeping, not a user-visible execution, and must not leak into the
/// output of the next Editor-view run.
pub fn op_def_forced_in_dictionary(
    interp: &mut Interpreter,
    dictionary: &str,
    name: &str,
    tokens: &[Token],
) -> Result<()> {
    let prev_dictionary = std::mem::replace(
        &mut interp.active_user_dictionary,
        dictionary.to_uppercase(),
    );
    let prev_output_len = interp.output_buffer.len();
    interp.force_flag = true;
    let result = op_def_inner(interp, name, tokens);
    interp.active_user_dictionary = prev_dictionary;
    interp.force_flag = false;
    interp.output_buffer.truncate(prev_output_len);
    result
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
