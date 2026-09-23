use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{
    extract_word_name_from_value, keep_mode_operands, restore_keep_mode_operands,
};
use crate::interpreter::word_contract::ContractFlow;
use crate::interpreter::{Interpreter, WordDefinition};
use crate::types::{ExecutionLine, Token};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Scan raw source for `#:contract NAME ...` lines, keyed by the upper-cased
/// name. `#:contract` is a tooling-only directive (an ordinary comment to the
/// interpreter, stripped before tokenization — `docs/dev/cost-contract-
/// design.md`); this does not parse or check its fields the way the CLI's
/// `agent::contract_decl` does (that module is `std`-only and unavailable to
/// the browser build). It only recovers the line's text, verbatim, so `DEF`
/// can attach it to the Word it names as a `description` for the host to
/// show — e.g. the Dictionary panel's hover — never as a checked contract.
pub(crate) fn extract_pending_word_descriptions(source: &str) -> HashMap<String, String> {
    let mut descriptions = HashMap::new();
    for line in source.lines() {
        let Some(rest) = line.trim_start().strip_prefix("#:contract") else {
            continue;
        };
        let rest = rest.trim();
        let Some(name_end) = rest.find(char::is_whitespace) else {
            continue;
        };
        let name = &rest[..name_end];
        let detail = rest[name_end..].trim();
        if name.is_empty() || detail.is_empty() {
            continue;
        }
        descriptions.insert(name.to_uppercase(), detail.to_string());
    }
    descriptions
}

/// Overwrite an existing Word's `description` in place. Used both when `DEF`
/// consumes a pending `#:contract` line for the Word it just defined, and
/// when a restored Word (`restore_user_words`) carries a saved description
/// alongside its body. The Word was just inserted with a fresh `Arc` in
/// every caller, so `Arc::get_mut` succeeds; the clone-and-replace fallback
/// only guards a future caller that might not hold sole ownership.
pub(crate) fn set_word_description(
    interp: &mut Interpreter,
    name: &str,
    description: Option<String>,
) {
    let upper_name = name.to_uppercase();
    let Some(arc_def) = interp.user_words.get_mut(&upper_name) else {
        return;
    };
    if let Some(def) = Arc::get_mut(arc_def) {
        def.description = description;
    } else {
        let mut cloned = (**arc_def).clone();
        cloned.description = description;
        *arc_def = Arc::new(cloned);
    }
}

/// DEF is strictly two positional arguments: `[ params | body ] 'NAME' DEF`.
///
/// The top of the stack is the name (a string), and directly below it is the
/// body — any Vector, since the CodeBlock/Vector unification
/// (docs/dev/type-unification-work-order-2026-08.md) — usually written as a
/// literal `[ ]` right there, but not required to be: a Vector built,
/// stored, or passed through any other means defines just as well. No value
/// types are inspected to *guess* roles — position alone determines them —
/// which is why a leftover string-like value on the stack can no longer
/// shift argument interpretation.
pub fn op_def(interp: &mut Interpreter) -> Result<()> {
    if interp.stack.len() < 2 {
        return Err(AjisaiError::StackUnderflow);
    }

    // `KEEP` preserves the operands of a Word that answers with nothing too:
    // `{ 1 } 'W' KEEP DEF` defines the Word and leaves the body and the name
    // on the stack. See `keep_mode_operands`.
    let kept = keep_mode_operands(interp, 2);

    let name_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;
    let name_str = extract_word_name_from_value(&name_val)?;

    let def_val = interp.stack.pop().ok_or(AjisaiError::StackUnderflow)?;

    // Prefer the body's own written tokens when the operand was a literal
    // right here (`execution_loop.rs`'s `def_body_tokens_if_literal_precedes_def`,
    // see `pending_def_body_tokens`'s doc comment): re-deriving them from
    // `def_val` through `value_as_code.rs` always re-expands a nested Vector
    // as `[ ]`, which is fine for *running* the body (either spelling
    // executes identically) but loses exactly the bracket-spelling fact the
    // contract engine's vector-depth gate (`word_contract_widen.rs`) reads to
    // tell code from data. `None` here just means the body came from a
    // computed Vector rather than a literal, and the bridge is the only way
    // to get tokens from it.
    let tokens = match interp.pending_def_body_tokens.take() {
        Some(tokens) => tokens,
        None => match def_val.as_vector_view() {
            // `as_vector_view` (Tensor-aware) — see control.rs's EXEC for why.
            Some(elements) => {
                crate::interpreter::value_as_code::value_elements_to_tokens(&elements)?
            }
            None => {
                return Err(AjisaiError::declared(
                    "invalidDefinitionBody",
                    "DEF: expected a Vector [ ... ] definition body, got a non-vector value",
                ));
            }
        },
    };

    op_def_inner(interp, &name_str, &tokens)?;
    if let Some(description) = interp
        .pending_word_descriptions
        .remove(&name_str.to_uppercase())
    {
        set_word_description(interp, &name_str, Some(description));
    }
    restore_keep_mode_operands(interp, kept);
    Ok(())
}

