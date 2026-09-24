//! Behavioral probes for the shape Words (LANG.COLLECTIONS.LIFT): the two
//! projections `SHAPE` and `RESHAPE` declare, and the depth walk `RANK`
//! shares with `MAP`.

#[cfg(test)]
mod shape_words_tests {
    use crate::interpreter::runtime_limits::RuntimeLimits;
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

    /// A ragged Vector has no shape: the question does not fit the data, which
    /// is the trichotomy's middle case, not a malformed program.
    #[tokio::test]
    async fn a_ragged_vector_projects_domain_miss_from_shape() {
        for code in [
            "[ [ 1 2 ] [ 3 ] ] SHAPE",
            "[ 1 [ 2 ] ] SHAPE",
            "[ [ [ 1 ] ] [ [ 1 2 ] ] ] SHAPE",
        ] {
            assert_eq!(
                reason(code).await.as_deref(),
                Some("domainMiss"),
                "`{code}`"
            );
        }
        for (code, want) in [
            ("[ 1 2 3 ] SHAPE", "[ 3/1 ]"),
            ("[ [ 1 2 ] [ 3 4 ] [ 5 6 ] ] SHAPE", "[ 3/1 2/1 ]"),
            ("[ 'a' 'bc' ] SHAPE", "[ 2/1 ]"),
            ("[ ] SHAPE", "[ 0/1 ]"),
            ("[ [ ] [ ] ] SHAPE", "[ 2/1 0/1 ]"),
        ] {
            assert_eq!(top(code).await, want, "`{code}`");
        }
    }

    /// A well-formed shape too large to materialize projects the same reason
    /// FILL and RANGE project, and never allocates first.
    #[tokio::test]
    async fn an_over_budget_reshape_projects_space_exhausted() {
        let mut interp = Interpreter::new();
        interp.set_runtime_limits(RuntimeLimits {
            max_materialized_elements: 8,
            ..RuntimeLimits::default()
        });
        interp
            .execute("[ 1 2 3 4 5 6 7 8 9 ] [ 3 3 ] RESHAPE")
            .await
            .expect("an over-budget RESHAPE projects rather than erroring");
        let answer = interp.stack.last().cloned().expect("an answer");
        assert!(answer.is_nil(), "must project NIL, got {answer:?}");
        assert_eq!(
            answer
                .absence_metadata()
                .and_then(|absence| absence.reason.as_ref())
                .map(|reason| reason.as_protocol_str().to_string())
                .as_deref(),
            Some("spaceExhausted")
        );
    }

    /// A shape whose product is not the leaf count is the program being wrong,
    /// so it raises rather than padding or truncating.
    #[tokio::test]
    async fn a_mismatched_shape_raises_invalid_shape() {
        for code in [
            "[ 1 2 3 4 5 ] [ 2 3 ] RESHAPE",
            "[ 1 2 3 4 ] [ 0 ] RESHAPE",
            "[ 1 2 3 4 ] [ ] RESHAPE",
            "[ 1 2 3 4 ] 'x' RESHAPE",
        ] {
            let mut interp = Interpreter::new();
            let result = interp.execute(code).await;
            let message = result
                .expect_err(&format!("`{code}` must raise"))
                .to_string();
            assert!(
                message.contains("RESHAPE"),
                "`{code}` must name RESHAPE in its diagnosis, got: {message}"
            );
        }
    }

    /// RANK at depth 1 is MAP; at depth 0 the block sees the whole Vector; a
    /// leaf met before the depth is reached is what the block gets.
    #[tokio::test]
    async fn rank_descends_to_the_stated_depth() {
        for (code, want) in [
            (
                "[ [ 1 2 ] [ 3 4 ] ] 2 [ 10 MUL ] RANK",
                "[ [ 10/1 20/1 ] [ 30/1 40/1 ] ]",
            ),
            ("[ [ 1 2 ] [ 3 4 ] ] 1 [ LENGTH ] RANK", "[ 2/1 2/1 ]"),
            ("[ [ 1 2 ] [ 3 4 ] ] 0 [ LENGTH ] RANK", "2/1"),
            ("[ 1 [ 2 3 ] ] 2 [ 10 MUL ] RANK", "[ 10/1 [ 20/1 30/1 ] ]"),
            ("[ ] 3 [ 10 MUL ] RANK", "[ ]"),
            (
                "[ 1 2 3 ] 1 [ 2 MUL ] RANK [ 1 2 3 ] [ 2 MUL ] MAP EQ",
                "TRUE",
            ),
        ] {
            assert_eq!(top(code).await, want, "`{code}`");
        }
    }
}
