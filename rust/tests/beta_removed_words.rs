use ajisai_core::interpreter::Interpreter;
use ajisai_core::AjisaiError;

/// The alpha Words the beta vocabulary freeze retired.
///
/// `UNIQUE` is not among them any more. It was cut as one of several
/// overlapping collection Words; it has since come back on its own terms —
/// a contract in `spec/words.json`, a law witness, a conformance case — so
/// asserting it is unknown would assert the opposite of what the language now
/// says. A retired name returning is a vocabulary decision, and this list
/// records the decision rather than freezing the first one made.
const REMOVED_WORDS: &[&str] = &[
    "CEIL",
    "SIGN",
    "INSERT",
    "REPLACE",
    "REMOVE",
    "SPLIT",
    "REORDER",
    "CONTAINS",
    "STARTS-WITH?",
    "ENDS-WITH?",
    "CHR",
    "EAT",
];

#[tokio::test]
async fn removed_beta_words_are_unknown_at_runtime() {
    for word in REMOVED_WORDS {
        let mut interpreter = Interpreter::new();
        let error = match interpreter.execute(word).await {
            Ok(()) => panic!("removed Word {word} unexpectedly executed"),
            Err(error) => error,
        };
        assert!(
            matches!(error, AjisaiError::UnknownWord(ref name) if name == word),
            "{word} resolved through a stale runtime entry: {error}"
        );
    }
}

/// A removed name reached through `REFLECT`'s code-data decoder is still an
/// Unknown Word: decoding builds a CodeBlock, it does not resolve names, so the
/// deleted entry cannot reappear behind the reflection boundary.
#[tokio::test]
async fn removed_beta_words_are_unknown_through_reflect_decode() {
    for word in REMOVED_WORDS {
        let mut interpreter = Interpreter::new();
        let source = format!("[ 'AJISAI-CODE-1' [ 'symbol' '{word}' ] ] REFLECT EXEC");
        let error = match interpreter.execute(&source).await {
            Ok(()) => panic!("removed Word {word} executed after REFLECT decode"),
            Err(error) => error,
        };
        assert!(
            matches!(error, AjisaiError::UnknownWord(ref name) if name == word),
            "{word} resolved through the REFLECT decoder: {error}"
        );
    }
}

/// The compiled path a User Word body takes is the same inventory: defining a
/// body over a removed name is allowed (the body is data until called), and
/// calling it fails as an Unknown Word rather than dispatching a stale arm.
#[tokio::test]
async fn removed_beta_words_are_unknown_in_a_compiled_user_word() {
    for word in REMOVED_WORDS {
        let mut interpreter = Interpreter::new();
        let source = format!("{{ 1 {word} }} 'PROBE' DEF PROBE");
        let error = match interpreter.execute(&source).await {
            Ok(()) => panic!("removed Word {word} executed from a compiled body"),
            Err(error) => error,
        };
        assert!(
            matches!(error, AjisaiError::UnknownWord(ref name) if name == word),
            "{word} resolved through the compiled plan: {error}"
        );
    }
}

/// `LOOKUP` is the dictionary's own reading surface: it must not describe a
/// Word the inventory no longer has.
#[tokio::test]
async fn removed_beta_words_are_unknown_to_lookup() {
    for word in REMOVED_WORDS {
        let mut interpreter = Interpreter::new();
        let error = match interpreter.execute(&format!("'{word}' LOOKUP")).await {
            Ok(()) => panic!("LOOKUP described removed Word {word}"),
            Err(error) => error,
        };
        assert!(
            matches!(error, AjisaiError::UnknownWord(ref name) if name == word),
            "{word} is still documented by LOOKUP: {error}"
        );
    }
}

#[tokio::test]
async fn removed_eat_alias_is_not_a_no_op() {
    let mut interpreter = Interpreter::new();
    assert!(matches!(
        interpreter.execute(",").await,
        Err(AjisaiError::UnknownWord(name)) if name == ","
    ));
}
