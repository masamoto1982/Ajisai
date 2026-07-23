//! Test suite for `crate::interpreter::control::op_or_else` (OR-ELSE).
//!
//! OR-ELSE is the value-based, block-taking counterpart to VENT (`^`). These
//! tests pin the two behaviours that matter for the P1 surface-syntax work
//! (docs/dev/external-evaluation-response-strategy.md): it mirrors VENT's
//! NIL-fallback semantics on values, and — unlike `^` — its fallback is a whole
//! `{ ... }` block, so its meaning does not depend on the lexical structure of
//! the tokens that follow.

#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::Value;

    #[tokio::test]
    async fn non_nil_candidate_is_kept_and_block_is_not_run() {
        let mut interp = Interpreter::new();
        // 7 is not NIL, so it survives and the fallback block never runs.
        let result = interp.execute("7 { 0 } OR-ELSE").await;
        assert!(result.is_ok(), "OR-ELSE should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        let f = interp.stack.last().unwrap().as_scalar().expect("scalar");
        assert_eq!(f.to_i64().unwrap(), 7, "non-NIL candidate is preserved");
    }

    #[tokio::test]
    async fn nil_candidate_runs_the_fallback_block() {
        let mut interp = Interpreter::new();
        // 1 0 / is a reasoned NIL (division by zero), so the block runs and its
        // result replaces the NIL.
        let result = interp.execute("1 0 / { 42 } OR-ELSE").await;
        assert!(result.is_ok(), "OR-ELSE should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        let f = interp.stack.last().unwrap().as_scalar().expect("scalar");
        assert_eq!(f.to_i64().unwrap(), 42, "fallback replaces the NIL");
    }

    #[tokio::test]
    async fn fallback_block_may_contain_a_whole_expression() {
        let mut interp = Interpreter::new();
        // The entire { 2 3 ADD } is the fallback unit — its internal grouping
        // does not leak. This is the robustness `^` lacks: `NIL ^ 2 3 ADD`
        // would evaluate the trailing tokens one source unit at a time.
        let result = interp.execute("1 0 / { 2 3 ADD } OR-ELSE").await;
        assert!(result.is_ok(), "OR-ELSE should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        let f = interp.stack.last().unwrap().as_scalar().expect("scalar");
        assert_eq!(f.to_i64().unwrap(), 5, "the whole block is the fallback");
    }

    #[tokio::test]
    async fn non_nil_candidate_ignores_a_multi_token_block() {
        let mut interp = Interpreter::new();
        // Symmetric to the previous test: when the candidate is kept, the whole
        // block is discarded regardless of how many tokens it holds, and the
        // stack is left with just the candidate.
        let result = interp.execute("1 { 2 3 ADD } OR-ELSE").await;
        assert!(result.is_ok(), "OR-ELSE should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1, "block leaves nothing behind");
        let f = interp.stack.last().unwrap().as_scalar().expect("scalar");
        assert_eq!(f.to_i64().unwrap(), 1);
    }

    #[tokio::test]
    async fn unknown_passes_through_like_vent() {
        // The logical UNKNOWN (U) is not NIL, so OR-ELSE keeps it and does not
        // run the fallback — matching VENT's documented U handling.
        let mut interp = Interpreter::new();
        interp.stack.push(Value::unknown());
        let result = interp.execute("{ 0 } OR-ELSE").await;
        assert!(result.is_ok(), "OR-ELSE should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
        assert!(
            interp.stack.last().unwrap().is_unknown(),
            "U passes through unchanged, fallback not run"
        );
    }

    #[tokio::test]
    async fn matches_vent_on_the_simple_nil_fallback() {
        // OR-ELSE and `^` agree on the canonical single-unit fallback.
        let mut a = Interpreter::new();
        let mut b = Interpreter::new();
        a.execute("1 0 / { 0 } OR-ELSE").await.expect("OR-ELSE");
        b.execute("1 0 / ^ 0").await.expect("VENT");
        let fa = a.stack.last().unwrap().as_scalar().expect("scalar");
        let fb = b.stack.last().unwrap().as_scalar().expect("scalar");
        assert_eq!(fa.to_i64(), fb.to_i64(), "OR-ELSE matches ^ here");
        assert_eq!(fa.to_i64().unwrap(), 0);
    }

    #[tokio::test]
    async fn missing_block_is_a_clear_error() {
        let mut interp = Interpreter::new();
        // Top of stack is a plain number, not a code block.
        let result = interp.execute("1 2 OR-ELSE").await;
        assert!(result.is_err(), "OR-ELSE without a block must fail");
        let msg = format!("{:?}", result.unwrap_err()).to_uppercase();
        assert!(
            msg.contains("OR-ELSE") && msg.contains("BLOCK"),
            "error should name OR-ELSE and the missing block: {msg}"
        );
    }

    #[tokio::test]
    async fn empty_stack_underflows() {
        let mut interp = Interpreter::new();
        let result = interp.execute("OR-ELSE").await;
        assert!(result.is_err(), "OR-ELSE on an empty stack must fail");
    }
}
