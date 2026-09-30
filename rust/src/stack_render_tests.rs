//! CS3 (observation): the shared stack rendering.
//!
//! Every observation surface (CLI stack display, REPL, in-process conformance
//! runner, JSON report) renders through one function —
//! `crate::types::display::render_stack` — and it renders each slot from its
//! value alone (LANG.VALUES.DENOTATION).

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
async fn arithmetic_result_renders_as_a_number() {
    assert_eq!(render("1 2 ADD").await, vec!["3/1".to_string()]);
}

#[tokio::test]
async fn truth_and_absence_render_canonically() {
    assert_eq!(render("TRUE").await, vec!["TRUE".to_string()]);
    assert_eq!(render("FALSE").await, vec!["FALSE".to_string()]);
    assert_eq!(render("NIL").await, vec!["NIL".to_string()]);
}

/// The same value renders the same way however it was produced: a Boolean
/// from a comparison and one from a `FOLD` of `AND`, a NIL from a literal and
/// one that passed through `MUL`.
#[tokio::test]
async fn rendering_does_not_depend_on_the_producing_word() {
    assert_eq!(
        render("3 2 GT").await,
        render("[ 3 4 ] [ 2 GT ] MAP TRUE [ AND ] FOLD").await
    );
    assert_eq!(render("NIL -1 MUL").await, render("NIL").await);
}
