//! Structural consistency checks for built-in `hover_syntax` examples
//! (structural-constraint ledger items 9 and 10; see
//! `docs/dev/structural-constraint-ledger.md`). Kept in a sibling file so
//! `builtin_word_details.rs` stays within the §14.1 file-size budget.
//!
//! These convert two invariants from authoring convention into a build-time
//! guarantee: a `hover_syntax` example must be a well-formed snippet, and every
//! word it names must be a real word.

use super::builtin_word_definitions::builtin_specs;
use crate::core_word_aliases::canonicalize_core_word_name;
use crate::coreword_registry::get_coreword_metadata;
use crate::tokenizer::tokenize;
use crate::types::Token;

#[test]
fn every_hover_syntax_is_a_well_formed_snippet() {
    // Ledger item 9. A `hover_syntax` is a runnable example, so requiring it to
    // tokenize makes well-formedness a build-time guarantee. Only tokenization
    // is sound to require of all of them — some are deliberate modifier fragments
    // (`. +`); symbol resolution is the sibling check below (item 10).
    for spec in builtin_specs() {
        if spec.hover_syntax.is_empty() {
            continue;
        }
        assert!(
            tokenize(spec.hover_syntax).is_ok(),
            "{}: hover_syntax `{}` does not tokenize (malformed doc example)",
            spec.name,
            spec.hover_syntax
        );
    }
}

#[test]
fn every_hover_syntax_names_only_real_words() {
    // Ledger item 10. Every word a `hover_syntax` names must be a real word: a
    // `Symbol` token, after alias canonicalization, must resolve in the Coreword
    // registry (covering operators like `+`, modifiers like `. ,,`, casts like
    // `>CF`, and `@`-module words like `MATH@SQRT`, which all canonicalize to
    // registry entries). This catches a doc example referencing a removed or
    // misspelled word, and it forces every example to be a concrete runnable
    // snippet rather than a schematic one with metavariable placeholders
    // (`a b 64 COMPARE-WITHIN`), which never ran.
    for spec in builtin_specs() {
        let Ok(tokens) = tokenize(spec.hover_syntax) else {
            continue; // malformed snippets are the sibling test's job (item 9)
        };
        for token in &tokens {
            let Token::Symbol(name) = token else {
                continue;
            };
            let canonical = canonicalize_core_word_name(name);
            assert!(
                get_coreword_metadata(&canonical).is_some(),
                "{}: hover_syntax `{}` names `{}`, which is not a real word \
                 (a typo, a removed word, or a schematic placeholder)",
                spec.name,
                spec.hover_syntax,
                name
            );
        }
    }
}
