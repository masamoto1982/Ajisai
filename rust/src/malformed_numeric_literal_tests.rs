//! A malformed numeric literal is a source error, refused before anything runs.
//!
//! `1/0` has the shape of a number and denotes none. It used to tokenize as a
//! Number and fail only when reached, so `1 PRINT 1/0` printed and then failed —
//! a source error surfacing halfway through a run, the one source error that
//! did. The lexical grammar (`spec/grammar.json`) is total: text is either one
//! token sequence or one source error, decided before evaluation. The zero
//! denominator is now one of those errors, like an unclosed string.
//!
//! Split out of `tokenizer_regression_tests` rather than added to it — that file
//! is a regression corpus for the lexer's token stream, this is a question about
//! when a refusal happens, and the 500-line budget
//! (docs/dev/specification-implementation-rules.md) wanted the split anyway.

#[cfg(test)]
mod malformed_numeric_literal_tests {
    use crate::interpreter::Interpreter;

    /// Nothing before the bad literal runs: the program is refused whole.
    #[tokio::test]
    async fn nothing_before_a_malformed_literal_runs() {
        for source in ["1 PRINT 1/0", "'hi' PRINT 1/0"] {
            let mut interp = Interpreter::new();
            let result = interp.execute(source).await;
            assert!(result.is_err(), "`{source}` must be refused");
            assert!(
                interp.host_effects().is_empty(),
                "`{source}` must be refused before its PRINT runs"
            );
            assert_eq!(interp.collect_output(), "", "`{source}` printed");
        }
    }

    /// One numeric grammar, one answer at every entry point: a zero mantissa
    /// is zero at any scale, so `0e2147483648` — an exponent past `i32` —
    /// denotes 0 in source, through `NUM` and through `JSON-DECODE` alike. It
    /// used to be three things (a mid-run `malformedSource`, a NIL and 0),
    /// because the exponent's range was checked before the mantissa's zero.
    #[tokio::test]
    async fn a_zero_mantissa_is_zero_past_the_exponent_range_everywhere() {
        for source in [
            "1 PRINT 0e2147483648",
            "1 PRINT '0e2147483648' NUM",
            "1 PRINT '[0e2147483648]' JSON-DECODE 0 GET",
            "1 PRINT 0e-2147483649",
        ] {
            let mut interp = Interpreter::new();
            interp
                .execute(source)
                .await
                .unwrap_or_else(|e| panic!("`{source}` must run: {e}"));
            assert_eq!(interp.collect_output(), "1/1\n", "`{source}`");
            assert_eq!(
                interp.get_stack().last().and_then(|v| v.as_i64()),
                Some(0),
                "`{source}` must leave 0"
            );
        }
        // The exponent's form is still checked: a malformed one is not a number.
        let mut interp = Interpreter::new();
        interp.execute("'0eX' NUM NIL?").await.unwrap();
        assert_eq!(
            interp.get_stack().last().map(|v| v.as_truth()),
            Some(Some(true))
        );
    }

    /// And the refusal is the same wherever the literal is written — bare,
    /// inside a vector literal, and in a `DEF` body.
    #[tokio::test]
    async fn a_malformed_literal_is_refused_with_the_parses_own_message() {
        for source in [
            "1/0",
            "0/0",
            "-1/0",
            "[ 1/0 ]",
            "1/0 2 ADD",
            "[ 1/0 MUL ] 'W' DEF",
        ] {
            let mut interp = Interpreter::new();
            let error = interp
                .execute(source)
                .await
                .expect_err(&format!("`{source}` must be refused"));
            let text = format!("{error:?}");
            assert!(
                text.contains("MalformedSource") && text.contains("zero denominator"),
                "`{source}` must be refused as a malformed source with the \
                 parse's own reason, got: {text}"
            );
        }
    }

    /// A spelling the lexer does not accept as a number stays a name, so it is
    /// an unknown word rather than a malformed literal. This is the boundary the
    /// case above sits against.
    #[tokio::test]
    async fn a_non_numeric_spelling_is_not_a_numeric_literal_at_all() {
        for source in ["1.2.3", "1/2/3"] {
            let mut interp = Interpreter::new();
            let error = interp
                .execute(source)
                .await
                .expect_err(&format!("`{source}` must be refused"));
            assert!(
                format!("{error:?}").contains("UnknownWord"),
                "`{source}` must be an unknown word, not a numeric literal"
            );
        }
    }
}
