//! Parameter headers: `[ A B | body ] 'NAME' DEF` (LANG.SOURCE.FRAME).
//!
//! Every User Word states its arity in a header. The call takes exactly that
//! many operands, binds them, and runs the body on an empty stack, so what
//! `KEEP` keeps is read from the definition, and the Word means the same as
//! its body with its parameters bound by `BIND` — an expansion into Core
//! Words alone (`docs/dev/word-arity-header-work-order-2026-09.md`).

#[cfg(test)]
mod tests {
    use crate::interpreter::word_contract::ContractFlow;
    use crate::interpreter::Interpreter;

    async fn stack_of(src: &str) -> Vec<String> {
        let mut interp = Interpreter::new();
        interp
            .execute(src)
            .await
            .unwrap_or_else(|e| panic!("`{src}` failed: {e}"));
        interp
            .stack
            .iter_slots()
            .map(|(v, _)| v.to_string())
            .collect()
    }

    async fn error_of(src: &str) -> String {
        let mut interp = Interpreter::new();
        interp
            .execute(src)
            .await
            .expect_err(&format!("`{src}` should fail"))
            .to_string()
    }

    const DIFF: &str = "[ A B | A B - ] 'DIFF' DEF ";

    #[tokio::test]
    async fn parameters_bind_deepest_operand_first() {
        assert_eq!(stack_of(&format!("{DIFF} 10 3 DIFF")).await, ["7/1"]);
    }

    #[tokio::test]
    async fn the_call_takes_exactly_the_declared_operands() {
        assert_eq!(
            stack_of(&format!("{DIFF} 1 10 3 DIFF")).await,
            ["1/1", "7/1"]
        );
        let err = error_of(&format!("{DIFF} 3 DIFF")).await;
        assert!(err.contains("underflow"), "got: {err}");
    }

    /// The body starts on an empty stack: reading past its own frame is an
    /// ERROR, not a silent reach into the caller's values. When the body's
    /// stack effect is fixed, `DEF` sees that every call would fail and
    /// refuses the definition; when it depends on a value, the call fails.
    #[tokio::test]
    async fn the_body_cannot_read_below_its_frame() {
        let mut interp = Interpreter::new();
        let err = interp
            .execute("[ X | + ] 'BAD' DEF")
            .await
            .expect_err("a body that always underflows is refused")
            .to_string();
        assert!(err.contains("below its frame"), "got: {err}");
        assert!(!interp.user_words.contains_key("BAD"));

        let err = error_of("[ B | B EXEC ] 'RUN' DEF 1 2 [ + ] RUN").await;
        assert!(err.contains("underflow"), "got: {err}");
    }

    /// Every User Word states its arity: a body without a header is refused,
    /// and the dictionary is left as it was.
    #[tokio::test]
    async fn a_body_without_a_header_is_refused() {
        let mut interp = Interpreter::new();
        let err = interp
            .execute("[ 2 * ] 'TWICE' DEF")
            .await
            .expect_err("a header-less body is refused")
            .to_string();
        assert!(err.contains("parameter header"), "got: {err}");
        assert!(!interp.user_words.contains_key("TWICE"));
    }

    #[tokio::test]
    async fn a_header_makes_an_empty_body_and_a_zero_arity_meaningful() {
        assert_eq!(stack_of("[ X | ] 'DROP1' DEF 1 2 DROP1").await, ["1/1"]);
        assert_eq!(stack_of("[ | 42 ] 'K' DEF K K").await, ["42/1", "42/1"]);
    }

    /// The pilot's counterexample: `KEEP` on a Word used to keep whatever the
    /// body happened to reach. With a header it keeps what the header names.
    #[tokio::test]
    async fn keep_keeps_exactly_the_declared_operands() {
        assert_eq!(
            stack_of(&format!("{DIFF} 10 3 KEEP DIFF")).await,
            ["10/1", "3/1", "7/1"]
        );
        assert_eq!(
            stack_of("[ X | X 1 + ] 'H' DEF [ 3 ] KEEP H").await,
            ["[ 3/1 ]", "[ 4/1 ]"]
        );
    }

    /// `F` ≡ `'B' BIND 'A' BIND body`, and `KEEP F` ≡ the same with the
    /// operands pushed back first — the Core-only expansion a header buys.
    #[tokio::test]
    async fn a_header_word_expands_into_core_through_bind() {
        let body = "A B - A B *";
        let word = format!("[ A B | {body} ] 'F' DEF ");
        assert_eq!(
            stack_of(&format!("{word} 5 10 3 F")).await,
            stack_of(&format!("5 10 3 'B' BIND 'A' BIND {body}")).await
        );
        assert_eq!(
            stack_of(&format!("{word} 5 10 3 KEEP F")).await,
            stack_of(&format!("5 10 3 'B' BIND 'A' BIND A B {body}")).await
        );
    }

