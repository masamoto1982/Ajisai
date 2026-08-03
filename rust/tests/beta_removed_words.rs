use ajisai_core::interpreter::Interpreter;
use ajisai_core::AjisaiError;

const REMOVED_WORDS: &[&str] = &[
    "CEIL",
    "SIGN",
    "INSERT",
    "REPLACE",
    "REMOVE",
    "SPLIT",
    "REORDER",
    "UNIQUE",
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

#[tokio::test]
async fn removed_eat_alias_is_not_a_no_op() {
    let mut interpreter = Interpreter::new();
    assert!(matches!(
        interpreter.execute(",").await,
        Err(AjisaiError::UnknownWord(name)) if name == ","
    ));
}
