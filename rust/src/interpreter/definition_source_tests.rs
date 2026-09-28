//! A definition is kept as its source (LANG.DICTIONARY.MUTATION).
//!
//! A body built from a computed Vector can carry a value no source text
//! denotes. `DEF` writes it back as the source that builds it
//! (`value_as_code::value_elements_to_source_tokens`), so the body the
//! dictionary holds, shows and saves restores to the same Word, with the same
//! identity, in every session.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    fn definition_text(interp: &Interpreter, name: &str) -> String {
        interp
            .lookup_word_definition_tokens(name)
            .expect("a User Word has a definition")
    }

    async fn restored_from_text(name: &str, text: &str) -> Interpreter {
        let mut fresh = Interpreter::new();
        let skipped = fresh
            .restore_user_word_definitions([(name.to_string(), text.to_string(), None)])
            .expect("a restore of source text");
        assert!(skipped.is_empty(), "{skipped:?}");
        fresh
    }

    #[tokio::test]
    async fn a_record_carried_whole_is_kept_as_the_source_that_builds_it() {
        let mut interp = Interpreter::new();
        interp
            .execute("[ 'k' ] [ 5 ] RECORD 1 COLLECT [ 'k' GET ] CONCAT 'W' DEF W")
            .await
            .unwrap();
        assert_eq!(
            format!("{}", interp.get_stack().last().expect("a result")),
            "5/1"
        );
        let text = definition_text(&interp, "W");
        assert_eq!(text, "[ 'k' ] [ 5 ] RECORD 'k' GET");

        let mut fresh = restored_from_text("W", &text).await;
        fresh.execute("W").await.expect("the restored Word runs");
        assert_eq!(
            format!("{}", fresh.get_stack().last().expect("a result")),
            "5/1"
        );
        assert_eq!(
            interp.word_identity("W"),
            fresh.word_identity("W"),
            "the same source is the same Word in every session"
        );
    }

    #[tokio::test]
    async fn an_irrational_carried_whole_is_kept_as_its_normal_form() {
        let mut interp = Interpreter::new();
        interp
            .execute("2 SQRT 3 ADD 1 2 DIV 3 SQRT MUL ADD 1 COLLECT 'S' DEF S")
            .await
            .unwrap();
        let shown = format!("{}", interp.get_stack().last().expect("a result"));
        let text = definition_text(&interp, "S");
        assert!(
            text.starts_with("0 ") && text.ends_with(" ADD") && text.contains("SQRT"),
            "{text}"
        );

        let mut fresh = restored_from_text("S", &text).await;
        fresh.execute("S").await.unwrap();
        assert_eq!(
            format!("{}", fresh.get_stack().last().expect("a result")),
            shown
        );
    }

    #[tokio::test]
    async fn a_symbol_beside_a_record_stays_data() {
        let mut interp = Interpreter::new();
        interp
            .execute("[ 'k' ] [ 1 ] RECORD [ V ] 0 GET 2 COLLECT 1 COLLECT 'M' DEF M")
            .await
            .unwrap();
        assert_eq!(
            format!("{}", interp.get_stack().last().expect("a result")),
            "[ { 'k' 1/1 } V ]"
        );
        let text = definition_text(&interp, "M");
        assert_eq!(text, "[ 'k' ] [ 1 ] RECORD [ V ] 0 GET 2 COLLECT");
        let mut fresh = restored_from_text("M", &text).await;
        fresh.execute("M").await.unwrap();
        assert_eq!(
            format!("{}", fresh.get_stack().last().expect("a result")),
            "[ { 'k' 1/1 } V ]"
        );
    }

    #[tokio::test]
    async fn a_nested_record_rebuilds_inside_out() {
        let mut interp = Interpreter::new();
        interp
            .execute("[ 'k' ] [ 'a' ] [ 1 ] RECORD 1 COLLECT RECORD 1 COLLECT 'N' DEF")
            .await
            .unwrap();
        assert_eq!(
            definition_text(&interp, "N"),
            "[ 'k' ] [ 'a' ] [ 1 ] RECORD 1 COLLECT RECORD"
        );
    }

    /// The `NIL` name denotes the literal NIL and no other, so a body holding
    /// a NIL that carries a reason has no source and is refused; a literal
    /// NIL is written as its name.
    #[tokio::test]
    async fn a_reasoned_nil_has_no_source_and_is_refused() {
        let mut interp = Interpreter::new();
        let error = interp
            .execute("1 0 DIV 1 COLLECT 'Z' DEF")
            .await
            .expect_err("no source denotes a reasoned NIL");
        assert!(
            matches!(
                crate::error::ErrorCategory::from_error(&error),
                Some(crate::error::ErrorCategory::Declared(
                    "invalidDefinitionBody"
                ))
            ),
            "{error}"
        );
        assert!(error.to_string().contains("divisionByZero"), "{error}");
        assert!(!interp.user_words.contains_key("Z"));

        interp.execute("NIL 1 COLLECT 'L' DEF").await.unwrap();
        assert_eq!(definition_text(&interp, "L"), "NIL");
    }

    /// A body with no carried value is kept as it is.
    #[tokio::test]
    async fn a_literal_body_is_kept_as_written() {
        let mut interp = Interpreter::new();
        interp
            .execute("[ 1 [ 2 ] 'x' TRUE NIL ] 1 COLLECT 'U' DEF")
            .await
            .unwrap();
        assert_eq!(definition_text(&interp, "U"), "[ 1 [ 2 ] 'x' TRUE NIL ]");
    }
}
