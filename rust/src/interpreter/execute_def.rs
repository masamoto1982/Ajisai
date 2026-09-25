use crate::error::{AjisaiError, Result};
use crate::interpreter::value_extraction_helpers::extract_word_name_from_value;
use crate::interpreter::{Interpreter, WordDefinition};
use crate::types::Token;
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

/// DEF is strictly two positional arguments: `[ body ] 'NAME' DEF`.
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
        return Err(AjisaiError::stack_underflow());
    }

    let name_val = interp.stack.pop().ok_or(AjisaiError::stack_underflow())?;
    let name_str = extract_word_name_from_value(&name_val)?;

    let def_val = interp.stack.pop().ok_or(AjisaiError::stack_underflow())?;

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
                    format!(
                        "expected a Vector [ ... ] definition body, got {}",
                        def_val.domain_name()
                    ),
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
        return Err(AjisaiError::declared(
            "protectedWord",
            format!("Cannot redefine Core Word '{}'", upper_name),
        ));
    }

    // The other half of `BIND`'s refusal to take a Word's name. Together they
    // keep the two name spaces disjoint at every moment, so a reader never has
    // to know which of the two a name resolved through.
    if interp.lookup_binding(&upper_name).is_some() {
        return Err(AjisaiError::declared("nameConflict", format!(
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

    if tokens.is_empty() {
        return Err(AjisaiError::declared(
            "invalidDefinitionBody",
            "expected a non-empty definition body, got an empty body",
        ));
    }

    // Content store (Section 8.6): share one stored body across textually
    // identical definitions so copying or re-importing a word group does not
    // duplicate its code.
    let body_key = crate::interpreter::word_identity::body_content_key(tokens);
    let body: Arc<[Token]> = match interp.body_store.get(&body_key) {
        Some(shared) => shared.clone(),
        None => {
            let arc: Arc<[Token]> = tokens.into();
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
    for token in body.iter() {
        if let Token::Symbol(s) = token {
            let upper_s = crate::core_word_aliases::canonicalize_core_word_name(s);
            new_text_references.insert(upper_s.to_string());
            if let Some((resolved_name, resolved_def)) = interp.resolve_word_entry(&upper_s) {
                if !resolved_def.is_builtin || resolved_name.contains('@') {
                    new_dependencies.insert(resolved_name.to_string());
                }
            }
        }
    }

    // Section 8.7: the User dictionary's reference graph is acyclic — no Word
    // may name itself, directly or through any chain of other User words.
    // Repetition is expressed only through the bounded higher-order Words
    // (`MAP`, `FILTER`, `FOLD`, `SCAN`) over an already-finite Vector,
    // never through a Word calling itself: every evaluation is then
    // structurally finite, not merely bounded by a runtime step budget.
    if let Some(cycle) = interp.find_reference_cycle(&upper_name, &new_text_references) {
        return Err(AjisaiError::declared(
            "selfReferentialDefinition",
            format!(
                "Cannot define '{}': the body names itself ({})",
                upper_name,
                cycle.join(" -> ")
            ),
        ));
    }

    for dep_name in &new_dependencies {
        interp
            .dependents
            .entry(dep_name.clone())
            .or_default()
            .insert(upper_name.clone());
    }

    let new_def = WordDefinition {
        body,
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
