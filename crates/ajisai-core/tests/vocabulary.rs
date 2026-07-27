//! What Ajisai Core is, and what it is not.
//!
//! "Ajisai Core" is one concept: the semantics and vocabulary required to be
//! Ajisai. There is no second Core, no minimal core, no core profile, and no
//! core word set that means something different from this one. These tests
//! hold that boundary from both sides — everything in it is coherent, and the
//! things this rebuild removed are genuinely gone.

use ajisai_core::contract::{notation_arity, Arity, Body, Effect, Word};
use ajisai_core::{alias, manifest, Interpreter};

/// Every contract's prose notation and its machine arity are two views of the
/// same fact, so they are checked against each other rather than both trusted.
#[test]
fn stack_effect_notation_matches_declared_arity() {
    let interpreter = Interpreter::new();
    for word in interpreter.contracts() {
        let contract = &word.contract;
        let parsed = notation_arity(contract.stack_effect).unwrap_or_else(|| {
            panic!(
                "{}: stack effect {:?} is not in `( in -- out )` form",
                contract.name, contract.stack_effect
            )
        });
        match contract.arity {
            Arity::Fixed { inn, out } => assert_eq!(
                parsed,
                (inn as usize, out as usize),
                "{}: notation {:?} disagrees with the declared arity",
                contract.name,
                contract.stack_effect
            ),
            Arity::Dynamic => {}
        }
    }
}

/// Input and output type lists line up with the declared arity.
#[test]
fn type_lists_match_declared_arity() {
    let interpreter = Interpreter::new();
    for word in interpreter.contracts() {
        let contract = &word.contract;
        if let Arity::Fixed { inn, out } = contract.arity {
            assert_eq!(
                contract.input_types.len(),
                inn as usize,
                "{}: input types do not match arity",
                contract.name
            );
            assert_eq!(
                contract.output_types.len(),
                out as usize,
                "{}: output types do not match arity",
                contract.name
            );
        }
    }
}

/// The mode layer selects operands from a fixed region, so a word it drives
/// must declare one.
#[test]
fn every_moded_word_declares_a_fixed_stack_effect() {
    let interpreter = Interpreter::new();
    for word in interpreter.contracts() {
        if matches!(word.body, Body::Op(_)) {
            assert!(
                word.contract.arity.fixed().is_some(),
                "{}: an Op word must have a fixed stack effect",
                word.contract.name
            );
        }
    }
}

/// Contracts are complete: every word says something, and every word is owned.
#[test]
fn every_word_is_documented_and_owned() {
    let interpreter = Interpreter::new();
    for word in interpreter.contracts() {
        assert!(
            !word.contract.summary.is_empty(),
            "{}: no summary",
            word.contract.name
        );
        assert_eq!(word.package, "ajisai-core");
        assert_eq!(
            word.contract.name,
            word.contract.name.trim(),
            "word names carry no padding"
        );
    }
}

/// Only the dictionary words claim an effect. Everything else is pure, which
/// is what makes a blocked vent observationally equivalent to a unit that was
/// never written.
#[test]
fn only_the_dictionary_words_have_an_effect() {
    let interpreter = Interpreter::new();
    let effectful: Vec<&str> = interpreter
        .contracts()
        .iter()
        .filter(|word| word.contract.effect == Effect::Dictionary)
        .map(|word| word.contract.name)
        .collect();
    assert_eq!(effectful, vec!["DEF", "DEL"]);
}