pub(crate) fn op_def_inner(interp: &mut Interpreter, name: &str, tokens: &[Token]) -> Result<()> {
    crate::tokenizer::validate_code_tokens(tokens).map_err(AjisaiError::MalformedSource)?;
    interp.check_source_numeric_literals(tokens)?;
    if let Some(message) =
        crate::interpreter::naming_convention_checker::check_reserved_word_name(name, "define")
    {
        return Err(AjisaiError::declared("protectedWord", message));
    }

    // A Word is reached by writing its name as one token, so a name that
    // cannot be written is not a name: `DEF` took one anyway, and the entry it
    // made could be listed, hovered and exported but never called. That splits
    // exactly what LANG.DICTIONARY.RESOLUTION joins — "the host's lookup,
    // hover, the Reference, and execution must identify the same canonical
    // entry" — so refuse it at the one moment the name is chosen. `BIND`
    // already refuses the same names, and says a binding is named like a Word;
    // this is the Word half of that sentence.
    //
    // `DEL` deliberately keeps no such check: a name saved before this rule, or
    // before a lexical rule changed under it, must stay removable.
    if !crate::tokenizer::is_symbol_token_lexeme(name) {
        return Err(AjisaiError::declared(
            "invalidName",
            format!(
                "Cannot define '{}': it is not a name. A Word is called by writing its name as one token, and this cannot be written — the definition could never be reached (LANG.DICTIONARY.RESOLUTION).",
                name
            ),
        ));
    }

    let upper_name = name.to_uppercase();

    if interp.core_vocabulary.contains_key(&upper_name) {
        return Err(AjisaiError::BuiltinProtection {
            word: upper_name,
            operation: "redefine".into(),
        });
    }

    // The other half of `BIND`'s refusal to take a Word's name. Together they
    // keep the two name spaces disjoint at every moment, so a reader never has
    // to know which of the two a name resolved through.
    if interp.lookup_binding(&upper_name).is_some() {
        return Err(AjisaiError::NameConflict(format!(
            "Cannot define '{}': the name is bound in this frame. A binding and a Word may not share a name.",
            upper_name
        )));
    }

    // The header is read after the name and before anything is mutated, so
    // a malformed or missing one leaves the dictionary exactly as it was
    // (LANG.DICTIONARY.MUTATION).
    let (params, tokens) = split_param_header(interp, name, tokens)?;

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
            return Err(AjisaiError::declared(
                "definitionConflict",
                format!(
                    "Cannot redefine '{}': referenced by {}. Delete those words first.",
                    upper_name, dep_list
                ),
            ));
        }

        for dep_name in &existing.dependencies {
            if let Some(dependents) = interp.dependents.get_mut(dep_name) {
                dependents.remove(&upper_name);
            }
        }
    }

    let staged_tokens = tokens.to_vec();
    // A header makes an empty body meaningful: `[ X | ]` takes one operand
    // and leaves nothing.
    let lines = if staged_tokens.iter().all(|t| matches!(t, Token::LineBreak)) {
        Vec::new()
    } else {
        parse_definition_body(&staged_tokens)?
    };

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
    // Section 8.7: every named symbol, resolved or not — the acyclicity check
    // below needs to see a forward reference to a word that does not exist
    // yet, which `new_dependencies` cannot represent.
    let mut new_text_references = HashSet::new();
    for line in lines.iter() {
        for token in line.body_tokens.iter() {
            if let Token::Symbol(s) = token {
                let upper_s = crate::core_word_aliases::canonicalize_core_word_name(s);
                // A parameter is a binding, not a reference to a Word.
                if params.iter().any(|p| p.as_str() == upper_s.as_ref()) {
                    continue;
                }
                new_text_references.insert(upper_s.to_string());
                if let Some((resolved_name, resolved_def)) = interp.resolve_word_entry(&upper_s) {
                    if !resolved_def.is_builtin || resolved_name.contains('@') {
                        new_dependencies.insert(resolved_name.to_string());
                    }
                }
            }
        }
    }

    // Section 8.7: the User dictionary's reference graph is acyclic — no Word
    // may name itself, directly or through any chain of other User words.
    // Repetition is expressed only through the bounded higher-order Words
    // (`MAP`, `FILTER`, `FOLD`, `ANY`, `ALL`) over an already-finite Vector,
    // never through a Word calling itself: every evaluation is then
    // structurally finite, not merely bounded by a runtime step budget.
    if let Some(cycle) = interp.find_reference_cycle(&upper_name, &new_text_references) {
        return Err(AjisaiError::SelfReferentialDefinition {
            word: upper_name,
            cycle,
        });
    }

    refuse_reads_below_frame(interp, &upper_name, &params, &lines)?;

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
        description: None,
        dependencies: new_dependencies,
        text_references: new_text_references,
        original_source: None,
        namespace: None,
        registration_order: interp.next_registration_order(),
        compiled_plan: None,
        // A User Word has no registry entry: `DEF` cannot define a Core Word
        // (LANG.DICTIONARY.RESOLUTION seals Core), so this is `None` by
        // construction rather than by omission.
        generated: None,
        params: Some(Arc::from(params)),
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

