//! Property-based name-resolution / dictionary laws (Phase 6).
//!
//! Encodes the algebraic content of
//! `docs/dev/ajisai-mathematical-formalization.md` §9-quinquies F (Phase 6):
//! the dictionary `Dict = Name ⇀ Blk`, the deterministic resolver
//! `resolve : Name × Vis ⇀ Blk + Unknown` with order **Core → user**, and
//! `DEF`/`DEL` as state transducers with a dependency guard (SPEC §8.2).
//!
//! Every law was checked against the reference implementation with a throwaway
//! probe (`_probe_naming.rs`, deleted) before being written (roadmap §1.2-(T)
//! discipline).
//!
//! Observation stays firewall-clean: laws compare whole-stack renders / the
//! Ok-vs-Err resolution outcome, never a Rust enum or `Debug` string.

mod test_support;

use ajisai_core::interpreter::Interpreter;
use proptest::prelude::*;
use test_support::generators::{small, user_word_body, user_word_name};
use test_support::observe::{render, run, run_err};

// ─────────────────────────── observation helpers ───────────────────────────

/// Whole-stack rendering (one value per element), the conformance observation.
fn obs(src: &str) -> Vec<String> {
    run(src).iter().map(|v| render(v, v.hint)).collect()
}

/// The resolution *outcome* of a program: `Ok(stack-render)` when every word
/// resolved and ran, `Err(())` when resolution (or execution) failed. This is
/// the firewall-clean way to observe "does this name resolve here" — we never
/// inspect the error text, only whether a binding was found.
fn outcome(src: &str) -> Result<Vec<String>, ()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("tokio current-thread runtime");
    rt.block_on(async {
        let mut interp = Interpreter::new();
        match interp.execute(src).await {
            Ok(()) => Ok(interp
                .get_stack()
                .iter()
                .map(|v| render(v, v.hint))
                .collect()),
            Err(_) => Err(()),
        }
    })
}

// ───────────────── resolve : Name × Vis ⇀ Blk + Unknown (§7/§9) ──────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]
    /// **User never shadows Core** (LANG.DICTIONARY.RESOLUTION). Core is
    /// sealed: defining a user word over a Core name is rejected outright, so
    /// a Core name always means the Core Word.
    #[test]
    fn user_definition_cannot_shadow_a_core_word(n in 2i64..12) {
        let sq = n * n;
        assert!(
            outcome("{ 99 ADD } 'SQRT' DEF").is_err(),
            "defining a user word over the Core name SQRT must fail"
        );
        prop_assert_eq!(obs(&format!("{sq} SQRT")), vec![format!("{n}/1")]);
    }
}

// ─────────────── DEF / DEL as Dict state transducers (§8) ────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(40))]

    /// **DEF makes a name resolvable; the defined word equals its inlined
    /// body.** `{body} 'W' DEF  x W  ≡  x body` — defining then calling is the
    /// identity on the body transducer (SPEC §8.1).
    #[test]
    fn def_then_call_inlines_body(name in user_word_name(), (body, inline) in user_word_body(), x in small()) {
        let defined = obs(&format!("{{ {body} }} '{name}' DEF {x} {name}"));
        let inlined = obs(&format!("{x} {inline}"));
        prop_assert_eq!(defined, inlined);
    }

    /// **DEF then DEL is the identity on resolution.** A name is `Unknown`
    /// before definition and `Unknown` again after deletion — `DEL` is the left
    /// inverse of `DEF` on the visibility of a fresh name (SPEC §8.3).
    #[test]
    fn def_del_round_trip_restores_unknown(name in user_word_name(), (body, _i) in user_word_body(), x in small()) {
        let fresh = format!("{x} {name}");
        let defined = format!("{{ {body} }} '{name}' DEF {x} {name}");
        let def_del = format!("{{ {body} }} '{name}' DEF '{name}' DEL {x} {name}");
        // Fresh name resolves to Unknown.
        prop_assert!(outcome(&fresh).is_err());
        // Defined → resolves.
        prop_assert!(outcome(&defined).is_ok());
        // Defined then deleted → Unknown again.
        prop_assert!(outcome(&def_del).is_err());
    }
}