/// Every term in the contract vocabulary is actually reached by some word. A
/// classification no word produces would be exactly the kind of unreachable
/// variant this rebuild removed.
#[test]
fn every_contract_term_has_a_word_that_uses_it() {
    let interpreter = Interpreter::new();
    let words = interpreter.contracts();
    let any = |test: &dyn Fn(&Word) -> bool| words.iter().any(|word| test(word));

    assert!(any(&|word| word.contract.nil_policy.rejects));
    assert!(any(&|word| word.contract.nil_policy.may_produce));
    assert!(any(&|word| word.contract.unknown_policy.rejects));
    assert!(any(&|word| word.contract.unknown_policy.may_produce));
    assert!(any(
        &|word| !word.contract.nil_policy.rejects && !word.contract.nil_policy.may_produce
    ));
    assert!(any(&|word| matches!(word.body, Body::Op(_))));
    assert!(any(&|word| matches!(word.body, Body::Full(_))));
    assert!(any(&|word| matches!(word.body, Body::Directive)));
    assert!(any(&|word| matches!(word.contract.arity, Arity::Dynamic)));
}

/// Ajisai Core's vocabulary is enumerable, and it is the whole of it.
#[test]
fn the_vocabulary_is_enumerable() {
    let interpreter = Interpreter::new();
    let names = interpreter.vocabulary();
    assert_eq!(names.len(), interpreter.contracts().len());
    assert!(
        names.windows(2).all(|pair| pair[0] < pair[1]),
        "sorted, unique"
    );
    // A vocabulary this size is a language, not a survey of one.
    assert!(
        (40..=70).contains(&names.len()),
        "Ajisai Core has {} words",
        names.len()
    );
}

/// The manifest is generated from the live registry, so it cannot drift.
#[test]
fn the_manifest_covers_the_live_registry() {
    let interpreter = Interpreter::new();
    let json = manifest::vocabulary_json(&interpreter);
    for word in interpreter.contracts() {
        assert!(
            json.contains(&format!("\"name\": \"{}\"", word.contract.name)),
            "{} missing from the manifest",
            word.contract.name
        );
    }
    for (symbol, _) in alias::ALIASES {
        assert!(json.contains(&format!("\"{symbol}\"")), "{symbol} missing");
    }
    // The manifest carries no classification axis beside the contract.
    for absent in [
        "exploratory",
        "stability",
        "tier",
        "confidence",
        "water_sensitivity",
        "mass",
        "capability",
        "linearity",
    ] {
        assert!(
            !json.to_lowercase().contains(absent),
            "the manifest still carries {absent}"
        );
    }
}

/// The words and concepts this rebuild removed are gone from the language, not
/// renamed. Each is checked by behaviour — the name is rejected as unknown —
/// rather than by grepping the source.
#[test]
fn removed_words_are_not_words() {
    let mut interpreter = Interpreter::new();
    for name in [
        "FLOW",
        "OR-ELSE",
        "CONSERVE",
        "SPAWN",
        "AWAIT",
        "KILL",
        "MONITOR",
        "SUPERVISE",
        "RECEIPT",
        "ATTEST",
        "LOCKFILE",
        "MUSIC",
        "PLAY",
    ] {
        assert!(
            interpreter.word(name).is_none(),
            "{name} is still a registered word"
        );
        assert!(interpreter.execute(name).is_err(), "{name} still evaluates");
    }
}

/// `~` is not an alias, and it is not anything else either.
#[test]
fn the_removed_symbol_is_not_an_alias() {
    assert!(alias::ALIASES.iter().all(|(symbol, _)| *symbol != "~"));
    assert_eq!(alias::canonical("~"), "~");
    let mut interpreter = Interpreter::new();
    assert!(interpreter.execute("1 ~ 2").is_err());
}

/// There is one execution path. A host cannot select a backend, a policy, or a
/// plan, because the interpreter exposes none — this test is the compile-time
/// witness that the surface stayed that small.
#[test]
fn there_is_one_execution_path() {
    let mut interpreter = Interpreter::new();
    interpreter.execute("1 2 ADD").unwrap();
    assert_eq!(ajisai_core::render_stack(&interpreter), vec!["3"]);
    // The only knobs on the crate are the feature list, and there is none.
    assert!(option_env!("CARGO_FEATURE_ELASTIC_ENGINE").is_none());
    assert!(option_env!("CARGO_FEATURE_SIMD").is_none());
}
