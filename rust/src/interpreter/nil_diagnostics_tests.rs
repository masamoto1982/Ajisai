//! Tests for the diagnostic absence accessors (SPEC §4.5.0 / §7.15):
//! `NIL?`, `NIL-REASON`, `NIL-ORIGIN`, `NIL-RECOVERABLE?`, `NIL-DIAGNOSIS`.
//!
//! Coverage follows the §15 discipline: success paths, the non-NIL path, the
//! reason-present vs reason-absent split, protocol-string (not Rust `Debug`)
//! output, the U firewall, source retention, and MC/DC over the two governing
//! decisions (`is_operational_nil` and reason `Some`/`None`).

use crate::error::NilReason;
use crate::interpreter::value_extraction_helpers::value_as_string;
use crate::interpreter::Interpreter;

async fn run(code: &str) -> Interpreter {
    let mut interp = Interpreter::new();
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("execute({:?}) failed: {}", code, e));
    interp
}

/// The top-of-stack value, as a protocol string, or `None` when it is NIL.
fn top_text(interp: &Interpreter) -> Option<String> {
    let top = interp.get_stack().last().expect("stack must be non-empty");
    if top.is_nil() {
        return None;
    }
    Some(value_as_string(top).expect("top must be a Text value"))
}

fn top_is_nil(interp: &Interpreter) -> bool {
    interp
        .get_stack()
        .last()
        .map(|v| v.is_nil())
        .unwrap_or(false)
}

fn top_is_true(interp: &Interpreter) -> bool {
    interp
        .get_stack()
        .last()
        .and_then(|v| v.as_truth())
        .unwrap_or(false)
}
// ── NIL? ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn nil_check_is_true_for_operational_nil_and_retains_source() {
    let interp = run("1 0 / NIL?").await;
    let stack = interp.get_stack();
    assert_eq!(stack.len(), 2, "NIL? must retain the inspected value");
    assert!(stack[0].is_nil(), "the inspected NIL is retained below");
    assert!(top_is_true(&interp), "NIL? on an operational NIL is TRUE");
}

#[tokio::test]
async fn nil_check_is_false_for_present_value() {
    let interp = run("5 NIL?").await;
    let stack = interp.get_stack();
    assert_eq!(stack.len(), 2, "NIL? retains the inspected value");
    assert_eq!(
        stack[1].as_truth(),
        Some(false),
        "NIL? on a present value is FALSE"
    );
}

/// The logical Unknown (U) is a NIL read in truth position
/// (LANG.VALUES.TRUTH), so it is an absence and `NIL?` answers TRUE — the
/// same answer `OR-NIL` acts on, and the same story the host protocol
/// tells about it (`type: "nil"`, a published `absence.reason`).
///
/// `TRUE NIL AND` is the strong-Kleene UNKNOWN row (neither operand absorbs
/// the other), so it produces a genuine U — `AND`/`OR`/`NOT` are what makes
/// U reachable from source at all.
#[tokio::test]
async fn nil_check_is_true_for_logical_unknown() {
    let interp = run("TRUE NIL AND NIL?").await;
    assert_eq!(
        interp.get_stack()[1].as_truth(),
        Some(true),
        "NIL? on the logical Unknown must be TRUE: U is an absence"
    );
}

/// A reason survives being read in truth position. `AND` used to swallow it:
/// `1 0 DIV TRUE AND NIL-REASON` answered `notAvailable` while the protocol
/// still published `absence.reason = divisionByZero` for that value, so the
/// language contradicted its own boundary and LANG.VALUES.NIL ("the reason
/// is the entire observable content of a NIL").
#[tokio::test]
async fn nil_reason_survives_a_kleene_word() {
    let interp = run("1 0 DIV TRUE AND NIL-REASON").await;
    assert_eq!(
        top_text(&interp).as_deref(),
        Some("divisionByZero"),
        "an UNKNOWN must keep the reason it arrived with"
    );
}

// ── NIL-REASON ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn nil_reason_reports_division_by_zero_protocol_string() {
    let interp = run("1 0 / NIL-REASON").await;
    let stack = interp.get_stack();
    assert_eq!(stack.len(), 2, "NIL-REASON must retain the inspected value");
    assert!(stack[0].is_nil(), "the inspected NIL is retained below");
    assert_eq!(
        top_text(&interp).as_deref(),
        Some("divisionByZero"),
        "NIL-REASON must be the lowerCamelCase protocol string, not a Debug name"
    );
}

/// The output must be the protocol string, never the Rust `Debug` rendering of
/// the `NilReason` enum (`DivisionByZero`).
#[tokio::test]
async fn nil_reason_is_protocol_string_not_debug_name() {
    let interp = run("1 0 / NIL-REASON").await;
    let text = top_text(&interp).expect("reason must be Text");
    assert_eq!(text, "divisionByZero");
    assert_ne!(text, format!("{:?}", NilReason::DivisionByZero));
}

#[tokio::test]
async fn nil_reason_reports_index_out_of_bounds() {
    let interp = run("[ 1 2 3 ] [ 9 ] GET NIL-REASON").await;
    assert_eq!(top_text(&interp).as_deref(), Some("indexOutOfBounds"));
}

/// A written NIL reads back as `literal`. Every NIL carries a reason now, so
/// the reason-`None` branch of `NIL-REASON` is no longer reachable through a
/// literal; the branch itself is covered by `nil_reason_is_nil_for_present_value`
/// below, where the subject is not an operational NIL at all.
#[tokio::test]
async fn nil_reason_of_a_written_nil_is_literal() {
    let interp = run("NIL NIL-REASON").await;
    assert_eq!(top_text(&interp).as_deref(), Some("literal"));
}

/// The non-NIL path: `NIL-REASON` on a present value yields NIL, not an error.
#[tokio::test]
async fn nil_reason_is_nil_for_present_value() {
    let interp = run("5 NIL-REASON").await;
    assert!(top_is_nil(&interp));
    assert_eq!(interp.get_stack()[0].as_truth(), None);
    assert!(!interp.get_stack()[0].is_nil(), "the 5 is retained below");
}

/// `NIL-REASON` on the result of an exact-arithmetic comparison must yield
/// NIL, never a reason — same as `nil_reason_is_nil_for_present_value`
/// above. This expression was originally written to exercise the logical
/// Unknown (U), but Tier ≤1 exact comparisons are always decidable in
/// finite time (`types/exact/computable.rs`), so `2 SQRT 1 ADD` compared
/// against itself resolves to a definite `TRUE` here, not U — the current
/// vocabulary has no Tier 2 word and so cannot construct U at all. A test
/// that actually exercises the `NIL-REASON` firewall on U will need one.
#[tokio::test]
async fn nil_reason_is_nil_for_a_decidable_exact_comparison() {
    let interp = run("2 SQRT 1 ADD 2 SQRT 1 ADD SUB 0 EQ NIL-REASON").await;
    assert!(
        top_is_nil(&interp),
        "NIL-REASON on a non-NIL value must be NIL, never a reason string"
    );
}
// ── Domain miss (SPEC §5: "SQRT of a negative rational is a well-formed
//    domain miss") ────────────────────────────────────────────────────────────
/// Division by zero keeps its own reason. The domain-miss variant is a new
/// classification, not a rename of an existing one.
#[tokio::test]
async fn division_by_zero_is_untouched_by_the_domain_miss_split() {
    let interp = run("1 0 DIV NIL-REASON").await;
    assert_eq!(top_text(&interp).as_deref(), Some("divisionByZero"));
}
