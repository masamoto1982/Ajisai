//! What Ajisai Core is, and what it is not.
//!
//! "Ajisai Core" is one concept: the semantics and vocabulary required to be
//! Ajisai. There is no second Core, no minimal core, no core profile, and no
//! core word set that means something different from this one. These tests
//! hold that boundary from both sides — everything in it is coherent, and the
//! things this rebuild removed are genuinely gone.

use ajisai_core::contract::{notation_arity, Arity, Body, StakSupport, TypeSpec, Word};
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

/// A word's stack effect is a fact about the language; how it is dispatched is
/// a fact about this implementation. Conflating them made the lint go blind at
/// every higher-order word for no reason, so `EXEC` is now the only word in
/// Ajisai Core whose effect is genuinely dynamic.
#[test]
fn dispatch_does_not_decide_the_stack_effect() {
    let interpreter = Interpreter::new();
    let dynamic: Vec<&str> = interpreter
        .contracts()
        .iter()
        .filter(|word| matches!(word.contract.arity, Arity::Dynamic))
        .map(|word| word.contract.name)
        .collect();
    assert_eq!(dynamic, vec!["EXEC"]);
    for name in ["MAP", "FILTER", "FOLD", "DEF", "DEL", "DEPTH"] {
        let word = interpreter.word(name).expect(name);
        assert!(matches!(word.body, Body::Full(_)), "{name} is a Full word");
        assert!(
            word.contract.arity.fixed().is_some(),
            "{name} has a perfectly definite stack effect"
        );
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

/// What `STAK` means for a word is declared by the word, and the declaration
/// is held to a rule rather than taken on trust.
///
/// `FoldLeft` demands a **closed** operation: the result of one step has to be
/// a legitimate operand for the next. Deriving this from arity alone — "two in,
/// one out, therefore foldable" — was the same mistake as Flow Mass
/// Conservation, and it made `1 1 1 STAK EQ` answer `FALSE`.
#[test]
fn stak_support_is_declared_and_well_formed() {
    let interpreter = Interpreter::new();
    for word in interpreter.contracts() {
        let contract = &word.contract;
        match contract.stak {
            StakSupport::Unsupported => {}
            _ => assert!(
                matches!(word.body, Body::Op(_)),
                "{}: only an operand-to-result word can be driven across a flow",
                contract.name
            ),
        }
        match contract.stak {
            StakSupport::MapEach => assert_eq!(
                contract.arity.fixed().map(|(inn, _)| inn),
                Some(1),
                "{}: MapEach needs exactly one input",
                contract.name
            ),
            StakSupport::FoldLeft => {
                assert_eq!(
                    contract.arity.fixed(),
                    Some((2, 1)),
                    "{}: FoldLeft needs two in and one out",
                    contract.name
                );
                assert_eq!(
                    contract.input_types.first(),
                    contract.output_types.first(),
                    "{}: FoldLeft needs a closed operation — its result must be a \
                     legitimate operand for the next step",
                    contract.name
                );
            }
            StakSupport::Unsupported => {}
        }
    }
}

/// The comparison words are the ones the derived rule got wrong, so name them:
/// they take two and leave one, and folding them across a flow says nothing.
#[test]
fn comparison_words_are_not_foldable() {
    let interpreter = Interpreter::new();
    for name in ["EQ", "NE", "LT", "LE", "GT", "GE", "NTH", "APPEND", "RANGE"] {
        let word = interpreter.word(name).expect(name);
        assert_eq!(
            word.contract.stak,
            StakSupport::Unsupported,
            "{name} must not be foldable across a flow"
        );
    }
}

/// The Semantic Plane is read by exactly two words, and the specification says
/// which. A third would be a language change.
#[test]
fn the_semantic_plane_is_read_by_exactly_two_words() {
    let interpreter = Interpreter::new();
    let readers: Vec<&str> = interpreter
        .contracts()
        .iter()
        .filter(|word| word.contract.role_required.is_some())
        .map(|word| word.contract.name)
        .collect();
    assert_eq!(readers, vec!["DEF", "DEL"]);
    for name in ["DEF", "DEL"] {
        let contract = &interpreter.word(name).expect(name).contract;
        let (position, role) = contract.role_required.expect("declared");
        assert_eq!(role, ajisai_core::Role::Text);
        assert_eq!(contract.input_types[position], TypeSpec::Text);
    }
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
    assert!(any(&|word| word.contract.stak == StakSupport::MapEach));
    assert!(any(&|word| word.contract.stak == StakSupport::FoldLeft));
    assert!(any(&|word| word.contract.stak == StakSupport::Unsupported));
    assert!(any(&|word| word.contract.role_required.is_some()));
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
