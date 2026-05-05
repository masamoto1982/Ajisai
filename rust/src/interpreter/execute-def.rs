use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_word_name_from_value;
use crate::interpreter::vector_exec::format_vector_to_source;
use crate::interpreter::{Interpreter, OperationTargetMode, WordDefinition};
use crate::types::{Capabilities, ExecutionLine, Stability, Tier, Token, Value, ValueData};
use std::collections::HashSet;
use std::sync::Arc;

const PRECOMPUTE_STEP_LIMIT: usize = 20_000;

fn extract_block(tokens: &[Token], start: usize) -> Result<(Vec<Token>, usize)> {
    let mut depth = 0usize;
    let mut out = Vec::new();
    let mut i = start;
    while i < tokens.len() {
        match &tokens[i] {
            Token::BlockStart => {
                depth += 1;
                out.push(tokens[i].clone());
            }
            Token::BlockEnd => {
                if depth == 0 {
                    return Err(AjisaiError::from("PRECOMPUTE rejected: malformed block"));
                }
                depth -= 1;
                out.push(tokens[i].clone());
                if depth == 0 {
                    return Ok((out, i));
                }
            }
            _ => out.push(tokens[i].clone()),
        }
        i += 1;
    }
    Err(AjisaiError::from("PRECOMPUTE rejected: unterminated block"))
}

fn assert_comptime_safe_tokens(
    interp: &mut Interpreter,
    tokens: &[Token],
    visiting: &mut std::collections::HashSet<String>,
) -> Result<()> {
    for token in tokens {
        if let Token::Symbol(name) = token {
            if name.eq_ignore_ascii_case("PRECOMPUTE") {
                return Err(AjisaiError::from(
                    "PRECOMPUTE rejected: nested PRECOMPUTE is not supported in Phase 1",
                ));
            }
            let Some((resolved_name, def)) = interp.resolve_word_entry(name) else {
                return Err(AjisaiError::from(format!(
                    "PRECOMPUTE rejected: unresolved word {}",
                    name
                )));
            };

            if def.is_builtin {
                if !def.capabilities.contains(Capabilities::PURE)
                    || def.capabilities.contains(Capabilities::EVAL)
                    || def.capabilities.contains(Capabilities::IO)
                    || def.capabilities.contains(Capabilities::SPAWN)
                    || def.capabilities.contains(Capabilities::MUTATES_DICT)
                {
                    return Err(AjisaiError::from(format!(
                        "PRECOMPUTE rejected: word {} is not comptime-safe",
                        name
                    )));
                }
            } else {
                if visiting.contains(&resolved_name) {
                    return Err(AjisaiError::from(format!(
                        "PRECOMPUTE rejected: recursive dependency detected at {}",
                        resolved_name
                    )));
                }
                visiting.insert(resolved_name.clone());
                for line in def.lines.iter() {
                    assert_comptime_safe_tokens(interp, &line.body_tokens, visiting)?;
                }
                visiting.remove(&resolved_name);
            }
        }
    }
    Ok(())
}

fn precompute_definition_tokens(interp: &mut Interpreter, tokens: &[Token]) -> Result<Vec<Token>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        if matches!(tokens[i], Token::BlockStart) {
            let (block_tokens, block_end) = extract_block(tokens, i)?;
            if matches!(tokens.get(block_end + 1), Some(Token::Symbol(s)) if s.eq_ignore_ascii_case("PRECOMPUTE"))
            {
                assert_comptime_safe_tokens(
                    interp,
                    &block_tokens[1..block_tokens.len() - 1],
                    &mut std::collections::HashSet::new(),
                )?;

                let mut sandbox = Interpreter::new();
                sandbox.core_vocabulary = interp.core_vocabulary.clone();
                sandbox.user_words = interp.user_words.clone();
                sandbox.user_dictionaries = interp.user_dictionaries.clone();
                sandbox.module_vocabulary = interp.module_vocabulary.clone();
                sandbox.active_user_dictionary = interp.active_user_dictionary.clone();
                sandbox.max_execution_steps = PRECOMPUTE_STEP_LIMIT;
                sandbox.execute_section_core(&block_tokens[1..block_tokens.len() - 1], 0)?;
                if sandbox.execution_step_count >= PRECOMPUTE_STEP_LIMIT {
                    return Err(AjisaiError::from("PRECOMPUTE failed: execution exceeded step limit"));
                }
                for v in &sandbox.stack {
                    match &v.data {
                        ValueData::Scalar(n) => out.push(Token::Number(n.to_string().into())),
                        ValueData::Vector(vals) => {
                            out.push(Token::VectorStart);
                            for child in vals.iter() {
                                match &child.data {
                                    ValueData::Scalar(n) => {
                                        out.push(Token::Number(n.to_string().into()))
                                    }
                                    _ => {
                                        return Err(AjisaiError::from(
                                            "PRECOMPUTE failed: result contains unsupported value type",
                                        ))
                                    }
                                }
                            }
                            out.push(Token::VectorEnd);
                        }
                        _ => {
                            return Err(AjisaiError::from(
                                "PRECOMPUTE failed: result contains unsupported value type",
                            ))
                        }
                    }
                }
                i = block_end + 2;
                continue;
            }
        }
        out.push(tokens[i].clone());
        i += 1;
    }
    Ok(out)
}

