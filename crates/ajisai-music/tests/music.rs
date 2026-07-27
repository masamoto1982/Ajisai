//! `ajisai-music` on its own, and the boundary it sits behind.

use ajisai_core::Interpreter;

fn ajisai() -> Interpreter {
    let mut interpreter = Interpreter::new();
    interpreter
        .register_package(ajisai_music::package())
        .expect("the package registers");
    interpreter
}

fn line(interpreter: &mut Interpreter, source: &str) -> String {
    interpreter
        .execute(source)
        .unwrap_or_else(|error| panic!("`{source}` failed: {error}"));
    ajisai_core::render_stack(interpreter).join(" ")
}

/// Ajisai Core, on its own, has never heard of any of this.
#[test]
fn core_alone_does_not_know_the_package() {
    let mut plain = Interpreter::new();
    for word in ["MUSIC:JUST", "MUSIC:NOTE", "MUSIC:PITCH", "MUSIC:TRANSPOSE"] {
        assert!(plain.word(word).is_none(), "{word} leaked into Core");
        assert!(plain.execute(word).is_err(), "{word} evaluates in Core");
    }
}

/// The package adds words and nothing else: no value shape, no role, no mode.
#[test]
fn the_package_adds_only_words() {
    let plain = Interpreter::new();
    let extended = ajisai();
    assert_eq!(
        extended.vocabulary().len(),
        plain.vocabulary().len() + 7,
        "the package should add exactly its seven words"
    );
    for word in extended.contracts() {
        assert!(
            word.package == "ajisai-core" || word.package == "ajisai-music",
            "{} has an unexpected owner",
            word.contract.name
        );
    }
    // Every Ajisai Core word still means what it meant.
    let mut extended = ajisai();
    assert_eq!(line(&mut extended, "1 3 DIV 3 MUL"), "1");
}

/// A package may not take a name Ajisai Core owns, and registering the same
/// package twice fails rather than shadowing.
#[test]
fn registration_refuses_to_shadow() {
    let mut interpreter = ajisai();
    assert!(interpreter
        .register_package(ajisai_music::package())
        .is_err());
}

/// Just intonation, exactly. A perfect fifth is 3/2 and it stays 3/2 however
/// far the chain runs.
#[test]
fn intervals_are_exact() {
    let mut interpreter = ajisai();
    assert_eq!(line(&mut interpreter, "440 3 2 MUSIC:JUST"), "660");

    // Twelve perfect fifths do not close the circle, and an exact language can
    // say by exactly how much: the Pythagorean comma, 531441/524288.
    let mut interpreter = ajisai();
    let mut source = String::from("1");
    for _ in 0..12 {
        source.push_str(" 3 2 MUSIC:JUST");
    }
    // Divide down by the seven octaves that chain spans.
    source.push_str(" 128 DIV");
    assert_eq!(
        line(&mut interpreter, &source),
        "531441/524288",
        "the comma is exact, not a rounding artefact"
    );
}

#[test]
fn notes_carry_pitch_and_duration() {
    let mut interpreter = ajisai();
    assert_eq!(line(&mut interpreter, "440 1 MUSIC:NOTE"), "[ 440 1 ]");
    let mut interpreter = ajisai();
    assert_eq!(
        line(&mut interpreter, "440 1 MUSIC:NOTE 3 2 MUSIC:TRANSPOSE"),
        "[ 660 1 ]"
    );
    let mut interpreter = ajisai();
    assert_eq!(
        line(&mut interpreter, "440 1 MUSIC:NOTE MUSIC:PITCH"),
        "440"
    );
    let mut interpreter = ajisai();
    assert_eq!(line(&mut interpreter, "1 2 DIV MUSIC:REST"), "[ 0 1/2 ]");
    // A dotted quarter at 90bpm is exactly one second.
    let mut interpreter = ajisai();
    assert_eq!(
        line(&mut interpreter, "440 3 2 DIV MUSIC:NOTE 90 MUSIC:SECONDS"),
        "1"
    );
}

/// Package words are ordinary words, so the Ajisai Core machinery applies to
/// them without the package doing anything to opt in.
#[test]
fn package_words_get_the_core_machinery_for_free() {
    // Vector words compose with them.
    let mut interpreter = ajisai();
    assert_eq!(
        line(&mut interpreter, "[ 1 2 3 ] { 440 SWAP 1 MUSIC:JUST } MAP"),
        "[ 440 880 1320 ]"
    );
    // The flow modes apply, through the same operand layer.
    let mut interpreter = ajisai();
    assert_eq!(
        line(&mut interpreter, "[ 440 1 ] KEEP MUSIC:PITCH"),
        "[ 440 1 ] 440"
    );
    let mut interpreter = ajisai();
    assert_eq!(
        line(&mut interpreter, "[ 440 1 ] [ 660 2 ] STAK MUSIC:BEATS"),
        "1 2"
    );
    // And so does the vent.
    let mut interpreter = ajisai();
    assert_eq!(
        line(&mut interpreter, "FALSE VENT { 440 0 0 MUSIC:JUST } 7"),
        "7"
    );
}

/// The contract lint reads package contracts through the same registry.
#[test]
fn the_lint_reads_package_contracts() {
    let interpreter = ajisai();
    let findings = ajisai_core::lint::lint(&interpreter, "440 MUSIC:JUST").expect("parses");
    assert!(
        findings.iter().any(|f| f.message.contains("needs 3")),
        "{findings:?}"
    );
    let findings = ajisai_core::lint::lint(&interpreter, "440 3 2 MUSIC:JUST").expect("parses");
    assert!(findings.is_empty(), "{findings:?}");
}

/// Music has no reading for an absent or undetermined pitch, and the contract
/// says so rather than quietly propagating one.
#[test]
fn absence_is_refused_rather_than_propagated() {
    let mut interpreter = ajisai();
    assert!(interpreter.execute("NIL 3 2 MUSIC:JUST").is_err());
    let mut interpreter = ajisai();
    assert!(interpreter.execute("440 UNKNOWN 2 MUSIC:JUST").is_err());
    let mut interpreter = ajisai();
    assert!(interpreter.execute("440 3 0 MUSIC:JUST").is_err());
}
