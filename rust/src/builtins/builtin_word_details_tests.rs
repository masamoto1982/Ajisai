//! Structural consistency checks for built-in `hover_syntax` examples
//! (structural-constraint ledger items 9 and 10; see
//! `docs/dev/structural-constraint-ledger.md`). Kept in a sibling file so
//! `builtin_word_details.rs` stays within the file-size budget in docs/dev/specification-implementation-rules.md.
//!
//! These convert three invariants from authoring convention into a build-time
//! guarantee: a `hover_syntax` example must be a well-formed snippet (item 9),
//! every word it names must be a real word (item 10), and every *concrete*
//! example must actually run (item 10b).
//!
//! The declared-effect guard below is here for the same reason: it keeps the
//! LOOKUP "Side Effects" prose total over what `spec/words.json` declares.

use super::builtin_word_definitions::builtin_specs;
use super::builtin_word_details::effect_sentence;
use crate::interpreter::Interpreter;
use crate::kernel::generated::GENERATED_WORDS;
use crate::tokenizer::tokenize;

/// The effect names are the specification's, not the runtime's, so a Word
/// gaining an effect in `spec/words.json` reaches the LOOKUP prose without any
/// Rust edit — and would print the raw protocol name if nobody wrote a sentence
/// for it. Requiring a sentence for every declared effect makes that a test
/// failure instead of a reader-visible `consoleWrite`.
#[test]
fn every_declared_effect_has_a_user_facing_sentence() {
    let mut checked = 0;
    for word in GENERATED_WORDS {
        for effect in word.effects {
            checked += 1;
            assert!(
                effect_sentence(effect).is_some(),
                "{} declares effect `{}` with no user-facing sentence",
                word.name,
                effect
            );
        }
    }
    assert!(checked > 0, "no Word declares an effect; the guard is idle");
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
#[tokio::test]
async fn every_hover_syntax_calls_its_word_and_runs() {
    // Ledger items 10 and 10b. A `hover_syntax` is also the "one correct call"
    // a diagnosis quotes, so it must be one: it ends in the Word's own
    // canonical name — never an alias, which is a second spelling of the same
    // Word — and it runs on a fresh interpreter. FAIL's one correct call is the
    // ERROR it exists to raise.
    let aliases: Vec<&str> = GENERATED_WORDS
        .iter()
        .flat_map(|word| word.aliases.iter().copied())
        .collect();
    let mut ran = 0u32;
    for spec in builtin_specs() {
        if spec.hover_syntax.is_empty() {
            continue;
        }
        assert!(
            !spec
                .hover_syntax
                .split_whitespace()
                .any(|token| aliases.contains(&token)),
            "{}: hover_syntax `{}` spells a Word by an alias",
            spec.name,
            spec.hover_syntax
        );
        assert_eq!(
            spec.hover_syntax.split_whitespace().last(),
            Some(spec.name),
            "{}: hover_syntax `{}` does not end in the Word's canonical name",
            spec.name,
            spec.hover_syntax
        );
        let mut interp = Interpreter::new();
        let outcome = interp.execute(spec.hover_syntax).await;
        if spec.name == "FAIL" {
            let err = outcome.expect_err("FAIL's hover_syntax must raise");
            assert_eq!(
                crate::error::ErrorCategory::from_error(&err).as_protocol_str(),
                "declaredFailure"
            );
        } else {
            assert!(
                outcome.is_ok(),
                "{}: hover_syntax `{}` does not run: {:?}",
                spec.name,
                spec.hover_syntax,
                outcome.err()
            );
        }
        ran += 1;
    }
    assert!(ran >= 70, "only {ran} hover_syntax examples ran");
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
    // LANG.MACHINE.WORD) are two descriptions of one word's arity that could drift. For
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
/// The rendered stack `code` leaves on a fresh interpreter, or `None` if it
/// raised.
async fn run_render(code: &str) -> Option<Vec<String>> {
    let mut interp = Interpreter::new();
    interp.execute(code).await.ok()?;
    Some(crate::types::display::render_stack(interp.get_stack()))
}

/// Every "`code` is `value`" a summary states, as `(code, value)`.
fn stated_examples(summary: &str) -> Vec<(&str, &str)> {
    let parts: Vec<&str> = summary.split('`').collect();
    (1..parts.len().saturating_sub(2))
        .step_by(2)
        .filter(|&k| parts[k + 1] == " is ")
        .map(|k| (parts[k], parts[k + 2]))
        .collect()
}

#[tokio::test]
async fn every_summary_example_holds() {
    // A summary is the one prose source for what a Word does (LOOKUP, hover,
    // SKILL.md, the MCP quickstart all render it), so what it states is
    // executed: every "`code` is `value`" in it must leave exactly the stack
    // `value` leaves, and every Word whose call leaves a value states at
    // least one. PRINT and FAIL leave none — one writes, the other raises.
    for word in builtin_specs() {
        let examples = stated_examples(word.summary);
        if !matches!(word.name, "PRINT" | "FAIL") {
            assert!(
                !examples.is_empty(),
                "{}: the summary states no `code` is `value` example",
                word.name
            );
        }
        for (code, value) in examples {
            let actual = run_render(code).await;
            let expected = run_render(value).await;
            assert!(
                actual.is_some() && actual == expected,
                "{}: the summary says `{code}` is `{value}`, but they leave {actual:?} and {expected:?}",
                word.name
            );
        }
    }
}
