#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    // ========================================================================
    // Helper
    // ========================================================================

    fn assert_stack_top_scalar(interp: &Interpreter, expected: i64, msg: &str) {
        let val = interp.stack.last().expect("Stack should not be empty");
        if let ValueData::Vector(children) = &val.data {
            assert_eq!(children.len(), 1, "{}: expected single-element vector", msg);
            let f = children[0].as_scalar().expect("Expected scalar");
            assert_eq!(f.to_i64().unwrap(), expected, "{}", msg);
        } else {
            panic!("{}: expected vector, got {:?}", msg, val.data);
        }
    }

    // ========================================================================
    // . ROUTE — single branch selection (if-else / case)
    // ========================================================================

    #[tokio::test]
    async fn test_route_branch_condition_true() {
        let mut interp = Interpreter::new();
        // ABS: if negative, multiply by -1; otherwise pass through
        // Action `[ -1 ] *` consumes the flow and produces result
        let result = interp
            .execute("[ -5 ] { ,, [ 0 ] < } { [ -1 ] * } ROUTE")
            .await;
        assert!(result.is_ok(), "ROUTE branch should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        assert_stack_top_scalar(&interp, 5, "ABS(-5) → 5");
    }

    #[tokio::test]
    async fn test_route_branch_condition_false_default() {
        let mut interp = Interpreter::new();
        // [ 5 ] is not negative → default (pass through with empty code block)
        let result = interp
            .execute("[ 5 ] { ,, [ 0 ] < } { [ -1 ] * } ROUTE")
            .await;
        assert!(result.is_ok(), "ROUTE branch should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        assert_stack_top_scalar(&interp, 5, "ABS(5) → 5 (pass-through default)");
    }

    #[tokio::test]
    async fn test_route_branch_three_way() {
        let mut interp = Interpreter::new();
        // SIGN-like: negative → multiply by -1 (→ positive), positive → keep, zero → keep
        // But we test with actions that transform the flow value
        let def = r#"{ { ,, [ 0 ] < } { [ -1 ] * } { ,, [ 0 ] = } { [ 0 ] * } ROUTE } 'ABS-OR-ZERO' DEF"#;
        interp.execute(def).await.unwrap();

        // Test negative: -5 * -1 = 5
        interp.execute("[ -5 ] ABS-OR-ZERO").await.unwrap();
        assert_stack_top_scalar(&interp, 5, "ABS-OR-ZERO(-5) → 5");
        interp.stack.clear();

        // Test zero: 0 * 0 = 0
        interp.execute("[ 0 ] ABS-OR-ZERO").await.unwrap();
        assert_stack_top_scalar(&interp, 0, "ABS-OR-ZERO(0) → 0");
        interp.stack.clear();

        // Test positive: pass through
        interp.execute("[ 10 ] ABS-OR-ZERO").await.unwrap();
        assert_stack_top_scalar(&interp, 10, "ABS-OR-ZERO(10) → 10");
    }

    #[tokio::test]
    async fn test_route_branch_no_default_pass_through() {
        let mut interp = Interpreter::new();
        // No default, condition false → flow passes through unchanged
        let result = interp
            .execute("[ 42 ] { ,, [ 0 ] < } { [ -1 ] * } ROUTE")
            .await;
        assert!(result.is_ok(), "ROUTE pass-through should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        assert_stack_top_scalar(&interp, 42, "Pass-through → original value");
    }

    #[tokio::test]
    async fn test_route_branch_default_only() {
        let mut interp = Interpreter::new();
        // Only a default block (1 code block = odd → default)
        // Action adds 1 to the flow
        let result = interp.execute("[ 99 ] { [ 1 ] + } ROUTE").await;
        assert!(result.is_ok(), "ROUTE default-only should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        assert_stack_top_scalar(&interp, 100, "Default-only → 99 + 1 = 100");
    }

    #[tokio::test]
    async fn test_route_no_code_blocks_error() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 ] ROUTE").await;
        assert!(result.is_err(), "ROUTE without code blocks should fail");
    }

    // ========================================================================
    // .. ROUTE — loop
    // ========================================================================

    #[tokio::test]
    async fn test_route_loop_count_up() {
        let mut interp = Interpreter::new();
        // Count from 0 to 5
        let result = interp
            .execute("[ 0 ] { ,, [ 5 ] < } { [ 1 ] + } .. ROUTE")
            .await;
        assert!(result.is_ok(), "ROUTE loop should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        assert_stack_top_scalar(&interp, 5, "Count up to 5");
    }

    #[tokio::test]
    async fn test_route_loop_doubling() {
        let mut interp = Interpreter::new();
        // Double until > 1000
        let result = interp
            .execute("[ 1 ] { ,, [ 1000 ] < } { [ 2 ] * } .. ROUTE")
            .await;
        assert!(result.is_ok(), "ROUTE loop should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        assert_stack_top_scalar(&interp, 1024, "Double until > 1000 → 1024");
    }

    #[tokio::test]
    async fn test_route_loop_with_default() {
        let mut interp = Interpreter::new();
        // Loop with default: when loop ends, default adds 100
        let result = interp
            .execute("[ 0 ] { ,, [ 3 ] < } { [ 1 ] + } { [ 100 ] + } .. ROUTE")
            .await;
        assert!(result.is_ok(), "ROUTE loop with default: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        assert_stack_top_scalar(&interp, 103, "Loop to 3 then default adds 100 → 103");
    }

    #[tokio::test]
    async fn test_route_loop_immediate_exit() {
        let mut interp = Interpreter::new();
        // Condition immediately false → no iterations
        let result = interp
            .execute("[ 100 ] { ,, [ 5 ] < } { [ 1 ] + } .. ROUTE")
            .await;
        assert!(result.is_ok(), "ROUTE loop immediate exit: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        assert_stack_top_scalar(&interp, 100, "Immediate exit → 100 unchanged");
    }

    #[tokio::test]
    async fn test_route_loop_multi_condition() {
        let mut interp = Interpreter::new();
        // Collatz-like: while != 1, if even /2 else *3+1
        // Start with 4: 4 → 2 → 1 (stop)
        // Outer loop condition: != 1
        // Inner branching: even → /2, odd → *3+1
        // We use a single loop with two condition-action pairs
        let result = interp
            .execute("[ 4 ] { ,, [ 2 ] MOD [ 0 ] = } { [ 2 ] / } { ,, [ 1 ] = NOT } { [ 3 ] * [ 1 ] + } .. ROUTE")
            .await;
        assert!(result.is_ok(), "ROUTE multi-cond loop: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        assert_stack_top_scalar(&interp, 1, "Collatz(4) → 1");
    }

    // ========================================================================
    // ,, ROUTE — bifurcation (keep original flow)
    // ========================================================================

    #[tokio::test]
    async fn test_route_bifurcation() {
        let mut interp = Interpreter::new();
        // [ 5 ] with bifurcation: positive → multiply by 2, keep original
        let result = interp
            .execute("[ 5 ] { ,, [ 0 ] < NOT } { [ 2 ] * } ,, ROUTE")
            .await;
        assert!(result.is_ok(), ",, ROUTE should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 2, "Should have original + result");
        // Stack: [ 5 ] (original) then [ 10 ] (result)
        assert_stack_top_scalar(&interp, 10, "Result is 10");
        let first = &interp.stack[0];
        if let ValueData::Vector(children) = &first.data {
            assert_eq!(children[0].as_scalar().unwrap().to_i64().unwrap(), 5);
        } else {
            panic!("Expected vector for original flow");
        }
    }

    // ========================================================================
    // .. ,, ROUTE — loop + bifurcation
    // ========================================================================

    #[tokio::test]
    async fn test_route_loop_bifurcation() {
        let mut interp = Interpreter::new();
        // Start with [ 1 ], loop double while < 100, keep original
        let result = interp
            .execute("[ 1 ] { ,, [ 100 ] < } { [ 2 ] * } .. ,, ROUTE")
            .await;
        assert!(result.is_ok(), ".. ,, ROUTE should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 2, "Should have original + result");
        // Original [ 1 ] + result [ 128 ]
        let first = &interp.stack[0];
        if let ValueData::Vector(children) = &first.data {
            assert_eq!(children[0].as_scalar().unwrap().to_i64().unwrap(), 1);
        }
        assert_stack_top_scalar(&interp, 128, "Double until >= 100 → 128");
    }

    // ========================================================================
    // ROUTE in DEF — custom words using ROUTE
    // ========================================================================

    #[tokio::test]
    async fn test_route_in_custom_word() {
        let mut interp = Interpreter::new();
        // ABS word using ROUTE
        let def = r#"{ { ,, [ 0 ] < } { [ -1 ] * } ROUTE } 'ABS' DEF"#;
        interp.execute(def).await.unwrap();

        interp.execute("[ -5 ] ABS").await.unwrap();
        assert_stack_top_scalar(&interp, 5, "ABS(-5) → 5");
        interp.stack.clear();

        interp.execute("[ 3 ] ABS").await.unwrap();
        assert_stack_top_scalar(&interp, 3, "ABS(3) → 3");
    }

    // ========================================================================
    // Code block push (preserved from old tests)
    // ========================================================================

    #[tokio::test]
    async fn test_code_block_push() {
        let mut interp = Interpreter::new();
        let result = interp.execute("{ [ 0 ] [ 1 ] + }").await;
        assert!(result.is_ok(), "Code block should parse successfully");
        assert_eq!(interp.stack.len(), 1);
        assert!(interp.stack[0].as_code_block().is_some());
    }

    // ========================================================================
    // Edge cases
    // ========================================================================

    #[tokio::test]
    async fn test_route_empty_action() {
        let mut interp = Interpreter::new();
        // Condition true, action is empty code block (no-op) → flow unchanged
        let result = interp
            .execute("[ 42 ] { ,, [ 0 ] < NOT } { } ROUTE")
            .await;
        assert!(result.is_ok(), "Empty action: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        assert_stack_top_scalar(&interp, 42, "Empty action → unchanged");
    }
}
