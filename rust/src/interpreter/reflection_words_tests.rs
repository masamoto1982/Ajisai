//! Behavioral probes for the reflection Words `DIGEST` and
//! `CONTRACT` — over a Symbol and over a block
//! (LANG.DICTIONARY.RESOLUTION, LANG.DICTIONARY.MUTATION,
//! LANG.CONTRACT.REGISTRY, LANG.CONTRACT.CHECK).

#[cfg(test)]
mod reflection_words_tests {
    use crate::interpreter::Interpreter;

    async fn run(code: &str) -> Interpreter {
        let mut interp = Interpreter::new();
        interp
            .execute(code)
            .await
            .unwrap_or_else(|e| panic!("`{code}` must not error: {e}"));
        interp
    }

    async fn top(code: &str) -> String {
        run(code)
            .await
            .get_stack()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    }

    async fn error_of(code: &str) -> String {
        let mut interp = Interpreter::new();
        let err = interp.execute(code).await.expect_err("must raise an ERROR");
        crate::error::ErrorCategory::from_error(&err)
            .as_protocol_str()
            .to_string()
    }

    #[tokio::test]
    async fn a_string_is_not_a_name() {
        assert_eq!(error_of("'ADD' CONTRACT").await, "notASymbol");
        assert_eq!(error_of("NIL CONTRACT").await, "notASymbol");
        assert_eq!(error_of("5 CONTRACT").await, "notASymbol");
        // The operand is back on the stack after the ERROR.
        let mut interp = Interpreter::new();
        let _ = interp.execute("'ADD' CONTRACT").await;
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn digest_is_a_word_identity_or_a_denotation_digest() {
        let core = top("[ ADD ] 0 GET DIGEST").await;
        assert_eq!(core.len(), 2 + 1 + 64, "a #-prefixed 64-hex digest, quoted");
        assert_eq!(
            top("[ ADD ] 0 GET DIGEST [ + ] 0 GET DIGEST EQ").await,
            "TRUE"
        );
        assert_eq!(
            top("[ ADD ] 0 GET DIGEST [ SUB ] 0 GET DIGEST EQ").await,
            "FALSE"
        );
        // A User Word's digest is the dictionary's own content identity.
        let interp = run("[ 2 MUL ] 'TWICE' DEF [ TWICE ] 0 GET DIGEST").await;
        let answer = interp.stack.last().unwrap().as_text().unwrap().to_string();
        assert_eq!(Some(&answer), interp.word_identity("TWICE"));
        // Content, not spelling: the same body under two names is one Word.
        assert_eq!(
            top("[ 2 MUL ] 'TWICE' DEF [ 2 MUL ] 'DOUBLE' DEF [ TWICE ] 0 GET DIGEST [ DOUBLE ] 0 GET DIGEST EQ")
                .await,
            "TRUE"
        );
        // A value digests by its denotation (LANG.VALUES.DENOTATION).
        assert_eq!(
            top("8 SQRT DIGEST 2 SQRT 2 SQRT ADD DIGEST EQ").await,
            "TRUE"
        );
        assert_eq!(top("[ 1 2 ] DIGEST [ 1 2 ] DIGEST EQ").await, "TRUE");
        assert_eq!(top("[ 1 2 ] DIGEST [ 2 1 ] DIGEST EQ").await, "FALSE");
        assert_eq!(top("1 0 DIV DIGEST 1 0 DIV DIGEST EQ").await, "TRUE");
        assert_eq!(top("1 0 DIV DIGEST NIL DIGEST EQ").await, "FALSE");
        // A Symbol naming nothing is still a value.
        assert_eq!(
            top("[ TWICE ] 0 GET DIGEST [ TWICE ] 0 GET DIGEST EQ").await,
            "TRUE"
        );
        // Every value has a finite canonical form to digest, an irrational's
        // inside a Vector included.
        assert_eq!(
            top("1 2 SQRT 2 COLLECT DIGEST 1 8 SQRT 2 DIV 2 COLLECT DIGEST EQ").await,
            "TRUE"
        );
    }

    #[tokio::test]
    async fn contract_answers_the_registered_record_of_a_core_word() {
        assert_eq!(top("[ DIV ] 0 GET CONTRACT 'name' AT").await, "'DIV'");
        assert_eq!(top("[ DIV ] 0 GET CONTRACT 'inputs' AT").await, "2/1");
        assert_eq!(top("[ DIV ] 0 GET CONTRACT 'outputs' AT").await, "1/1");
        assert_eq!(
            top("[ DIV ] 0 GET CONTRACT 'projection' AT").await,
            "[ 'divisionByZero' ]"
        );
        assert_eq!(
            top("[ DIV ] 0 GET CONTRACT 'errors' AT").await,
            "[ 'nonNumeric' 'shapeMismatch' ]"
        );
        assert_eq!(
            top("[ DIV ] 0 GET CONTRACT 'cost' AT KEYS").await,
            "[ 'steps' 'numeric' 'collection' ]"
        );
        assert_eq!(
            top("[ MAP ] 0 GET CONTRACT 'cost' AT 'steps' AT").await,
            "'unbounded'"
        );
        assert_eq!(
            top("[ PRINT ] 0 GET CONTRACT 'effects' AT").await,
            "[ 'consoleWrite' ]"
        );
        assert_eq!(top("[ MAP ] 0 GET CONTRACT 'inputs' AT").await, "2/1");
        assert_eq!(
            top("[ SORT ] 0 GET CONTRACT KEYS").await,
            "[ 'name' 'tier' 'inputs' 'outputs' 'consumption' 'nil' 'projection' 'errors' 'partiality' 'purity' 'determinism' 'cost' 'effects' ]"
        );
        assert_eq!(top("[ SORT ] 0 GET CONTRACT 'tier' AT").await, "'standard'");
    }

    #[tokio::test]
    async fn contract_infers_a_user_word_and_a_block_in_one_shape() {
        let code = "[ 42 PRINT ] 'SHOUT' DEF [ SHOUT ] 0 GET CONTRACT [ 42 PRINT ] CONTRACT";
        let interp = run(code).await;
        let stack = interp.get_stack();
        assert_eq!(stack.len(), 2);
        assert_eq!(
            stack[0], stack[1],
            "CONTRACT of a User Word is CONTRACT of its body"
        );
        assert_eq!(
            top("[ 42 PRINT ] CONTRACT KEYS").await,
            "[ 'inputs' 'outputs' 'nil' 'purity' 'determinism' 'cost' 'effects' 'confidence' 'gaps' ]"
        );
        assert_eq!(top("[ 1 2 ADD ] CONTRACT 'purity' AT").await, "'pure'");
        assert_eq!(
            top("[ 1 2 ADD ] CONTRACT 'confidence' AT").await,
            "'complete'"
        );
        assert_eq!(top("[ 1 2 ADD ] CONTRACT 'inputs' AT").await, "0/1");
        assert_eq!(top("[ ADD ] CONTRACT 'inputs' AT").await, "2/1");
        assert_eq!(top("[ ] CONTRACT 'purity' AT").await, "'pure'");
        assert_eq!(
            top("[ 42 PRINT ] CONTRACT 'effects' AT").await,
            "[ 'consoleWrite' ]"
        );
        assert_eq!(
            top("[ 42 PRINT ] CONTRACT 'purity' AT").await,
            "'effectful'"
        );
        assert_eq!(
            top("[ NOPE ] CONTRACT 'confidence' AT [ NOPE ] CONTRACT 'gaps' AT").await,
            "'conservative' [ 'gap.unresolvedWord' ]"
        );
        // Nothing ran: the effect was reported, not performed, for the
        // named body and for the bare block alike.
        let interp =
            run("[ 42 PRINT ] 'SHOUT' DEF [ SHOUT ] 0 GET CONTRACT [ 42 PRINT ] CONTRACT").await;
        assert!(interp.host_effects().is_empty());
        // Inferring never mutates the dictionary.
        let before = run("[ 2 MUL ] 'TWICE' DEF").await;
        let after = run("[ 2 MUL ] 'TWICE' DEF [ TWICE 1 ADD ] CONTRACT").await;
        assert_eq!(before.dictionary_epoch, after.dictionary_epoch);
        assert_eq!(before.user_words.len(), after.user_words.len());
    }

    #[tokio::test]
    async fn contract_reads_the_block_and_restores_a_bad_operand() {
        assert_eq!(
            top("[ 1 ] 'B' BIND B B CONTRACT 'inputs' AT").await,
            "[ 1/1 ] 0/1"
        );
        for source in ["1 CONTRACT", "NIL CONTRACT"] {
            let mut interp = Interpreter::new();
            assert!(interp.execute(source).await.is_err(), "accepted {source}");
            assert_eq!(interp.stack.len(), 1, "operand was not restored: {source}");
        }
    }

    #[tokio::test]
    async fn contract_projects_missing_field_for_an_unknown_name() {
        assert_eq!(
            top("[ NOPE ] 0 GET CONTRACT NIL-REASON").await,
            "NIL 'missingField'"
        );
        assert_eq!(
            top("7 'N' BIND [ N ] 0 GET CONTRACT NIL?").await,
            "NIL TRUE"
        );
    }
}
