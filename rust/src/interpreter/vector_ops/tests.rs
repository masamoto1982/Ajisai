//! Test suite for `crate::interpreter::vector_ops`.

use crate::interpreter::Interpreter;

#[tokio::test]
async fn test_range_basic_stacktop() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 5 ] RANGE").await;
    assert!(result.is_ok(), "RANGE should succeed: {:?}", result);

    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_range_with_step() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 10 2 ] RANGE").await;
    assert!(
        result.is_ok(),
        "RANGE with step should succeed: {:?}",
        result
    );

    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_range_descending() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 10 0 -2 ] RANGE").await;
    assert!(
        result.is_ok(),
        "RANGE descending should succeed: {:?}",
        result
    );
    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_range_single_element() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 5 5 ] RANGE").await;
    assert!(
        result.is_ok(),
        "RANGE single element should succeed: {:?}",
        result
    );
    assert_eq!(interp.stack.len(), 1);
}

#[tokio::test]
async fn test_range_error_step_zero_restores_stack_stacktop() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 10 0 ] RANGE").await;
    assert!(result.is_err(), "RANGE with step=0 should fail");

    assert_eq!(
        interp.stack.len(),
        1,
        "Arguments should be restored on error"
    );
}

#[tokio::test]
async fn test_range_error_step_zero_restores_stack_stack_mode() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 10 0 ] .. RANGE").await;
    assert!(result.is_err(), "RANGE stack mode with step=0 should fail");

    assert_eq!(
        interp.stack.len(),
        1,
        "Arguments should be restored on error in stack mode"
    );
}

#[tokio::test]
async fn test_range_error_infinite_restores_stack() {
    let mut interp = Interpreter::new();

    let result = interp.execute("[ 0 10 -1 ] RANGE").await;
    assert!(result.is_err(), "RANGE with infinite sequence should fail");

    assert_eq!(
        interp.stack.len(),
        1,
        "Arguments should be restored on infinite error"
    );
}

// ── RANGE builds columns, and they are the same numbers ─────────────────────
//
// The tests above assert that RANGE succeeds and leaves one value; none of them
// look at what that value holds, so they passed whatever the lanes were. That
// mattered when RANGE stopped boxing per-lane `Value`s and started writing `i64`
// columns: the construction changed, and nothing in the suite could have noticed
// if it had changed the numbers with it.

/// Whether the top of the stack is a dense (SoA) tensor.
///
/// The representation is the point of the change, so it is stated outright: a
/// RANGE result is a 1-D pure-integer dense tensor *by construction* —
/// `parse_range_args` answers in `i64`, so no lane can be rational, irrational
/// or absent. Building `Vec<Value>` instead threw that away at the source, and
/// every Word downstream then had to decline its dense fast path.
async fn range_lands_a_dense_tensor(code: &str) -> bool {
    let mut interp = Interpreter::new();
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` must compute, got: {e:?}"));
    matches!(
        interp.get_stack().as_slice().last().map(|v| &v.data),
        Some(crate::types::ValueData::Tensor { .. })
    )
}

/// `EQ` against the literal spelling of the same sequence. This is the gate
/// that matters: a dense tensor and the nested vector of the same numbers are
/// one value (`value_identity` grants them cross-representation equality), so
/// if the columns hold anything other than what the literal says, the language
/// itself reports it.
async fn range_equals_literal(range: &str, literal: &str) -> Option<bool> {
    let code = format!("{range} {literal} EQ");
    let mut interp = Interpreter::new();
    interp
        .execute(&code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` must compute, got: {e:?}"));
    interp.get_stack().as_slice().last()?.as_truth()
}

#[tokio::test]
async fn range_results_are_dense_tensors() {
    for code in [
        "[ 0 5 ] RANGE",
        "[ 0 10 2 ] RANGE",
        "[ 10 0 -2 ] RANGE",
        "[ 5 5 ] RANGE",
        "[ -3 3 ] RANGE",
    ] {
        assert!(
            range_lands_a_dense_tensor(code).await,
            "`{code}` must build i64 columns, not boxed per-lane Values"
        );
    }
}

#[tokio::test]
async fn range_lanes_are_the_numbers_the_literal_spells() {
    // Every shape the argument parser admits: the default ascending step, an
    // explicit step that does not divide the span evenly, a descending step,
    // a single-element range, and a span crossing zero.
    for (range, literal) in [
        ("[ 0 5 ] RANGE", "[ 0 1 2 3 4 5 ]"),
        ("[ 0 10 2 ] RANGE", "[ 0 2 4 6 8 10 ]"),
        ("[ 0 9 2 ] RANGE", "[ 0 2 4 6 8 ]"),
        ("[ 10 0 -2 ] RANGE", "[ 10 8 6 4 2 0 ]"),
        ("[ 10 1 -3 ] RANGE", "[ 10 7 4 1 ]"),
        ("[ 5 5 ] RANGE", "[ 5 ]"),
        ("[ -3 3 ] RANGE", "[ -3 -2 -1 0 1 2 3 ]"),
        ("[ 3 -3 ] RANGE", "[ 3 2 1 0 -1 -2 -3 ]"),
    ] {
        assert_eq!(
            range_equals_literal(range, literal).await,
            Some(true),
            "`{range}` must hold exactly {literal}"
        );
    }
}

