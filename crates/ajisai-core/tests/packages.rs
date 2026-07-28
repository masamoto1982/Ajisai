//! The package boundary: registration is checked first, and committed whole.

use ajisai_core::contract::{Arity, Body, Policy, StakSupport, TypeSpec, WordContract};
use ajisai_core::extension::Package;
use ajisai_core::{Error, Interpreter, Result, Value};

fn double(word: &str, args: &[Value]) -> Result<Vec<Value>> {
    let [a] = args else {
        return Err(Error::StackUnderflow {
            word: word.to_string(),
            needed: 1,
            found: args.len(),
        });
    };
    let n = a.as_number().ok_or_else(|| Error::TypeMismatch {
        word: word.to_string(),
        expected: "number".to_string(),
        found: a.type_name().to_string(),
    })?;
    Ok(vec![Value::number(n * &ajisai_core::Number::integer(2))])
}

fn contract(name: &'static str) -> WordContract {
    WordContract {
        name,
        stack_effect: "( a -- b )",
        arity: Arity::Fixed { inn: 1, out: 1 },
        input_types: &[TypeSpec::Number],
        output_types: &[TypeSpec::Number],
        nil_policy: Policy::REFUSES,
        unknown_policy: Policy::REFUSES,
        stak: StakSupport::MapEach,
        role_required: None,
        summary: "Twice.",
    }
}

/// Registration is all or nothing. A package whose second word collides must
/// not leave its first word installed.
#[test]
fn a_rejected_package_installs_nothing() {
    let mut interpreter = Interpreter::new();
    let package = Package::new("test")
        .with(contract("TEST:FIRST"), Body::Op(double))
        // `ADD` is Ajisai Core's, so the package is rejected here.
        .with(contract("ADD"), Body::Op(double));
    assert!(matches!(
        interpreter.register_package(package),
        Err(Error::DuplicateWord { .. })
    ));
    assert!(
        interpreter.word("TEST:FIRST").is_none(),
        "the first word must not survive the rejection"
    );
    assert!(interpreter.execute("1 TEST:FIRST").is_err());
    // ...and Ajisai Core is untouched.
    interpreter.execute("1 2 ADD").expect("ADD still means ADD");
}

/// A package may not collide with itself either.
#[test]
fn a_package_may_not_repeat_a_name() {
    let mut interpreter = Interpreter::new();
    let package = Package::new("test")
        .with(contract("TEST:ONE"), Body::Op(double))
        .with(contract("TEST:ONE"), Body::Op(double));
    assert!(matches!(
        interpreter.register_package(package),
        Err(Error::DuplicateWord { .. })
    ));
    assert!(interpreter.word("TEST:ONE").is_none());
}

/// The dictionary is consulted before the registry, so registering a name a
/// user definition already answers to would install a word nothing could
/// reach. Refusing says so instead.
#[test]
fn a_package_may_not_hide_behind_a_user_definition() {
    let mut interpreter = Interpreter::new();
    interpreter
        .execute("{ 99 } \"TEST:TAKEN\" DEF")
        .expect("defines");
    let package = Package::new("test").with(contract("TEST:TAKEN"), Body::Op(double));
    assert!(matches!(
        interpreter.register_package(package),
        Err(Error::DuplicateWord { .. })
    ));
    interpreter.execute("TEST:TAKEN").expect("still the user's");
}

/// A contract that does not describe its word is caught at registration,
/// before it can mislead the mode layer or the lint. Ajisai Core holds itself
/// to these rules in a test; a package has no test of ours to run.
#[test]
fn a_malformed_contract_is_refused() {
    let cases: Vec<(&str, WordContract)> = vec![
        ("notation disagrees with arity", {
            let mut c = contract("TEST:A");
            c.stack_effect = "( a b -- c )";
            c
        }),
        ("type lists do not match arity", {
            let mut c = contract("TEST:B");
            c.input_types = &[];
            c
        }),
        ("name is not canonical", {
            let mut c = contract("test:c");
            c.stack_effect = "( a -- b )";
            c
        }),
        ("name is a directive", contract("KEEP")),
        ("no summary", {
            let mut c = contract("TEST:D");
            c.summary = "";
            c
        }),
        ("MapEach on a two-in word", {
            let mut c = contract("TEST:E");
            c.arity = Arity::Fixed { inn: 2, out: 1 };
            c.stack_effect = "( a b -- c )";
            c.input_types = &[TypeSpec::Number, TypeSpec::Number];
            c
        }),
        ("FoldLeft on an operation that is not closed", {
            let mut c = contract("TEST:F");
            c.arity = Arity::Fixed { inn: 2, out: 1 };
            c.stack_effect = "( a b -- c )";
            c.input_types = &[TypeSpec::Number, TypeSpec::Number];
            c.output_types = &[TypeSpec::TruthValue];
            c.stak = StakSupport::FoldLeft;
            c
        }),
    ];
    for (why, malformed) in cases {
        let mut interpreter = Interpreter::new();
        let name = malformed.name;
        let package = Package::new("test").with(malformed, Body::Op(double));
        let outcome = interpreter.register_package(package);
        assert!(
            matches!(
                outcome,
                Err(Error::MalformedContract { .. }) | Err(Error::DuplicateWord { .. })
            ),
            "{why}: should have been refused, got {outcome:?}"
        );
        // Nothing the package supplied is in the vocabulary. A name Ajisai
        // Core already owns is of course still there — as Ajisai Core's.
        match interpreter.word(name) {
            None => {}
            Some(word) => assert_eq!(word.package, "ajisai-core", "{why}: word survived"),
        }
    }
}

/// A well-formed package registers, and its words get the whole Ajisai Core
/// machinery without opting in.
#[test]
fn a_well_formed_package_registers_whole() {
    let mut interpreter = Interpreter::new();
    let package = Package::new("test")
        .with(contract("TEST:ONE"), Body::Op(double))
        .with(contract("TEST:TWO"), Body::Op(double));
    interpreter.register_package(package).expect("registers");
    interpreter.execute("21 TEST:ONE").expect("runs");
    assert_eq!(ajisai_core::render_stack(&interpreter), vec!["42"]);
    // The declared STAK reading applies, through the same operand layer.
    let mut interpreter = Interpreter::new();
    let package = Package::new("test").with(contract("TEST:ONE"), Body::Op(double));
    interpreter.register_package(package).unwrap();
    interpreter.execute("1 2 3 STAK TEST:ONE").expect("maps");
    assert_eq!(ajisai_core::render_stack(&interpreter), vec!["2", "4", "6"]);
}
