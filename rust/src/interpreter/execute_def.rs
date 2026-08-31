use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::{
    extract_word_name_from_value, keep_mode_operands, restore_keep_mode_operands,
};
use crate::interpreter::{Interpreter, WordDefinition};
use crate::types::{Capabilities, ExecutionLine, Stability, Tier, Token};
use std::collections::HashSet;
use std::sync::Arc;

/// DEF is strictly two positional arguments: `{ body } 'NAME' DEF`.
///
/// The top of the stack is the name (a string), and directly below it is the
/// body — any Vector, since the CodeBlock/Vector unification
/// (docs/dev/type-unification-work-order-2026-08.md) — usually written as a
/// literal `{ }` right there, but not required to be: a Vector built,
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
                return Err(AjisaiError::from(
                    "DEF requires a Vector [ ... ] as the definition body",
                ));
            }
        },
    };

    op_def_inner(interp, &name_str, &tokens)?;
    restore_keep_mode_operands(interp, kept);
    Ok(())
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

    // The other half of `BIND`'s refusal to take a Word's name. Together they
    // keep the two name spaces disjoint at every moment, so a reader never has
    // to know which of the two a name resolved through.
    if interp.lookup_binding(&upper_name).is_some() {
        return Err(AjisaiError::NameConflict(format!(
            "Cannot define '{}': the name is bound in this frame. A binding and a Word may not share a name.",
            upper_name
        )));
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
    // Section 8.7: every named symbol, resolved or not — the acyclicity check
    // below needs to see a forward reference to a word that does not exist
    // yet, which `new_dependencies` cannot represent.
    let mut new_text_references = HashSet::new();
    for line in lines.iter() {
        for token in line.body_tokens.iter() {
            if let Token::Symbol(s) = token {
                let upper_s = crate::core_word_aliases::canonicalize_core_word_name(s);
                new_text_references.insert(upper_s.to_string());
                if let Some((resolved_name, resolved_def)) = interp.resolve_word_entry(&upper_s) {
                    if !resolved_def.is_builtin || resolved_name.contains('@') {
                        new_dependencies.insert(resolved_name);
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
        text_references: new_text_references,
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

/// Split a word body into execution lines.
///
/// A line break separates *statements*, and a statement is a thing written at
/// the body's own level. A break written inside a `{ }` block or a `[ ]`
/// vector is interior to a single value, not a separator between two of them,
/// so it is carried through into that value's token stream untouched.
///
/// Splitting on interior breaks is what used to make a multi-line COND
/// unusable inside a Word: a body of
///
/// ```text
/// [ [ 0 GT | 1 ]
/// [ IDLE | 0 ]
/// COND ] MAP
/// ```
///
/// was cut at the two breaks, leaving `[ [ 0 GT | 1 ]` as its own "line" —
/// an unclosed block, and an error raised at the call rather than at the
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
                    Token::VectorStart => depth += 1,
                    Token::VectorEnd => depth = depth.saturating_sub(1),
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
        return Err(AjisaiError::from("Word definition cannot be empty"));
    }

    Ok(lines)
}
