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
