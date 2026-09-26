//! Test suite for tensor dimension-limit enforcement (`crate::interpreter::tensor_ops`).

#[cfg(test)]
mod dimension_limit_tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn test_dimension_limit_at_3_visible() {
        let mut interp = Interpreter::new();
        interp.execute("").await.unwrap();
        let result = interp.execute("[ [ [ 1 2 3 ] ] ]").await;
        assert!(result.is_ok(), "3 visible dimensions should succeed");

        let stack = interp.get_stack();
        assert_eq!(
            stack.len(),
            1,
            "Stack should have 1 element after parsing 3D tensor"
        );
    }

    #[tokio::test]
    async fn test_dimension_4_visible_succeeds() {
        let mut interp = Interpreter::new();
        interp.execute("").await.unwrap();
        let result = interp.execute("[ [ [ [ 1/1 ] ] ] ]").await;
        assert!(
            result.is_ok(),
            "4 visible dimensions should succeed with new limit"
        );
    }

    #[tokio::test]
    async fn test_dimension_5_visible_succeeds() {
        let mut interp = Interpreter::new();
        interp.execute("").await.unwrap();
        let result = interp.execute("[ [ [ [ [ 1 ] ] ] ] ]").await;
        assert!(result.is_ok(), "5 visible dimensions should succeed");
    }

    #[tokio::test]
    async fn test_dimension_limit_at_9_visible() {
        let mut interp = Interpreter::new();
        interp.execute("").await.unwrap();
        let result = interp
            .execute("[ [ [ [ [ [ [ [ [ 1/1 ] ] ] ] ] ] ] ] ]")
            .await;
        assert!(
            result.is_ok(),
            "9 visible dimensions (10 total) should succeed: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_dimension_10_visible_succeeds() {
        let mut interp = Interpreter::new();
        interp.execute("").await.unwrap();
        let result = interp
            .execute("[ [ [ [ [ [ [ [ [ [ 1 ] ] ] ] ] ] ] ] ] ]")
            .await;
        assert!(
            result.is_ok(),
            "10 visible dimensions should succeed after removing the dimension limit"
        );
    }

    #[tokio::test]
    async fn test_deeply_nested_vector_succeeds() {
        let mut interp = Interpreter::new();
        interp.execute("").await.unwrap();
        let result = interp
            .execute("[ [ [ [ [ [ [ [ [ [ [ [ 1 ] ] ] ] ] ] ] ] ] ] ] ]")
            .await;
        assert!(
            result.is_ok(),
            "Deeply nested vectors should succeed after removing the dimension limit"
        );
    }

    #[tokio::test]
    async fn test_bracket_display_1d() {
        let mut interp = Interpreter::new();
        interp.execute("").await.unwrap();
        interp.execute("[ 1 2 3 ]").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert!(
            result.starts_with('['),
            "1D should display with [ ], got: {}",
            result
        );
        assert!(
            result.ends_with(']'),
            "1D should display with [ ], got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_bracket_display_2d() {
        let mut interp = Interpreter::new();
        interp.execute("").await.unwrap();
        interp.execute("[ [ 1 2 ] [ 3 4 ] ]").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert!(
            result.starts_with('['),
            "2D outermost should be [ ], got: {}",
            result
        );
        assert!(
            result.contains("[ 1/1 2/1 ]"),
            "2D inner should use [ ], got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_bracket_display_3d() {
        let mut interp = Interpreter::new();
        interp.execute("").await.unwrap();
        interp.execute("[ [ [ 1 ] ] ]").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert!(
            result.starts_with('['),
            "3D outermost should be [ ], got: {}",
            result
        );
        assert!(
            result.contains('['),
            "3D innermost should contain [], got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_bracket_display_3d_complex() {
        let mut interp = Interpreter::new();
        interp.execute("").await.unwrap();
        interp
            .execute("[ [ [ 1/1 ] [ 2/1 ] [ 3/1 ] ] [ [ 4/1 ] [ 5/1 ] [ 6/1 ] ] ]")
            .await
            .unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert!(
            result.starts_with('['),
            "3D outermost should be [ ], got: {}",
            result
        );
        assert!(
            result.contains('['),
            "3D innermost should contain [], got: {}",
            result
        );
        assert_eq!(
            result, "[ [ [ 1/1 ] [ 2/1 ] [ 3/1 ] ] [ [ 4/1 ] [ 5/1 ] [ 6/1 ] ] ]",
            "Expected 3D structure"
        );
    }

    #[tokio::test]
    async fn test_bracket_display_4d() {
        let mut interp = Interpreter::new();
        interp.execute("").await.unwrap();
        interp.execute("[ [ [ [ 1/1 ] ] ] ]").await.unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert_eq!(
            result, "[ [ [ [ 1/1 ] ] ] ]",
            "4D should keep [ ] brackets: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_bracket_display_9d() {
        let mut interp = Interpreter::new();
        interp.execute("").await.unwrap();
        interp
            .execute("[ [ [ [ [ [ [ [ [ 1/1 ] ] ] ] ] ] ] ] ]")
            .await
            .unwrap();
        let stack = interp.get_stack();
        let result = format!("{}", stack[0]);
        assert_eq!(
            result, "[ [ [ [ [ [ [ [ [ 1/1 ] ] ] ] ] ] ] ] ]",
            "9D unified [ ] display: {}",
            result
        );
    }

    // Regression: deeply nested vector literals must be rejected before they
    // build a value whose recursive display/drop overflows the native stack
    // (an unrecoverable abort / WASM trap). See DEFAULT_MAX_NESTING_DEPTH.
    fn nested_literal(depth: usize) -> String {
        format!("{}1{}", "[ ".repeat(depth), " ]".repeat(depth))
    }

    #[tokio::test]
    async fn test_excessive_vector_nesting_errors_not_aborts() {
        let mut interp = Interpreter::new();
        let result = interp.execute(&nested_literal(5000)).await;
        assert!(
            result.is_err(),
            "5000-deep vector nesting must be a recoverable error, not a stack-overflow abort"
        );
        assert!(matches!(
            result.unwrap_err(),
            crate::error::AjisaiError::ResourceLimitExceeded {
                resource: crate::error::ResourceLimit::NestingDepth,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_excessive_nesting_survivable_and_recovers() {
        // After the deep-nesting error, the interpreter stays usable: it did not
        // build (and then have to recursively drop) the pathological value.
        let mut interp = Interpreter::new();
        assert!(interp.execute(&nested_literal(5000)).await.is_err());
        assert!(
            interp.execute("[ 1 2 3 ]").await.is_ok(),
            "interpreter must remain usable after a deep-nesting rejection"
        );
    }

    #[tokio::test]
    async fn test_moderate_vector_nesting_still_succeeds() {
        // Well within the cap: ordinary nested data keeps working.
        let mut interp = Interpreter::new();
        assert!(
            interp.execute(&nested_literal(32)).await.is_ok(),
            "32-deep nesting is far below the cap and must succeed"
        );
    }

    /// A value Words build meets the same ceiling a literal does. Each FOLD
    /// step wraps the accumulator one level deeper; before the ceiling covered
    /// built values, twenty thousand steps aborted the process in the first
    /// walk over the result.
    #[tokio::test]
    async fn a_value_built_past_the_nesting_ceiling_is_refused_not_aborted() {
        let mut interp = Interpreter::new();
        let err = interp
            .execute("0 20000 RANGE [ ] [ 2 COLLECT 1 TAKE ] FOLD DEPTH")
            .await
            .expect_err("a value nested 20,000 deep must be refused");
        assert!(matches!(
            err,
            crate::error::AjisaiError::ResourceLimitExceeded {
                resource: crate::error::ResourceLimit::NestingDepth,
                limit: 256,
                observed: Some(257),
                ..
            }
        ));
        let mut interp = Interpreter::new();
        interp
            .execute("0 254 RANGE [ ] [ 2 COLLECT 1 TAKE ] FOLD DEPTH")
            .await
            .expect("a value nested exactly to the ceiling is kept");
        assert_eq!(interp.get_stack()[0].to_string(), "256/1");
    }

    /// A generative Word asked for a result nested past the ceiling declines
    /// it, as it declines a count past `materializedElements`, and names the
    /// ceiling it declined under.
    #[tokio::test]
    async fn generative_words_decline_a_result_past_the_nesting_ceiling() {
        let axes = vec!["1"; 300].join(" ");
        let deep_json = format!("'{}{}' JSON-DECODE", "[".repeat(300), "]".repeat(300));
        for (word, source) in [
            ("FILL", format!("[ {axes} ] 7 FILL")),
            ("RESHAPE", format!("[ 7 ] [ {axes} ] RESHAPE")),
            ("JSON-DECODE", deep_json),
        ] {
            let mut interp = Interpreter::new();
            interp
                .execute(&source)
                .await
                .unwrap_or_else(|e| panic!("{word}: {e}"));
            let top = &interp.get_stack()[0];
            assert_eq!(
                top.nil_reason(),
                Some(&crate::error::NilReason::SpaceExhausted),
                "{word}"
            );
            let facts = top
                .absence
                .as_ref()
                .and_then(|a| a.diagnosis.as_ref())
                .and_then(|d| d.resource_limit.as_ref())
                .unwrap_or_else(|| panic!("{word}: the NIL names its ceiling"));
            assert_eq!(facts.resource, "nestingDepth", "{word}");
            assert_eq!(facts.limit, 256, "{word}");
        }
    }
}