    /// `KEEP` modifies the `EXEC` call, not the first Word in the block. The
    /// block's frame is the whole stack, so the whole stack is what the call
    /// keeps, and what the block leaves goes on top of it.
    #[tokio::test]
    async fn keep_on_exec_keeps_the_whole_stack_it_ran_on() {
        assert_eq!(
            stack_of("[ 3 ] [ 1 + ] KEEP EXEC").await,
            ["[ 3/1 ]", "[ 1/1 + ]", "[ 4/1 ]"]
        );
        assert_eq!(
            stack_of("1 [ 3 ] [ 1 + ] KEEP EXEC").await,
            ["1/1", "[ 3/1 ]", "[ 1/1 + ]", "1/1", "[ 4/1 ]"]
        );
        assert_eq!(stack_of("[ 3 ] [ 1 + ] EXEC").await, ["[ 4/1 ]"]);
    }

    #[tokio::test]
    async fn a_malformed_header_is_an_invalid_body_and_defines_nothing() {
        for body in ["X X | X", "1 | 1", "ADD | 1", "[ X ] | 1", "SELF | 1"] {
            let mut interp = Interpreter::new();
            let err = interp
                .execute(&format!("[ {body} ] 'SELF' DEF"))
                .await
                .expect_err(&format!("`{body}` should be refused"))
                .to_string();
            assert!(err.contains("parameter header"), "`{body}`: {err}");
            assert!(
                !interp.user_words.contains_key("SELF"),
                "`{body}` defined SELF"
            );
        }
    }

    /// A parameter is a binding, not a reference: a Word later defined under
    /// the same name is neither a dependency nor what the body reads.
    #[tokio::test]
    async fn a_parameter_is_not_a_reference_to_a_word() {
        assert_eq!(
            stack_of("[ X | X 1 + ] 'H' DEF [ | 9 ] 'X' DEF 5 H 'X' DEL").await,
            ["6/1"]
        );
    }

    /// Parameters are encoded by position, so their names do not reach the
    /// Word's content identity; a name the body binds itself still does.
    #[tokio::test]
    async fn parameter_names_do_not_reach_the_digest() {
        let digests = |a: &str, b: &str| {
            format!("{a} 'M1' DEF {b} 'M2' DEF [ M1 ] 0 GET DIGEST [ M2 ] 0 GET DIGEST EQ")
        };
        assert_eq!(
            stack_of(&digests("[ V | V LENGTH ]", "[ W | W LENGTH ]")).await,
            ["TRUE"]
        );
        assert_eq!(
            stack_of(&digests("[ X | X 'V' BIND V LENGTH ]", "[ V | V LENGTH ]")).await,
            ["FALSE"]
        );
    }

    /// The rendered definition carries its header, so a saved dictionary is
    /// restored through `DEF` with the same arity.
    #[tokio::test]
    async fn the_rendered_definition_round_trips_the_header() {
        let mut interp = Interpreter::new();
        interp.execute(DIFF).await.unwrap();
        let text = interp.lookup_word_definition_tokens("DIFF").unwrap();
        assert_eq!(text, "A B | A B -");
        let mut restored = Interpreter::new();
        restored
            .execute(&format!("[ {text} ] 'DIFF' DEF 10 3 KEEP DIFF"))
            .await
            .unwrap();
        let stack: Vec<String> = restored
            .stack
            .iter_slots()
            .map(|(v, _)| v.to_string())
            .collect();
        assert_eq!(stack, ["10/1", "3/1", "7/1"]);
    }

    /// The header is the arity, and a bound name — a parameter or a
    /// `'NAME' BIND` — reads as one value rather than an unresolved Word.
    #[tokio::test]
    async fn contract_inference_reads_the_header_and_bound_names() {
        for (src, name, consumes) in [
            ("[ V | V LENGTH ] 'L' DEF", "L", 1),
            ("[ X | X 'V' BIND V V LENGTH / ] 'M' DEF", "M", 1),
            ("[ A B | A ] 'FIRST' DEF", "FIRST", 2),
        ] {
            let mut interp = Interpreter::new();
            interp.execute(src).await.unwrap();
            let contract = interp.infer_word_contract(name).expect("contract");
            assert_eq!(
                contract.flow,
                ContractFlow::Fixed {
                    consumes,
                    produces: 1
                },
                "`{src}`"
            );
            assert!(contract.gaps.is_empty(), "`{src}`: {:?}", contract.gaps);
        }
    }
}
