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
