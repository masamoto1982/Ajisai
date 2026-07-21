//! CS3 (observation): the shared `(value, role)` stack rendering.
//!
//! SPEC §12 observes each stack slot as a `(data, role)` pair. Every observation
//! surface (CLI stack display, REPL, in-process conformance runner, JSON report)
//! renders through one function — `crate::types::display::render_stack` — so a
//! position role such as `>CF` or a timestamp cannot render one way on one
//! surface and another way on another. These tests pin that shared rendering and
//! prove it uses the *slot* role, not the value's construction-time hint.

use crate::interpreter::Interpreter;
use crate::types::display::render_stack;
use crate::types::{Interpretation, Value};

async fn render(code: &str) -> Vec<String> {
    let mut interp = Interpreter::new();
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` unexpectedly errored: {e}"));
    render_stack(interp.get_stack())
}

#[tokio::test]
async fn tocf_rational_renders_as_continued_fraction() {
    // `>CF` re-tags the slot role to ContinuedFraction; the shared surface
    // renders the canonical nested CF, identical to the CLI stack display.
    assert_eq!(render("1/3 >CF").await, vec!["( 0 ( 3 ) )".to_string()]);
}

#[tokio::test]
async fn arithmetic_result_keeps_raw_number_role() {
    assert_eq!(render("1 2 ADD").await, vec!["3/1".to_string()]);
}

#[tokio::test]
async fn truth_and_absence_roles_render_canonically() {
    assert_eq!(render("TRUE").await, vec!["TRUE".to_string()]);
    assert_eq!(render("FALSE").await, vec!["FALSE".to_string()]);
    assert_eq!(render("NIL").await, vec!["NIL".to_string()]);
}

#[tokio::test]
async fn render_uses_slot_role_not_value_hint() {
    // A slot whose role was cast away from the value's construction hint must
    // render via the slot role. Push a rational (hint RawNumber) under an
    // explicit ContinuedFraction slot role and confirm the CF rendering — the
    // value's own `Display` (which uses its hint) would instead show `1/3`.
    let mut interp = Interpreter::new();
    let one_third = Value::from_number(crate::types::fraction::Fraction::new(
        num_bigint::BigInt::from(1),
        num_bigint::BigInt::from(3),
    ));
    assert_eq!(format!("{one_third}"), "1/3");
    interp
        .stack
        .push_with_role(one_third, Interpretation::ContinuedFraction);
    assert_eq!(
        render_stack(interp.get_stack()),
        vec!["( 0 ( 3 ) )".to_string()]
    );
}
