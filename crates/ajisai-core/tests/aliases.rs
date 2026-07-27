//! Symbol notation is a first-class surface, and it is one table.

mod support;
use support::line;

use ajisai_core::alias::{aliases_for, ALIASES};
use ajisai_core::{lint, Interpreter};

/// Every registered alias names a word that actually exists.
#[test]
fn every_alias_points_at_a_real_word() {
    let interpreter = Interpreter::new();
    for (symbol, canonical) in ALIASES {
        assert!(
            interpreter.word(canonical).is_some(),
            "alias {symbol} points at {canonical}, which is not a word"
        );
    }
}

/// The table is the only table: nothing maps two symbols to different words
/// under the same name, and no symbol is listed twice.
#[test]
fn the_table_is_unambiguous() {
    let mut symbols: Vec<&str> = ALIASES.iter().map(|(symbol, _)| *symbol).collect();
    let count = symbols.len();
    symbols.sort_unstable();
    symbols.dedup();
    assert_eq!(symbols.len(), count, "a symbol is listed more than once");
}

/// Every alias produces the same flow as its canonical word — checked over the
/// whole table rather than for a hand-picked few.
#[test]
fn every_alias_is_semantically_identical_to_its_word() {
    // A program fragment that exercises each word, keyed by canonical name.
    let exercise = |word: &str| -> Option<String> {
        Some(match word {
            "ADD" => "7 5 {}".to_string(),
            "SUB" => "7 5 {}".to_string(),
            "MUL" => "7 5 {}".to_string(),
            "DIV" => "7 5 {}".to_string(),
            "EQ" => "7 5 {}".to_string(),
            "LT" => "7 5 {}".to_string(),
            "GT" => "7 5 {}".to_string(),
            "TOP" => "7 5 3 {} ADD".to_string(),
            "STAK" => "7 5 3 {} ADD".to_string(),
            "EAT" => "7 5 3 {} ADD".to_string(),
            "KEEP" => "7 5 3 {} ADD".to_string(),
            "VENT" => "7 TRUE {} { 5 ADD }".to_string(),
            _ => return None,
        })
    };
    let mut covered = 0;
    for (symbol, canonical) in ALIASES {
        let template = exercise(canonical)
            .unwrap_or_else(|| panic!("no equivalence exercise for alias {symbol} -> {canonical}"));
        assert_eq!(
            line(&template.replace("{}", canonical)),
            line(&template.replace("{}", symbol)),
            "{symbol} and {canonical} must produce the same flow"
        );
        covered += 1;
    }
    assert_eq!(covered, ALIASES.len(), "every alias must be exercised");
}

/// Errors are identical too — the alias is normalized before any layer that
/// could report a different name sees it.
#[test]
fn errors_name_the_canonical_word_whichever_form_was_written() {
    let mut interpreter = Interpreter::new();
    let canonical = interpreter.execute("[ 1 ] 2 ADD").unwrap_err().to_string();
    let mut interpreter = Interpreter::new();
    let symbolic = interpreter.execute("[ 1 ] 2 +").unwrap_err().to_string();
    assert_eq!(canonical, symbolic);
    assert!(canonical.contains("ADD"), "{canonical}");
}

/// The formatter normalizes to canonical words, so symbol and word forms have
/// one printed representation.
#[test]
fn the_formatter_normalizes_to_canonical_words() {
    let program = ajisai_core::syntax::parse("1 2 & + : ^ { 3 }").expect("parses");
    assert_eq!(
        ajisai_core::syntax::render_program(&program),
        "1 2 KEEP ADD STAK VENT { 3 }"
    );
}

/// The lint sees the same program either way.
#[test]
fn the_lint_sees_one_program() {
    let interpreter = Interpreter::new();
    let canonical = lint::lint(&interpreter, "[ 1 ] 2 ADD").unwrap();
    let symbolic = lint::lint(&interpreter, "[ 1 ] 2 +").unwrap();
    assert_eq!(canonical, symbolic);
    assert!(!canonical.is_empty(), "the mismatch should be reported");
}

/// Aliases are not a second way to name a word in the dictionary either.
#[test]
fn an_alias_cannot_be_redefined() {
    let mut interpreter = Interpreter::new();
    let error = interpreter.execute("{ 1 } \"+\" DEF").unwrap_err();
    assert!(error.to_string().contains("ADD"), "{error}");
}

#[test]
fn aliases_for_reports_the_table_consistently() {
    assert_eq!(aliases_for("ADD"), vec!["+"]);
    assert_eq!(aliases_for("VENT"), vec!["^"]);
    assert!(aliases_for("LENGTH").is_empty());
}