/// Refuse a body that reads below its frame on every run (LANG.SOURCE.FRAME).
///
/// The call starts the body on an empty stack, so such a Word could only ever
/// fail with a stack underflow. The body is read through its Core expansion
/// — `[ A B | body ]` as `'B' BIND 'A' BIND body` — which consumes exactly
/// the header's operands and whatever the body reaches below them; the
/// contract walk counts that without running anything. Only a fixed count
/// decides: a body whose stack effect depends on its values is left to fail,
/// or not, when it runs.
fn refuse_reads_below_frame(
    interp: &mut Interpreter,
    word_name: &str,
    params: &[String],
    lines: &[ExecutionLine],
) -> Result<()> {
    let mut expanded: Vec<Token> = Vec::new();
    for param in params.iter().rev() {
        expanded.push(Token::String(param.as_str().into()));
        expanded.push(Token::Symbol("BIND".into()));
    }
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            expanded.push(Token::LineBreak);
        }
        expanded.extend(line.body_tokens.iter().cloned());
    }
    let contract = interp.infer_contract_for_block(&expanded);
    match contract.flow {
        ContractFlow::Fixed { consumes, .. } if usize::from(consumes) > params.len() => {
            Err(AjisaiError::declared(
                "invalidDefinitionBody",
                format!(
                    "DEF: the body of '{}' reads {} value(s) below its frame. A call starts the body on an empty stack holding only its parameters, so every call would underflow; name each operand in the header instead (LANG.SOURCE.FRAME).",
                    word_name,
                    usize::from(consumes) - params.len()
                ),
            ))
        }
        _ => Ok(()),
    }
}

