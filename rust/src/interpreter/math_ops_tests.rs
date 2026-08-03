//! Test suite for `crate::interpreter::math_ops` (MATH module ABS/NEG/SIGN/MIN/MAX).

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    async fn top_i64(program: &str) -> i64 {
        let mut interp = Interpreter::new();
        interp
            .execute(program)
            .await
            .expect("program should succeed");
        assert_eq!(interp.stack.len(), 1, "program: {program}");
        interp.stack[0]
            .as_scalar()
            .expect("expected scalar result")
            .to_i64()
            .expect("expected integer result")
    }

    /// Render the top stack value, for comparing exact-real values (including
    /// lazy irrationals) by observation rather than by an integer projection.
    async fn render_top(program: &str) -> String {
        let mut interp = Interpreter::new();
        interp
            .execute(program)
            .await
            .expect("program should succeed");
        interp.stack.last().expect("non-empty stack").to_string()
    }

    #[tokio::test]
    async fn abs_of_negative_is_positive() {
        assert_eq!(top_i64("-7 ABS").await, 7);
    }

    #[tokio::test]
    async fn neg_flips_sign() {
        assert_eq!(top_i64("5 NEG").await, -5);
        assert_eq!(top_i64("-3 NEG").await, 3);
    }

    /// NEG computes the additive inverse directly on the exact-real
    /// representation, so it accepts lazy continued-fraction operands (not only
    /// rationals) and round-trips under double negation.
    #[tokio::test]
    async fn neg_handles_lazy_irrationals() {
        // NEG(√2) equals 0 - √2 (both -√2), compared exactly through EQ.
        assert_eq!(render_top("2 SQRT NEG 0 2 SQRT SUB EQ").await, "TRUE");
        // Double negation returns √2.
        assert_eq!(render_top("2 SQRT NEG NEG 2 SQRT EQ").await, "TRUE");
    }

    /// ABS decides the sign against 0 through the budgeted comparison
    /// (SPEC §7.4.3) and negates when negative, so it accepts the full numeric
    /// domain including lazy continued-fraction operands.
    #[tokio::test]
    async fn abs_handles_lazy_irrationals() {
        // |√2| = √2.
        assert_eq!(render_top("2 SQRT ABS 2 SQRT EQ").await, "TRUE");
        // |−√2| = √2, with −√2 built via 0 - √2.
        assert_eq!(render_top("0 2 SQRT SUB ABS 2 SQRT EQ").await, "TRUE");
        // |√2 − √2| = 0 decides exactly.
        assert_eq!(top_i64("2 SQRT 2 SQRT SUB ABS").await, 0);
    }

    #[tokio::test]
    async fn min_and_max_pick_correctly() {
        assert_eq!(top_i64("3 8 MIN").await, 3);
        assert_eq!(top_i64("3 8 MAX").await, 8);
        assert_eq!(top_i64("-2 -9 MIN").await, -9);
        assert_eq!(top_i64("-2 -9 MAX").await, -2);
    }

    #[tokio::test]
    async fn abs_handles_fractions() {
        let mut interp = Interpreter::new();
        interp.execute("-3/4 ABS").await.expect("should succeed");
        let scalar = interp.stack[0].as_scalar().expect("scalar");
        assert_eq!(scalar.numerator().to_string(), "3");
        assert_eq!(scalar.denominator().to_string(), "4");
    }

    #[tokio::test]
    async fn nil_passes_through_unary() {
        let mut interp = Interpreter::new();
        interp
            .execute("NIL ABS")
            .await
            .expect("NIL passthrough should not error");
        assert_eq!(interp.stack.len(), 1);
        assert!(interp.stack[0].is_nil(), "ABS of NIL should be NIL");
    }

    #[tokio::test]
    async fn nil_passes_through_binary() {
        let mut interp = Interpreter::new();
        interp
            .execute("NIL 5 MIN")
            .await
            .expect("NIL passthrough should not error");
        assert_eq!(interp.stack.len(), 1);
        assert!(interp.stack[0].is_nil(), "MIN with NIL should be NIL");
    }

    #[tokio::test]
    async fn non_number_input_errors() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'hello' ABS").await;
        assert!(
            result.is_err(),
            "ABS of text should be a malformed-use error"
        );
    }
    #[tokio::test]
    async fn keep_mode_retains_operands() {
        let mut interp = Interpreter::new();
        interp
            .execute("3 8 ,, MIN")
            .await
            .expect("keep mode should succeed");
        assert_eq!(interp.stack.len(), 3, "operands retained plus result");
        assert_eq!(interp.stack[2].as_scalar().unwrap().to_i64().unwrap(), 3);
    }
}
