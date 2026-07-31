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
use crate::interpreter::Interpreter;
use crate::tokenizer::tokenize;
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
///
/// The empty group has two spellings — `[]` and the spaced `[ ]` — and only
/// the first was recognized, so `[ x ] -> [ ]` read as one output instead of
/// none. Nothing caught it while `PRINT` carried a `Dynamic` mass and was
/// skipped; the declared 1 -> 0 arity engaged the check and exposed it.
fn count_stack_items(side: &str) -> Option<u16> {
    let side = side.replace("[ ]", "[]").replace("{ }", "{}");
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
/// Execute `code` on a fresh interpreter and return the render of its top stack
/// value, or `None` if it raised or left an empty stack.
async fn execute_top_render(code: &str) -> Option<String> {
    let mut interp = Interpreter::new();
    interp.execute(code).await.ok()?;
    crate::types::display::render_stack(interp.get_stack())
        .last()
        .cloned()
}

/// Execute `code` and return its top render *only if it produced exactly one
/// value* — used to interpret a documented `Pushes <value>.` as a single value.
async fn execute_single_value_render(code: &str) -> Option<String> {
    let mut interp = Interpreter::new();
    interp.execute(code).await.ok()?;
    let stack = crate::types::display::render_stack(interp.get_stack());
    (stack.len() == 1).then(|| stack[0].clone())
}

#[tokio::test]
async fn authored_example_results_match_execution() {
    // Structural-constraint ledger item 12b (convention -> structure): item 12
    // proved the authored `code` runs; this proves its stated `result` is
    // correct. When the result is a clean `Pushes <value>.`, the `<value>` is
    // itself Ajisai value syntax, so executing it yields the documented value —
    // and comparing it to the code's actual top through the *same* render path
    // needs no string normalization (an integer renders as `1/1` on both sides).
    // The check abstains whenever the result prose is not a clean single value
    // (an effect description, a ranged or multi-value result, or free prose like
    // "the first element, 10"), so it never raises a false mismatch.
    let mut compared = 0u32;
    for doc in super::builtin_word_lookup_docs::builtin_lookup_docs() {
        for example in doc.examples {
            let Some(value_src) = example
                .result
                .strip_prefix("Pushes ")
                .and_then(|rest| rest.strip_suffix('.'))
            else {
                continue; // not a "Pushes <value>." result
            };
            let Some(expected) = execute_single_value_render(value_src).await else {
                continue; // the documented result is not a single concrete value
            };
            let Some(actual) = execute_top_render(example.code).await else {
                continue; // the example itself does not leave a value (item 12's job)
            };
            compared += 1;
            assert_eq!(
                expected, actual,
                "{}: `{}` is documented to push `{}` (which renders as `{}`) \
                 but actually pushes `{}`",
                doc.word, example.code, value_src, expected, actual
            );
        }
    }
    assert!(
        compared >= 20,
        "authored-result value check only compared {compared} examples; \
         the `Pushes <value>.` extraction may have regressed"
    );
}