// ───────────────── dependency guard — DEL refuses while referenced ────────────

/// **DEL refuses while a dependent exists.** There is no force modifier: a
/// referenced word cannot be deleted, so a dangling reference is unreachable
/// (LANG.DICTIONARY.MUTATION).
#[test]
fn delete_with_dependents_is_refused() {
    let referenced = "{ 1 ADD } 'INC' DEF { INC INC } 'INC2' DEF 'INC' DEL";
    assert!(
        outcome(referenced).is_err(),
        "deleting a referenced word must fail"
    );

    // Removing the dependent first is what makes the delete legal.
    let ordered = "{ 1 ADD } 'INC' DEF { INC INC } 'INC2' DEF 'INC2' DEL 'INC' DEL 42";
    assert_eq!(outcome(ordered), Ok(vec!["42/1".to_string()]));
}

/// **Core words cannot be redefined** (LANG.DICTIONARY.RESOLUTION): `DEF` of a
/// Core name is rejected outright. Core is sealed from user space.
#[test]
fn builtin_words_cannot_be_redefined() {
    for w in ["ADD", "GET", "EQ"] {
        assert!(
            outcome(&format!("{{ 0 }} '{w}' DEF")).is_err(),
            "redefining built-in {w} must be rejected"
        );
    }
}

// ────────────────── local bindings — a name with a frame's life ──────────────

/// **A binding names a value, and the name is the value wherever it is
/// written.** This is the whole point: the stack answers only for the value on
/// top, so an expression that uses one value twice had no way to say so.
#[test]
fn a_binding_names_a_value_for_the_rest_of_the_frame() {
    assert_eq!(obs("5 'T' BIND T T ADD"), vec!["10/1"]);
    // Used three times, in an order the stack could not have produced.
    assert_eq!(obs("[ 2 7 ] [ 'W' 'B' ] BIND W B ADD W MUL"), vec!["18/1"]);
    // A single-name Vector is the single-name case, as with `GET`'s index.
    assert_eq!(obs("[ 1 2 ] [ 'A' ] BIND A"), vec!["[ 1/1 2/1 ]"]);
    // Both operands are consumed: a binding leaves nothing behind.
    assert_eq!(obs("9 5 'T' BIND"), vec!["9/1"]);
}

/// **A binding reaches the blocks written in its frame.** A block a Core Word
/// applies is not a new frame for names — which is what lets a threshold
/// learned at run time be used inside a predicate, the thing `REFLECT` partial
/// application was the only answer to.
#[test]
fn a_binding_reaches_blocks_written_in_its_frame() {
    assert_eq!(
        obs("3 'T' BIND [ 1 2 3 ] { T MUL } MAP"),
        vec!["[ 3/1 6/1 9/1 ]"]
    );
    assert_eq!(obs("3 'T' BIND [ 1 5 9 ] { T LT } FILTER"), vec!["[ 1/1 ]"]);
    assert_eq!(obs("4 'T' BIND [ 1 2 ] 0 { ADD T ADD } FOLD"), vec!["11/1"]);
    assert_eq!(obs("5 'T' BIND { T } EXEC"), vec!["5/1"]);
    // A COND clause is such a block: the isolation COND enforces is of the
    // stack, and a name is not on the stack.
    assert_eq!(
        obs("5 'T' BIND T { 1 GT } { T } { IDLE } { 0 } COND"),
        vec!["5/1"]
    );
    // A block that binds runs in its own scope per evaluation, so the name is
    // fresh each element rather than a collision on the second.
    assert_eq!(
        obs("5 'T' BIND [ 1 2 ] { 'E' BIND E T MUL } MAP"),
        vec!["[ 5/1 10/1 ]"]
    );
}

