//! A malformed numeric literal is refused where it is *reached*.
//!
//! The numeric parse moved to the lexer, and this is the property that stopped it
//! from *failing* there. `1/0` tokenizes as a Number and denotes no rational, but
//! the work before it is work a program is entitled to: refusing at tokenize time
//! would erase output a program had already produced.
//!
//! Split out of `tokenizer_regression_tests` rather than added to it — that file
//! is a regression corpus for the lexer's token stream, this is a question about
//! when a refusal happens, and the 500-line budget
//! (docs/dev/specification-implementation-rules.md) wanted the split anyway.

#[cfg(test)]
mod malformed_numeric_literal_tests {
    use crate::interpreter::Interpreter;

    /// `PRINT` before the bad literal must still reach the host.
    #[tokio::test]
    async fn work_before_a_malformed_literal_still_happens() {
        for (source, expected_output) in [("1 PRINT 1/0", "1/1\n"), ("'hi' PRINT 1/0", "hi\n")] {
            let mut interp = Interpreter::new();
            let result = interp.execute(source).await;
            assert!(result.is_err(), "`{source}` must still be refused");
            assert_eq!(
                interp.host_effects().len(),
                1,
                "`{source}` must still have recorded its PRINT"
            );
            assert_eq!(
                interp.collect_output(),
                expected_output,
                "`{source}` must still have produced its output before failing"
            );
        }
    }

    /// And the refusal is the parse's own, wherever the literal is reached —
    /// bare, inside a vector literal, and in a `DEF` body.
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
