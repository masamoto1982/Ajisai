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

    #[tokio::test]
    async fn abs_of_negative_is_positive() {
        assert_eq!(top_i64("'math' IMPORT -7 ABS").await, 7);
    }

    #[tokio::test]
    async fn neg_flips_sign() {
        assert_eq!(top_i64("'math' IMPORT 5 NEG").await, -5);
        assert_eq!(top_i64("'math' IMPORT -3 NEG").await, 3);
    }

    #[tokio::test]
    async fn sign_reports_three_values() {
        assert_eq!(top_i64("'math' IMPORT -42 SIGN").await, -1);
        assert_eq!(top_i64("'math' IMPORT 0 SIGN").await, 0);
        assert_eq!(top_i64("'math' IMPORT 42 SIGN").await, 1);
    }

    /// SIGN decides the order against 0 through the budgeted comparison
    /// (SPEC §7.4.3), so it accepts the full numeric domain — including lazy
    /// continued-fraction operands like `2 SQRT` — rather than only rationals.
    #[tokio::test]
    async fn sign_handles_lazy_irrationals() {
        assert_eq!(top_i64("'math' IMPORT 2 SQRT SIGN").await, 1);
        // -√2, built without NEG (which does not yet accept lazy operands).
        assert_eq!(top_i64("'math' IMPORT 0 2 SQRT SUB SIGN").await, -1);
        // √2 - √2 = 0 decides exactly to sign 0.
        assert_eq!(top_i64("'math' IMPORT 2 SQRT 2 SQRT SUB SIGN").await, 0);
        // √3 > √2, so their difference signs positive.
        assert_eq!(top_i64("'math' IMPORT 3 SQRT 2 SQRT SUB SIGN").await, 1);
    }

    /// SIGN is NIL-passthrough: a NIL operand yields NIL, not a sign.
    #[tokio::test]
    async fn sign_passes_nil_through() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 1 0 / SIGN")
            .await
            .expect("program should succeed");
        assert_eq!(interp.stack.len(), 1);
        assert!(
            interp.stack[0].is_nil(),
            "SIGN of NIL should be NIL, got {:?}",
            interp.stack[0]
        );
    }

    #[tokio::test]
    async fn min_and_max_pick_correctly() {
        assert_eq!(top_i64("'math' IMPORT 3 8 MIN").await, 3);
        assert_eq!(top_i64("'math' IMPORT 3 8 MAX").await, 8);
        assert_eq!(top_i64("'math' IMPORT -2 -9 MIN").await, -9);
        assert_eq!(top_i64("'math' IMPORT -2 -9 MAX").await, -2);
    }

    #[tokio::test]
    async fn abs_handles_fractions() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT -3/4 ABS")
            .await
            .expect("should succeed");
        let scalar = interp.stack[0].as_scalar().expect("scalar");
        assert_eq!(scalar.numerator().to_string(), "3");
        assert_eq!(scalar.denominator().to_string(), "4");
    }

    #[tokio::test]
    async fn nil_passes_through_unary() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT NIL ABS")
            .await
            .expect("NIL passthrough should not error");
        assert_eq!(interp.stack.len(), 1);
        assert!(interp.stack[0].is_nil(), "ABS of NIL should be NIL");
    }

    #[tokio::test]
    async fn nil_passes_through_binary() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT NIL 5 MIN")
            .await
            .expect("NIL passthrough should not error");
        assert_eq!(interp.stack.len(), 1);
        assert!(interp.stack[0].is_nil(), "MIN with NIL should be NIL");
    }

    #[tokio::test]
    async fn non_number_input_errors() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'math' IMPORT 'hello' ABS").await;
        assert!(
            result.is_err(),
            "ABS of text should be a malformed-use error"
        );
    }

    #[tokio::test]
    async fn stack_mode_is_rejected() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'math' IMPORT 1 2 3 .. MAX").await;
        assert!(result.is_err(), "MAX should reject Stack mode");
        assert!(result.unwrap_err().to_string().contains("Stack mode"));
    }

    #[tokio::test]
    async fn pow_positive_exponent() {
        assert_eq!(top_i64("'math' IMPORT 2 10 POW").await, 1024);
        assert_eq!(top_i64("'math' IMPORT 5 0 POW").await, 1);
        assert_eq!(top_i64("'math' IMPORT -3 3 POW").await, -27);
    }

    #[tokio::test]
    async fn pow_negative_exponent_is_reciprocal() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 2 -2 POW")
            .await
            .expect("should succeed");
        let scalar = interp.stack[0].as_scalar().expect("scalar");
        assert_eq!(scalar.numerator().to_string(), "1");
        assert_eq!(scalar.denominator().to_string(), "4");
    }

    #[tokio::test]
    async fn pow_zero_to_negative_is_bubble() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 0 -1 POW")
            .await
            .expect("0^negative should be a well-formed Bubble, not an error");
        assert_eq!(interp.stack.len(), 1);
        assert!(interp.stack[0].is_nil(), "0^negative should project to NIL");
    }

    #[tokio::test]
    async fn pow_non_integer_exponent_errors() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'math' IMPORT 2 1/2 POW").await;
        assert!(result.is_err(), "non-integer exponent is malformed use");
    }

    #[tokio::test]
    async fn pow_huge_exponent_is_bounded() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'math' IMPORT 2 2000000 POW").await;
        assert!(result.is_err(), "exponent past the safety bound errors");
    }

    #[tokio::test]
    async fn pow_nil_passes_through() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT NIL 2 POW")
            .await
            .expect("NIL passthrough should not error");
        assert!(interp.stack[0].is_nil());
    }

    #[tokio::test]
    async fn gcd_and_lcm_basic() {
        assert_eq!(top_i64("'math' IMPORT 12 18 GCD").await, 6);
        assert_eq!(top_i64("'math' IMPORT -12 18 GCD").await, 6);
        assert_eq!(top_i64("'math' IMPORT 0 0 GCD").await, 0);
        assert_eq!(top_i64("'math' IMPORT 4 6 LCM").await, 12);
        assert_eq!(top_i64("'math' IMPORT 0 5 LCM").await, 0);
    }

    #[tokio::test]
    async fn gcd_non_integer_errors() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'math' IMPORT 3/2 6 GCD").await;
        assert!(result.is_err(), "GCD of a non-integer is malformed use");
    }

    #[tokio::test]
    async fn lcm_nil_passes_through() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 4 NIL LCM")
            .await
            .expect("NIL passthrough should not error");
        assert!(interp.stack[0].is_nil());
    }

    #[tokio::test]
    async fn keep_mode_retains_operands() {
        let mut interp = Interpreter::new();
        interp
            .execute("'math' IMPORT 3 8 ,, MIN")
            .await
            .expect("keep mode should succeed");
        assert_eq!(interp.stack.len(), 3, "operands retained plus result");
        assert_eq!(interp.stack[2].as_scalar().unwrap().to_i64().unwrap(), 3);
    }
}