fn extract_string_from_value(val: &Value) -> Result<String> {
    fn collect_chars(val: &Value) -> Vec<char> {
        match &val.data {
            ValueData::Nil => vec![],
            ValueData::Scalar(f) => f
                .to_i64()
                .and_then(|n| {
                    if n >= 0 && n <= 0x10FFFF {
                        char::from_u32(n as u32)
                    } else {
                        None
                    }
                })
                .map(|c| vec![c])
                .unwrap_or_default(),
            ValueData::Vector(children)
            | ValueData::Record {
                pairs: children, ..
            } => children.iter().flat_map(|c| collect_chars(c)).collect(),
            ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => vec![],
        }
    }

    let chars = collect_chars(val);
    if chars.is_empty() {
        return Err(AjisaiError::from("Cannot convert NIL to string"));
    }

    Ok(chars.into_iter().collect())
}

fn is_string_like(val: &Value) -> bool {
    if val.is_nil() {
        return false;
    }

    fn check_codepoints(val: &Value) -> bool {
        match &val.data {
            ValueData::Nil => false,
            ValueData::Scalar(f) => f.to_i64().map(|n| n >= 0 && n <= 0x10FFFF).unwrap_or(false),
            ValueData::Vector(children)
            | ValueData::Record {
                pairs: children, ..
            } => children.iter().all(|c| check_codepoints(c)),
            ValueData::CodeBlock(_)
            | ValueData::ProcessHandle(_)
            | ValueData::SupervisorHandle(_) => false,
        }
    }

    check_codepoints(val)
}

fn check_definition_descriptor_on_stack(stack: &[Value]) -> bool {
    if stack.len() < 3 {
        return false;
    }
    let last = &stack[stack.len() - 1];
    let second_last = &stack[stack.len() - 2];
    is_string_like(last) && is_string_like(second_last)
}

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

    let mut description = None;

    let has_description = check_definition_descriptor_on_stack(&interp.stack);

    if has_description {
        if let Some(desc_val) = interp.stack.pop() {
            if let Ok(s) = extract_string_from_value(&desc_val) {
                description = Some(s);
            }
        }
    }

    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let name_str = extract_word_name_from_value(&name_val)?;

    let mut def_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    if let ValueData::CodeBlock(tokens) = &def_val.data {
        let mut merged_tokens: Vec<Token> = tokens.to_vec();
        while let Some(prev) = interp.stack.last() {
            let Some(prev_tokens) = prev.as_code_block() else {
                break;
            };
            let previous_block_tokens: Vec<Token> = prev_tokens.to_vec();
            let _ = interp.stack.pop();
            let mut composed: Vec<Token> = previous_block_tokens;
            composed.push(Token::LineBreak);
            composed.extend(merged_tokens);
            merged_tokens = composed;
        }
        def_val = Value::from_code_block(merged_tokens);
    }

    let definition_str = match &def_val.data {
        ValueData::CodeBlock(tokens) => tokens
            .iter()
            .map(|t| match t {
                Token::Number(n) => n.to_string(),
                Token::String(s) => format!("'{}'", s),
                Token::Symbol(s) => s.to_string(),
                Token::VectorStart => "[".to_string(),
                Token::VectorEnd => "]".to_string(),
                Token::BlockStart => "{".to_string(),
                Token::BlockEnd => "}".to_string(),
                Token::Pipeline => "==".to_string(),
                Token::NilCoalesce => "=>".to_string(),
                Token::CondClauseSep => "$".to_string(),
                Token::SafeMode => "~".to_string(),
                Token::LineBreak => "\n".to_string(),
            })
            .collect::<Vec<_>>()
            .join(" "),
        ValueData::Vector(_) | ValueData::Record { .. } => format_vector_to_source(&def_val)?,
        _ => {
            return Err(AjisaiError::from(
                "DEF requires a code block ({ ... } / ( ... )) or vector as definition body",
            ));
        }
    };

    let tokens = crate::tokenizer::tokenize(&definition_str)
        .map_err(|e| AjisaiError::from(format!("Tokenization error in DEF: {}", e)))?;

    op_def_inner(interp, &name_str, &tokens, description)
}

pub(crate) fn op_def_inner(
    interp: &mut Interpreter,
    name: &str,
    tokens: &[Token],
    description: Option<String>,
) -> Result<()> {
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

    let mut collision_modules = Vec::new();
    for (module_name, module_dict) in &interp.module_vocabulary {
        if module_dict.sample_words.contains_key(&upper_name) {
            collision_modules.push(module_name.clone());
        }
    }

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
                "Cannot redefine '{}': referenced by {}. Use ! [ ... ] '{}' DEF to force.",
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

    let mut new_dependencies = HashSet::new();
    for line in &lines {
        for token in line.body_tokens.iter() {
            if let Token::Symbol(s) = token {
                let upper_s = crate::core_word_aliases::canonicalize_core_word_name(s);
                if let Some((resolved_name, resolved_def)) = interp.resolve_word_entry(&upper_s) {
                    if !resolved_def.is_builtin {
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
            .insert(fq_name.clone());
    }

    let new_def = WordDefinition {
        lines: lines.into(),
        is_builtin: false,
        tier: Tier::Contrib,
        stability: Stability::Stable,
        capabilities: Capabilities::PURE,
        description,
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