/// The count the space-ceiling guard computes must be the count the loop
/// writes, or the guard is bounding a different number than the one allocated.
/// `LENGTH` is how the language asks.
#[tokio::test]
async fn range_length_matches_the_counted_span() {
    for (code, expected) in [
        ("[ 0 5 ] RANGE LENGTH", 6),
        ("[ 0 10 2 ] RANGE LENGTH", 6),
        ("[ 0 9 2 ] RANGE LENGTH", 5),
        ("[ 10 1 -3 ] RANGE LENGTH", 4),
        ("[ 5 5 ] RANGE LENGTH", 1),
        ("[ -3 3 ] RANGE LENGTH", 7),
    ] {
        let mut interp = Interpreter::new();
        interp
            .execute(code)
            .await
            .unwrap_or_else(|e| panic!("`{code}` must compute, got: {e:?}"));
        let length = interp
            .get_stack()
            .as_slice()
            .last()
            .and_then(|v| v.as_scalar().and_then(|f| f.to_i64()))
            .unwrap_or_else(|| panic!("`{code}` must leave an integer length"));
        assert_eq!(length, expected, "`{code}` must count {expected} lanes");
    }
}

// ── REVERSE keeps a flat dense buffer dense ─────────────────────────────────

/// `EQ` between two programs. Cross-representation equality means a dense
/// result and the nested literal of the same numbers are one value, so this
/// asks the language whether the reversal produced what it should have.
async fn programs_are_equal(left: &str, right: &str) -> Option<bool> {
    let code = format!("{left} {right} EQ");
    let mut interp = Interpreter::new();
    interp
        .execute(&code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` must compute, got: {e:?}"));
    interp.get_stack().as_slice().last()?.as_truth()
}

#[tokio::test]
async fn reverse_keeps_a_flat_dense_result_dense() {
    // The representation is the point: reversing used to unpack the tensor into
    // boxed per-lane `Value`s and hand back an AoS `Vector`, which left every
    // Word downstream to decline its own dense path.
    assert!(
        range_lands_a_dense_tensor("[ 0 7 ] RANGE REVERSE").await,
        "a flat dense REVERSE must stay dense"
    );
    assert!(
        range_lands_a_dense_tensor("[ 1 2 3 4 5 6 7 8 ] REVERSE").await,
        "a dense literal's REVERSE must stay dense"
    );
}

#[tokio::test]
async fn reverse_produces_the_reversed_sequence() {
    for (reversed, literal) in [
        ("[ 1 2 3 4 5 6 7 8 ] REVERSE", "[ 8 7 6 5 4 3 2 1 ]"),
        ("[ 0 5 ] RANGE REVERSE", "[ 5 4 3 2 1 0 ]"),
        ("[ 5 ] REVERSE", "[ 5 ]"),
        // Rational lanes: the columns reverse together, so 1/3 2/3 1 4/3
        // becomes 4/3 1 2/3 1/3 rather than pairing a numerator with the
        // wrong denominator.
        (
            "[ 1 2 3 4 ] [ 3 DIV ] MAP REVERSE",
            "[ 4 3 2 1 ] [ 3 DIV ] MAP",
        ),
    ] {
        assert_eq!(
            programs_are_equal(reversed, literal).await,
            Some(true),
            "`{reversed}` must equal `{literal}`"
        );
    }
}

#[tokio::test]
async fn reverse_twice_restores_the_original() {
    for source in [
        "[ 1 2 3 4 5 6 7 8 ]",
        "[ 0 20 3 ] RANGE",
        "[ 1 NIL 3 4 5 6 7 8 ]",
        "[ [ 1 2 ] [ 3 4 ] [ 5 6 ] ]",
        "[ 'a' 'b' 'c' ]",
    ] {
        assert_eq!(
            programs_are_equal(&format!("{source} REVERSE REVERSE"), source).await,
            Some(true),
            "`{source} REVERSE REVERSE` must restore `{source}`"
        );
    }
}

/// An absent lane lands at `len - 1 - index`, and the lanes around it do not
/// become absent. Read through the language, so the dense remap is checked
/// where a program would see it.
#[tokio::test]
async fn reverse_moves_an_absent_lane_to_its_mirrored_index() {
    // `[ 1 NIL 3 4 5 6 7 8 ]` densifies with lane 1 absent; reversed, index 6.
    for (code, expect_nil) in [
        ("[ 1 NIL 3 4 5 6 7 8 ] REVERSE 6 GET NIL?", true),
        ("[ 1 NIL 3 4 5 6 7 8 ] REVERSE 1 GET NIL?", false),
        ("[ 1 NIL 3 4 5 6 7 8 ] REVERSE 7 GET NIL?", false),
    ] {
        let mut interp = Interpreter::new();
        interp
            .execute(code)
            .await
            .unwrap_or_else(|e| panic!("`{code}` must compute, got: {e:?}"));
        assert_eq!(
            interp
                .get_stack()
                .as_slice()
                .last()
                .and_then(|v| v.as_truth()),
            Some(expect_nil),
            "`{code}` must answer {expect_nil}"
        );
    }
}

/// Rank above 1 keeps the nested route, because reversing a rank-2 tensor
/// reverses its *rows* and a row is a stride rather than a lane.
#[tokio::test]
async fn reverse_of_a_rank_two_value_reverses_rows() {
    assert_eq!(
        programs_are_equal(
            "[ [ 1 2 ] [ 3 4 ] [ 5 6 ] ] REVERSE",
            "[ [ 5 6 ] [ 3 4 ] [ 1 2 ] ]"
        )
        .await,
        Some(true),
        "a rank-2 REVERSE must reverse rows, not lanes"
    );
}