/// Separate a body's parameter header from the body proper.
///
/// `[ A B | … ]`: the names written before the first `|` of the body's first
/// statement, at the body's own level, are its parameters, deepest operand
/// first (LANG.SOURCE.FRAME). Every User Word states its arity this way, so a
/// body with no such `|` is refused. Every header fault is
/// `invalidDefinitionBody` — the body is what is malformed, whichever name in
/// it is at fault.
pub(crate) fn split_param_header<'t>(
    interp: &Interpreter,
    word_name: &str,
    tokens: &'t [Token],
) -> Result<(Vec<String>, &'t [Token])> {
    let start = tokens
        .iter()
        .take_while(|t| matches!(t, Token::LineBreak))
        .count();
    let mut depth: usize = 0;
    let mut separator = None;
    for (i, token) in tokens.iter().enumerate().skip(start) {
        match token {
            Token::VectorStart | Token::RecordStart => depth += 1,
            Token::VectorEnd | Token::RecordEnd => depth = depth.saturating_sub(1),
            Token::LineBreak if depth == 0 => break,
            Token::Symbol(s) if depth == 0 && s.as_ref() == "|" => {
                separator = Some(i);
                break;
            }
            _ => {}
        }
    }
    let malformed = |detail: String| {
        AjisaiError::declared(
            "invalidDefinitionBody",
            format!("DEF: the parameter header of '{}' {}", word_name, detail),
        )
    };
    let Some(separator) = separator else {
        return Err(malformed(
            "is missing. A body states the operands it takes before '|' — `[ X | X 2 * ]`, or `[ | 42 ]` for none (LANG.SOURCE.FRAME).".to_string(),
        ));
    };
    let word_upper = word_name.to_uppercase();
    let mut params: Vec<String> = Vec::new();
    for token in &tokens[start..separator] {
        let Token::Symbol(s) = token else {
            return Err(malformed(
                "may hold only names, one per operand, before '|'.".to_string(),
            ));
        };
        let upper = s.to_uppercase();
        interp
            .check_bindable_name(s)
            .map_err(|err| malformed(format!("names '{}', which cannot be bound: {}", s, err)))?;
        if upper == word_upper {
            return Err(malformed(format!(
                "names '{}', the Word being defined.",
                upper
            )));
        }
        if params.contains(&upper) {
            return Err(malformed(format!("names '{}' twice.", upper)));
        }
        params.push(upper);
    }
    Ok((params, &tokens[separator + 1..]))
}

/// Split a word body into execution lines.
///
/// A line break separates *statements*, and a statement is a thing written at
/// the body's own level. A break written inside a literal — a `[ ]` Vector
/// or a `{ }` Record — is interior to a single value, not a separator between two of them,
/// so it is carried through into that value's token stream untouched.
///
/// Splitting on interior breaks is what used to make a multi-line block
/// unusable inside a Word: a body of
///
/// ```text
/// [ [ 'N' BIND
/// [ 1 ] [ 0 ]
/// N [ 0 ] GT
/// SELECT ] MAP
/// ```
///
/// was cut at every break, leaving `[ [ 'N' BIND` as its own "line" — an
/// unclosed block, and an error raised at the call rather than at the
/// definition. Depth is the whole rule: at depth 0 a break ends a statement,
/// below it a break is just a token.
pub(crate) fn parse_definition_body(tokens: &[Token]) -> Result<Vec<ExecutionLine>> {
    let mut lines = Vec::new();
    let mut processed_tokens = Vec::new();
    let mut depth: usize = 0;

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::LineBreak if depth == 0 => {
                if !processed_tokens.is_empty() {
                    let execution_line = ExecutionLine {
                        body_tokens: processed_tokens.clone().into(),
                    };
                    lines.push(execution_line);
                    processed_tokens.clear();
                }
            }
            token => {
                match token {
                    Token::VectorStart | Token::RecordStart => depth += 1,
                    Token::VectorEnd | Token::RecordEnd => depth = depth.saturating_sub(1),
                    _ => {}
                }
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
        return Err(AjisaiError::declared(
            "invalidDefinitionBody",
            "DEF: expected a non-empty definition body, got an empty body",
        ));
    }

    Ok(lines)
}
