//! CS3 (ownership): the `Stack` is the sole authority for top-level roles, and
//! every save/restore boundary carries roles with values in lockstep.
//!
//! These drive the interpreter and observe through the shared `(value, role)`
//! rendering (SPEC §12), so they exercise the real save/restore paths that were
//! migrated from the parallel `SemanticStack` snapshot onto a `Stack` clone: a
//! position cast (`>CF`) applied to a slot *below* an isolated-stack word must
//! survive that word with its role intact.

use crate::interpreter::Interpreter;
use crate::types::display::render_stack;

async fn render(code: &str) -> Vec<String> {
    let mut interp = Interpreter::new();
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` unexpectedly errored: {e}"));
    render_stack(interp.get_stack())
}

#[tokio::test]
async fn cond_preserves_a_lower_slot_cast_role() {
    // `1/3 >CF` sits below the COND subject. COND saves and restores the
    // surrounding stack around each clause; the restored lower slot must keep
    // its ContinuedFraction role, so it still renders `( 0 ( 3 ) )`.
    let stack = render("1/3 >CF 5 { [ 2 ] EQ } { [ 200 ] } { IDLE } { [ 0 ] } COND").await;
    assert_eq!(
        stack,
        vec!["( 0 ( 3 ) )".to_string(), "[ 0/1 ]".to_string()]
    );
}

#[tokio::test]
async fn cond_pipe_clause_form_preserves_a_lower_slot_cast_role() {
    let stack = render("1/3 >CF 2 { [ 2 ] EQ | [ 200 ] }\n{ IDLE | [ 0 ] }\nCOND").await;
    assert_eq!(
        stack,
        vec!["( 0 ( 3 ) )".to_string(), "[ 200/1 ]".to_string()]
    );
}

#[tokio::test]
async fn count_preserves_a_lower_slot_cast_role() {
    // COUNT runs each predicate on an isolated stack, saving/restoring the
    // outer stack. The `1/3 >CF` slot below the target vector must survive.
    let stack = render("1/3 >CF [ 1 2 3 4 ] { [ 2 ] EAT MOD 0 EQ } COUNT").await;
    assert_eq!(
        stack,
        vec!["( 0 ( 3 ) )".to_string(), "[ 2/1 ]".to_string()]
    );
}
