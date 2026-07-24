//! Structural consistency checks for built-in `hover_syntax` examples
//! (structural-constraint ledger items 9 and 10; see
//! `docs/dev/structural-constraint-ledger.md`). Kept in a sibling file so
//! `builtin_word_details.rs` stays within the §14.1 file-size budget.
//!
//! These convert three invariants from authoring convention into a build-time
//! guarantee: a `hover_syntax` example must be a well-formed snippet (item 9),
//! every word it names must be a real word (item 10), and every *concrete*
//! example must actually run (item 10b).

use super::builtin_word_definitions::builtin_specs;
use crate::core_word_aliases::canonicalize_core_word_name;
use crate::coreword_registry::get_coreword_metadata;
use crate::interpreter::Interpreter;
use crate::tokenizer::tokenize;
use crate::types::Token;

/// A `hover_syntax` is *schematic* — a syntax template rather than a concrete
/// runnable program — when it starts with a bare modifier (the modifier words
/// `. , .. ,, !` demo their own syntax on an operand-less word) or when it
/// contains the ellipsis `...` ("your code here", e.g. `UNFOLD`, `PRECOMPUTE`).
/// Both markers are structural and unambiguous, so excluding them from the
/// execution check keeps it free of false failures while still requiring every
/// non-schematic example to run.
fn is_schematic(hover_syntax: &str) -> bool {
    if hover_syntax.contains("...") {
        return true;
    }
    let Ok(tokens) = tokenize(hover_syntax) else {
        return false;
    };
    match tokens.first() {
        Some(Token::Symbol(s)) => matches!(
            canonicalize_core_word_name(s).as_ref(),
            "TOP" | "EAT" | "STAK" | "KEEP" | "FORC"
        ),
        _ => false,
    }
}

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

#[tokio::test]
async fn every_concrete_hover_syntax_runs() {
    // Ledger item 10b. Items 9/10 guarantee every example tokenizes and names
    // only real words; this goes one step further and requires every *concrete*
    // (non-schematic) example to actually execute without a channel error. A
    // Bubble/NIL result is fine — that is a value, not a failure — so this only
    // rejects a raised error (a malformed or non-self-contained example). Each
    // runs on a fresh interpreter, so effectful examples (PRINT, DEF, IMPORT)
    // stay isolated.
    for spec in builtin_specs() {
        if spec.hover_syntax.is_empty() || is_schematic(spec.hover_syntax) {
            continue;
        }
        let mut interp = Interpreter::new();
        let result = interp.execute(spec.hover_syntax).await;
        assert!(
            result.is_ok(),
            "{}: hover_syntax `{}` does not run: {}",
            spec.name,
            spec.hover_syntax,
            result.unwrap_err()
        );
    }
}

/// Parse the `(consumes, produces)` arity from a `stack_effect` prose string,
/// or `None` when the prose is not in the machine-checkable subset (so the
/// caller abstains rather than risk a false mismatch). The DSL is `LHS -> RHS`,
/// where each side is a sequence of items: a bracketed group `[ … ]` / `{ … }`
/// counts as one stack slot, an empty group `[]` counts as zero, and a variadic
/// (`...`), annotated (`(…)`), or multi-arrow prose form abstains.
fn parse_stack_effect_arity(stack_effect: &str) -> Option<(u16, u16)> {
    if stack_effect == "no values popped or pushed" {
        return Some((0, 0));
    }
    let sides: Vec<&str> = stack_effect.split(" -> ").collect();
    if sides.len() != 2 {
        return None; // no single arrow: prose or a control-directive description
    }
    for side in &sides {
        if side.contains("...") || side.contains('(') {
            return None; // variadic or annotated: not a fixed arity
        }
    }
    Some((count_stack_items(sides[0])?, count_stack_items(sides[1])?))
}

/// Count top-level stack items in one side of a `stack_effect`. A new item
/// begins at each token seen at bracket depth 0; an empty group contributes
/// nothing. Unbalanced brackets abstain (`None`).
fn count_stack_items(side: &str) -> Option<u16> {
    let mut depth = 0i32;
    let mut count = 0u16;
    for token in side.split_whitespace() {
        if token == "[]" || token == "{}" {
            continue; // an empty group produces/consumes nothing
        }
        if depth == 0 {
            count += 1;
        }
        for ch in token.chars() {
            match ch {
                '[' | '{' => depth += 1,
                ']' | '}' => depth -= 1,
                _ => {}
            }
        }
        if depth < 0 {
            return None;
        }
    }
    (depth == 0).then_some(count)
}

#[test]
fn fixed_stack_effect_prose_matches_the_machine_mass() {
    // Structural-constraint ledger item 11 (convention -> structure): the
    // human-facing `stack_effect` prose and the machine `mass` contract (SPEC
    // §13.1) are two descriptions of one word's arity that could drift. For
    // every word with a `Fixed` mass, the arity parsed from the prose must equal
    // the mass. The parser abstains (skips) on any prose outside its
    // machine-checkable subset, so this never raises a false mismatch; it only
    // fires when the two descriptions provably disagree.
    let mut compared = 0u32;
    for spec in builtin_specs() {
        let Some((mass_consumes, mass_produces)) =
            crate::coreword_registry::mass_contract(spec.name).fixed()
        else {
            continue; // Dynamic mass: no fixed arity to check against
        };
        let Some((prose_consumes, prose_produces)) = parse_stack_effect_arity(spec.stack_effect)
        else {
            continue; // prose outside the machine-checkable subset: abstain
        };
        compared += 1;
        assert_eq!(
            (prose_consumes, prose_produces),
            (u16::from(mass_consumes), u16::from(mass_produces)),
            "{}: stack_effect `{}` reads as arity ({}, {}) but mass is ({}, {})",
            spec.name,
            spec.stack_effect,
            prose_consumes,
            prose_produces,
            mass_consumes,
            mass_produces
        );
    }
    // Guard against the check silently going vacuous (e.g. if the parser starts
    // abstaining on everything): a healthy share of the fixed-mass words must
    // actually be compared. There are ~25 today; require a conservative floor.
    assert!(
        compared >= 20,
        "stack_effect/mass cross-check only compared {compared} words; \
         the prose parser may have regressed into abstaining"
    );
}
