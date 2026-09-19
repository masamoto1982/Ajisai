//! Behavioral probes for `ABSENT` and `FAIL`: the reason a program states is
//! the reason the value carries, observably and as part of its identity.

#[cfg(test)]
mod declared_outcomes_tests {
    use crate::interpreter::Interpreter;
    use crate::types::Value;

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

    #[tokio::test]
    async fn absent_carries_the_reason_the_program_states() {
        assert_eq!(
            top("'rate not quoted' ABSENT NIL-REASON").await,
            "NIL 'rate not quoted'"
        );
        assert_eq!(top("'why' ABSENT NIL?").await, "NIL TRUE");
        assert_eq!(top("0 'why' ABSENT NIL? SELECT").await, "0/1");
        let interp = run("'why' ABSENT").await;
        let value = interp.stack.last().cloned().expect("an answer");
        assert_eq!(
            value.nil_reason().map(|r| r.as_protocol_str()),
            Some("userDeclared")
        );
        assert_eq!(value.absence_detail(), Some("why"));
    }

    /// LANG.VALUES.NIL: the reason is the value's entire observable content,
    /// and the text is the reason, so it decides identity.
    #[tokio::test]
    async fn the_text_is_part_of_the_value() {
        assert_eq!(
            top("'a' ABSENT 'b' ABSENT 2 COLLECT UNIQUE LENGTH").await,
            "2/1"
        );
        assert_eq!(
            top("'a' ABSENT 'a' ABSENT 2 COLLECT UNIQUE LENGTH").await,
            "1/1"
        );
        assert_ne!(Value::nil_user_declared("a"), Value::nil_user_declared("b"));
        assert_eq!(Value::nil_user_declared("a"), Value::nil_user_declared("a"));
    }

    /// The detail survives the two places a value can be stored other than
    /// the stack: a dense lane, and the persistence codec.
    #[tokio::test]
    async fn the_detail_survives_a_dense_lane() {
        assert_eq!(
            top("1 'why' ABSENT 2 COLLECT [ 1 ] GET NIL-REASON").await,
            "NIL 'why'"
        );
    }

    #[tokio::test]
    async fn fail_raises_the_declared_category_with_the_message() {
        let mut interp = Interpreter::new();
        let error = interp
            .execute("1 'width must be positive' FAIL 2")
            .await
            .expect_err("FAIL must raise");
        let text = error.to_string();
        assert!(text.contains("width must be positive"), "got: {text}");
        // The operand is restored, and nothing after FAIL ran.
        assert_eq!(
            interp
                .get_stack()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            ["1/1", "'width must be positive'"]
        );
    }

    #[tokio::test]
    async fn a_non_text_operand_is_the_program_being_wrong() {
        for code in ["1 ABSENT", "1 FAIL", "NIL ABSENT"] {
            let mut interp = Interpreter::new();
            let text = interp
                .execute(code)
                .await
                .expect_err(&format!("`{code}` must raise"))
                .to_string();
            assert!(
                text.contains("String"),
                "`{code}` must name the String it expected, got: {text}"
            );
        }
    }
}
