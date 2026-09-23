//! Restoring a saved dictionary.
//!
//! A saved definition is source text, so restoring it re-runs the lexer and
//! `DEF` against today's rules — and those rules are not frozen. One entry this
//! build no longer accepts used to abort the whole restore, leaving the session
//! holding whichever words happened to precede it; these tests hold the
//! opposite contract, the one the host already states for a partially corrupt
//! import: everything restorable is restored, and what was not comes back named.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    fn word(name: &str, definition: &str) -> (String, String, Option<String>) {
        (name.to_string(), definition.to_string(), None)
    }

    #[tokio::test]
    async fn an_unreadable_entry_does_not_take_the_readable_ones_with_it() {
        let mut interp = Interpreter::new();
        let skipped = interp
            .restore_user_word_definitions([
                word("FIRST", "[ 1 ]"),
                // No longer lexes: a bracket must stand alone (LANG.SOURCE.TEXT).
                word("LEGACY", "[1]"),
                word("LAST", "[ 3 ]"),
            ])
            .expect("a skippable entry is not a restore failure");

        assert_eq!(skipped.len(), 1, "exactly one entry was unreadable");
        assert_eq!(skipped[0].name, "LEGACY");
        assert!(
            skipped[0].reason.contains("must stand alone"),
            "the skip should carry the lexer's reason, got: {}",
            skipped[0].reason
        );

        // The point of the exercise: the words either side of it survived.
        assert!(interp.user_words.contains_key("FIRST"));
        assert!(interp.user_words.contains_key("LAST"));
        assert!(!interp.user_words.contains_key("LEGACY"));

        // And they are callable, not just present.
        interp.execute("FIRST").await.expect("FIRST should run");
        assert_eq!(interp.stack.len(), 1);
    }

    /// The same holds when the entry is refused by `DEF` rather than by the
    /// lexer — a name saved before the unwritable-name rule, say. The two
    /// changes meet here: skipping is what makes tightening `DEF` safe for a
    /// dictionary saved under the older rule.
    #[tokio::test]
    async fn an_entry_def_refuses_is_skipped_too() {
        let mut interp = Interpreter::new();
        let skipped = interp
            .restore_user_word_definitions([
                word("KEPT", "[ 1 ]"),
                word("A[B", "[ 2 ]"),
                word("ALSO-KEPT", "[ 3 ]"),
            ])
            .expect("a refused name is not a restore failure");

        assert_eq!(
            skipped.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            vec!["A[B"]
        );
        assert!(interp.user_words.contains_key("KEPT"));
        assert!(interp.user_words.contains_key("ALSO-KEPT"));
    }

    #[tokio::test]
    async fn a_clean_dictionary_restores_whole_and_reports_nothing() {
        let mut interp = Interpreter::new();
        let skipped = interp
            .restore_user_word_definitions([word("ONE", "[ 1 ]"), word("TWO", "[ 2 ]")])
            .expect("nothing here is unreadable");

        assert!(skipped.is_empty(), "nothing should be reported skipped");
        assert!(interp.user_words.contains_key("ONE"));
        assert!(interp.user_words.contains_key("TWO"));
    }

    /// An entry with no saved body is not a failure to report — there is
    /// nothing to restore and nothing went wrong. The host relies on this:
    /// `restore_user_words` skips a definition-less word.
    #[tokio::test]
    async fn a_definition_less_entry_is_passed_over_silently() {
        let mut interp = Interpreter::new();
        let skipped = interp
            .restore_user_word_definitions([word("EMPTY", ""), word("REAL", "[ 1 ]")])
            .expect("an empty definition is not a failure");

        assert!(skipped.is_empty(), "an absent body is not a skip to report");
        assert!(!interp.user_words.contains_key("EMPTY"));
        assert!(interp.user_words.contains_key("REAL"));
    }

    /// A word whose body calls one that was skipped still restores: the
    /// reference simply does not resolve, exactly as a forward reference does
    /// not, and it fails at call time rather than at restore time.
    #[tokio::test]
    async fn a_dependent_of_a_skipped_word_still_restores() {
        let mut interp = Interpreter::new();
        let skipped = interp
            .restore_user_word_definitions([word("MISSING", "[1]"), word("CALLER", "[ MISSING ]")])
            .expect("the dependency rebuild must survive an unresolved reference");

        assert_eq!(skipped.len(), 1);
        assert!(interp.user_words.contains_key("CALLER"));
    }
}
