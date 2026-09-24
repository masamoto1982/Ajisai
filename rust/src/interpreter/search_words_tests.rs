//! Behavioral probes for the search Words: the projections `BSEARCH` and
//! `SEARCH` declare, `BSEARCH`'s order check, and `MEMBER?` over every domain.

#[cfg(test)]
mod search_words_tests {
    use crate::interpreter::Interpreter;

    async fn top(code: &str) -> String {
        let mut interp = Interpreter::new();
        interp
            .execute(code)
            .await
            .unwrap_or_else(|e| panic!("`{code}` must not error: {e}"));
        interp
            .get_stack()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    }

    async fn reason(code: &str) -> Option<String> {
        let mut interp = Interpreter::new();
        interp
            .execute(code)
            .await
            .unwrap_or_else(|e| panic!("`{code}` must not error: {e}"));
        let answer = interp.stack.last().cloned().expect("an answer");
        assert!(answer.is_nil(), "`{code}` must project NIL, got {answer:?}");
        answer
            .absence_metadata()
            .and_then(|absence| absence.reason.as_ref())
            .map(|reason| reason.as_protocol_str().to_string())
    }

    async fn raises(code: &str, naming: &str) {
        let mut interp = Interpreter::new();
        let message = interp
            .execute(code)
            .await
            .expect_err(&format!("`{code}` must raise"))
            .to_string();
        assert!(
            message.contains(naming),
            "`{code}` must raise naming `{naming}`, got: {message}"
        );
    }

    #[tokio::test]
    async fn member_answers_lane_for_lane_over_any_domain() {
        for (code, want) in [
            ("[ 1 2 3 ] 2 MEMBER?", "TRUE"),
            ("[ 1 2 3 ] 5 MEMBER?", "FALSE"),
            ("[ 'a' 'b' ] 'b' MEMBER?", "TRUE"),
            ("[ [ 1 2 ] [ 3 ] ] [ 3 ] MEMBER?", "TRUE"),
            ("[ [ 1 2 ] [ 3 ] ] [ 1 ] MEMBER?", "FALSE"),
            ("[ ] 1 MEMBER?", "FALSE"),
            ("[ 1 2 ] [ ] MEMBER?", "FALSE"),
        ] {
            assert_eq!(top(code).await, want, "`{code}`");
        }
    }

    #[tokio::test]
    async fn bsearch_answers_the_first_index_and_projects_an_absent_key() {
        for (code, want) in [
            ("[ 1 3 5 7 ] [ 5 ] BSEARCH", "[ 2/1 ]"),
            ("[ 1 3 5 7 ] 5 BSEARCH", "2/1"),
            ("[ 1 3 3 3 7 ] 3 BSEARCH", "1/1"),
            ("[ 1 3 5 7 ] [ 1 7 ] BSEARCH", "[ 0/1 3/1 ]"),
            ("[ 1/2 3/2 ] 3/2 BSEARCH", "1/1"),
        ] {
            assert_eq!(top(code).await, want, "`{code}`");
        }
        assert_eq!(
            reason("[ 1 3 5 7 ] 4 BSEARCH").await.as_deref(),
            Some("missingField")
        );
        assert_eq!(
            top("[ 1 3 5 7 ] [ 4 5 ] BSEARCH [ 0 ] GET NIL-REASON").await,
            "'missingField'"
        );
        assert_eq!(
            reason("[ ] 4 BSEARCH").await.as_deref(),
            Some("missingField")
        );
    }

    /// The order is checked before the search: an unsorted operand is the
    /// program being wrong.
    #[tokio::test]
    async fn bsearch_checks_the_order_first() {
        raises("[ 3 1 2 ] [ 2 ] BSEARCH", "BSEARCH").await;
        raises(
            "[ 1 'a' ] 1 BSEARCH",
            "expected Scalar elements, got String",
        )
        .await;
        raises(
            "[ 1 2 3 ] 'a' BSEARCH",
            "expected Scalar elements, got String",
        )
        .await;
    }

    #[tokio::test]
    async fn search_counts_characters_and_projects_an_absent_needle() {
        for (code, want) in [
            ("'hello world' 'world' SEARCH", "6/1"),
            ("'hello' 'l' SEARCH", "2/1"),
            ("'hello' '' SEARCH", "0/1"),
            ("'こんにちは' 'ち' SEARCH", "3/1"),
        ] {
            assert_eq!(top(code).await, want, "`{code}`");
        }
        assert_eq!(
            reason("'hello' 'z' SEARCH").await.as_deref(),
            Some("missingField")
        );
        raises("'hello' 1 SEARCH", "SEARCH").await;
    }

    #[tokio::test]
    async fn replace_substitutes_every_occurrence_without_overlap() {
        for (code, want) in [
            ("'a-b-c' '-' '+' REPLACE", "'a+b+c'"),
            ("'aaa' 'aa' 'b' REPLACE", "'ba'"),
            ("'hello' 'z' 'y' REPLACE", "'hello'"),
            ("'hello' '' 'x' REPLACE", "'hello'"),
            ("'hello' 'l' '' REPLACE", "'heo'"),
        ] {
            assert_eq!(top(code).await, want, "`{code}`");
        }
        raises("'a' 'b' 3 REPLACE", "REPLACE").await;
    }
}
