//! The full strong-Kleene truth tables for `AND`/`OR`/`NOT` (LANG.VALUES.TRUTH),
//! with NIL standing for UNKNOWN. Split out from `nil_conformance_tests` to
//! stay under the §14.1 file-size budget.

use crate::interpreter::Interpreter;
use crate::types::Value;

async fn run_ok(code: &str) -> Vec<Value> {
    let mut interp = Interpreter::new();
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` unexpectedly errored: {e}"));
    interp.get_stack().to_vec()
}

/// FALSE absorbs into `AND` and TRUE absorbs into `OR` even against a NIL
/// operand; only where neither operand is the absorbing value does a NIL
/// operand surface in the result.
#[tokio::test]
async fn strong_kleene_and_or_truth_tables() {
    for (code, expect_nil, expect_bool) in [
        // AND: definite rows.
        ("TRUE TRUE AND", false, Some(true)),
        ("TRUE FALSE AND", false, Some(false)),
        ("FALSE TRUE AND", false, Some(false)),
        ("FALSE FALSE AND", false, Some(false)),
        // AND: FALSE absorbs a NIL operand into FALSE, from either side.
        ("FALSE NIL AND", false, Some(false)),
        ("NIL FALSE AND", false, Some(false)),
        // AND: TRUE does not settle it, so a NIL operand surfaces as UNKNOWN.
        ("TRUE NIL AND", true, None),
        ("NIL TRUE AND", true, None),
        ("NIL NIL AND", true, None),
        // OR: definite rows.
        ("TRUE TRUE OR", false, Some(true)),
        ("TRUE FALSE OR", false, Some(true)),
        ("FALSE TRUE OR", false, Some(true)),
        ("FALSE FALSE OR", false, Some(false)),
        // OR: TRUE absorbs a NIL operand into TRUE, from either side.
        ("TRUE NIL OR", false, Some(true)),
        ("NIL TRUE OR", false, Some(true)),
        // OR: FALSE does not settle it, so a NIL operand surfaces as UNKNOWN.
        ("FALSE NIL OR", true, None),
        ("NIL FALSE OR", true, None),
        ("NIL NIL OR", true, None),
        // NOT: no second operand to absorb into, so NIL stays NIL.
        ("TRUE NOT", false, Some(false)),
        ("FALSE NOT", false, Some(true)),
        ("NIL NOT", true, None),
    ] {
        let stack = run_ok(code).await;
        assert_eq!(stack.len(), 1, "`{code}` must leave exactly one value");
        if expect_nil {
            assert!(stack[0].is_nil(), "`{code}` must produce NIL (UNKNOWN)");
            assert_eq!(
                stack[0].truth_value(),
                Some("unknown"),
                "`{code}`'s NIL result must observe as truthValue `unknown`"
            );
        } else {
            assert_eq!(
                stack[0].as_truth(),
                expect_bool,
                "`{code}` must decide {expect_bool:?}"
            );
        }
    }
}