/// **A binding does not cross a Word call.** A Word's meaning is a function of
/// its operands and the dictionary; if a caller's names leaked in, it would not
/// be. The diagnostic says which rule was met rather than claiming the name
/// does not exist.
#[test]
fn a_binding_does_not_cross_a_word_call() {
    let message = run_err("{ T } 'READS-T' DEF 5 'T' BIND READS-T");
    assert!(
        message.contains("bound in another frame"),
        "expected the scope rule, got: {message}"
    );
    // The value reaches the Word the ordinary way, as an operand.
    assert_eq!(obs("{ 1 ADD } 'INC' DEF 5 'T' BIND T INC"), vec!["6/1"]);
}

/// **A binding ends with its frame.** A Word that binds leaves no name behind
/// for its caller, and a run's bindings do not survive into the next one — a
/// binding that outlived its frame would be the second name space this design
/// exists not to be.
#[test]
fn a_binding_ends_with_its_frame() {
    let message = run_err("{ 5 'INNER' BIND INNER } 'MAKES' DEF MAKES INNER");
    assert!(!message.is_empty(), "INNER must not survive the call");
    // The Word itself still works; only the name is gone afterwards.
    assert_eq!(
        obs("{ 5 'INNER' BIND INNER } 'MAKES' DEF MAKES"),
        vec!["5/1"]
    );
}

/// **A name is a Word or a binding, never both** (LANG.DICTIONARY.RESOLUTION).
/// Both directions are refused, so resolution order settles no contest and a
/// reader never has to know which one they got.
#[test]
fn a_binding_and_a_word_may_not_share_a_name() {
    for word in ["ADD", "GET", "COND"] {
        assert!(
            run_err(&format!("5 '{word}' BIND")).contains("may not shadow"),
            "a binding must not take the Core name {word}"
        );
    }
    assert!(run_err("{ 1 } 'Q' DEF 5 'Q' BIND").contains("User Word"));
    assert!(run_err("5 'T' BIND { 1 } 'T' DEF").contains("bound in this frame"));
}

/// **Destructuring is exact.** A Vector longer than the name list would drop
/// its tail silently and a shorter one would name nothing; both are the caller
/// having said something other than what they meant.
#[test]
fn destructuring_requires_one_name_per_element() {
    assert!(!run_err("[ 1 2 3 ] [ 'W' 'B' ] BIND").is_empty());
    assert!(!run_err("[ 1 ] [ 'W' 'B' ] BIND").is_empty());
    assert_eq!(obs("[ 1 2 3 ] [ 'A' 'B' 'C' ] BIND C A ADD"), vec!["4/1"]);
}

/// A bound bubble stays a bubble, with its reason, however many times the name
/// is read. Binding is not an observation of the value (SPEC §7.12).
#[test]
fn binding_preserves_an_absence_and_its_reason() {
    assert_eq!(
        obs("1 0 DIV 'B' BIND B NIL-REASON"),
        vec!["NIL", "'divisionByZero'"]
    );
    assert_eq!(obs("1 0 DIV 'B' BIND B B 2 COLLECT"), vec!["[ NIL NIL ]"]);
}

/// Recursion still terminates, and the trampolined tail call starts each round
/// with no names — the frame outlives the iteration it belongs to, so the
/// binding must not.
#[test]
fn a_binding_survives_neither_a_recursive_call_nor_a_tail_jump() {
    let sumto = "{ 'N' BIND N { 0 EQ } { 0 } { IDLE } { N N 1 SUB SUMTO ADD } COND } 'SUMTO' DEF ";
    assert_eq!(obs(&format!("{sumto} 5 SUMTO")), vec!["15/1"]);

    // Past MAX_USER_WORD_DEPTH, so this is the backward jump rather than a call.
    let countdown = "{ [ 'ACC' 'N' ] BIND N { 0 EQ } { ACC } \
                     { IDLE } { ACC N ADD N 1 SUB 2 COLLECT COUNTDOWN } COND } 'COUNTDOWN' DEF ";
    assert_eq!(
        obs(&format!("{countdown} [ 0 2000 ] COUNTDOWN")),
        vec!["2001000/1"]
    );
}
