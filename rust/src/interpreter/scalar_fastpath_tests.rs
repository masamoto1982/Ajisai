//! Differential coverage for the D1 scalar-scalar arithmetic/comparison fast
//! path. The fast path is observational only: with it enabled or disabled, the
//! stack values, rendered forms, and per-value hints must be identical.

use crate::interpreter::{arithmetic, comparison, Interpreter};
use crate::types::Value;

fn block_on<F: std::future::Future>(fut: F) -> F::Output {
    use std::task::{Context, Poll};
    let mut fut = Box::pin(fut);
    let waker = std::task::Waker::noop();
    let mut cx = Context::from_waker(waker);
    loop {
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn run(src: &str, enabled: bool) -> Interpreter {
    let mut interp = Interpreter::new();
    interp.set_scalar_fastpath_enabled(enabled);
    block_on(interp.execute(src)).unwrap();
    interp
}

fn run_direct(
    stack: Vec<Value>,
    enabled: bool,
    op: fn(&mut Interpreter) -> crate::error::Result<()>,
) -> Interpreter {
    let mut interp = Interpreter::new();
    interp.set_scalar_fastpath_enabled(enabled);
    interp.update_stack(stack);
    op(&mut interp).unwrap();
    interp
}

fn singleton_vector(n: i64) -> Value {
    Value::from_vector(vec![Value::from_int(n)])
}

fn one_char_string(n: i64) -> Value {
    Value::from_string(
        &char::from_u32(n as u32)
            .expect("test codepoint")
            .to_string(),
    )
}

fn rendered_stack(interp: &Interpreter) -> Vec<String> {
    interp
        .get_stack()
        .iter()
        .map(|value| format!("{value}"))
        .collect()
}

fn hint_stack(interp: &Interpreter) -> Vec<String> {
    interp
        .get_stack()
        .iter()
        .map(|value| format!("{:?}", value.hint))
        .collect()
}

fn assert_on_equals_off(src: &str) -> (Interpreter, Interpreter) {
    let on = run(src, true);
    let off = run(src, false);
    assert_eq!(
        format!("{:?}", on.get_stack()),
        format!("{:?}", off.get_stack()),
        "fast path ON vs OFF stack diverged for: {src}"
    );
    assert_eq!(
        rendered_stack(&on),
        rendered_stack(&off),
        "fast path ON vs OFF render diverged for: {src}"
    );
    assert_eq!(
        hint_stack(&on),
        hint_stack(&off),
        "fast path ON vs OFF hints diverged for: {src}"
    );
    (on, off)
}

fn assert_direct_on_equals_off(
    stack: Vec<Value>,
    op: fn(&mut Interpreter) -> crate::error::Result<()>,
) -> (Interpreter, Interpreter) {
    let on = run_direct(stack.clone(), true, op);
    let off = run_direct(stack, false, op);
    assert_eq!(
        format!("{:?}", on.get_stack()),
        format!("{:?}", off.get_stack()),
        "direct fast path ON vs OFF stack diverged"
    );
    assert_eq!(
        rendered_stack(&on),
        rendered_stack(&off),
        "direct fast path ON vs OFF render diverged"
    );
    assert_eq!(
        hint_stack(&on),
        hint_stack(&off),
        "direct fast path ON vs OFF hints diverged"
    );
    (on, off)
}

#[test]
fn arithmetic_fast_path_matches_baseline_for_bare_scalars_and_singleton_tensors() {
    for src in [
        "2 3 +",
        "7 4 -",
        "6 5 *",
        "6 4 /",
        "[ 1 ] [ 2 ] +",
        "[ 7 ] [ 4 ] -",
        "[ 6 ] [ 5 ] *",
        "[ 6 ] [ 4 ] /",
    ] {
        let (on, off) = assert_on_equals_off(src);
        assert!(
            on.runtime_metrics().scalar_fastpath_count >= 1,
            "expected scalar fast path to fire for: {src}"
        );
        assert_eq!(
            off.runtime_metrics().scalar_fastpath_count,
            0,
            "disabled scalar fast path should not count for: {src}"
        );
    }
}

#[test]
fn fast_path_preserves_tensor_wrapping() {
    let (on, _) = assert_on_equals_off("[ 1 ] [ 2 ] +");
    let rendered = rendered_stack(&on);
    assert_eq!(rendered, vec!["[ 3/1 ]"]);
}

#[test]
fn unsupported_or_semantically_sensitive_shapes_fall_back() {
    for src in ["2 [ 3 ] +", "[ 2 ] 3 +", "NIL 3 +", "3 NIL >"] {
        let (on, off) = assert_on_equals_off(src);
        assert_eq!(
            on.runtime_metrics().scalar_fastpath_count,
            0,
            "fast path should fall back for: {src}"
        );
        assert_eq!(off.runtime_metrics().scalar_fastpath_count, 0);
    }
}

#[test]
fn keep_mode_fast_path_preserves_operands_and_pushes_result() {
    // The shaped comparison forms (`[ 3 ] [ 4 ] KEEP >`) moved out of this
    // list with the singleton fast path; they lift now.
    for src in [
        "3 4 KEEP ADD",
        "[ 3 ] [ 4 ] KEEP ADD",
        "3 4 KEEP >",
        "3 3 KEEP =",
    ] {
        let (on, off) = assert_on_equals_off(src);
        assert!(
            on.runtime_metrics().scalar_fastpath_count >= 1,
            "expected KEEP scalar fast path to fire for: {src}"
        );
        assert_eq!(
            off.runtime_metrics().scalar_fastpath_count,
            0,
            "disabled scalar fast path should not count for: {src}"
        );
        assert_eq!(
            on.get_stack().len(),
            3,
            "KEEP fast path must retain both operands and push one result for: {src}"
        );
    }
}

#[test]
fn direct_singleton_vector_fast_path_matches_baseline() {
    // Arithmetic only. The comparison family no longer has a singleton fast
    // path: a one-element Vector is not its element (LANG.VALUES.DISJOINT), so
    // `[ 6 ] [ 3 ] LT` lifts element-wise to `[ FALSE ]` rather than deciding
    // as a scalar. See `comparison_on_singletons_lifts_instead_of_fast_pathing`.
    for op in [
        arithmetic::op_add as fn(&mut Interpreter) -> crate::error::Result<()>,
        arithmetic::op_sub,
        arithmetic::op_mul,
        arithmetic::op_div,
    ] {
        let (on, off) =
            assert_direct_on_equals_off(vec![singleton_vector(6), singleton_vector(3)], op);
        assert!(
            on.runtime_metrics().scalar_fastpath_count >= 1,
            "expected singleton Vector fast path to fire"
        );
        assert_eq!(off.runtime_metrics().scalar_fastpath_count, 0);
    }
}

#[test]
fn string_operand_stays_on_baseline_path() {
    // A String has no numeric lane at all now, so the fast path cannot see a
    // singleton to project — and arithmetic rejects it outright rather than
    // adding 1 to a codepoint. It used to be excluded by a hint check, and
    // `'A' 1 ADD` used to answer `[ 66/1 ]`.
    for enabled in [true, false] {
        let mut interp = Interpreter::new();
        interp.set_scalar_fastpath_enabled(enabled);
        interp.update_stack(vec![one_char_string(65), one_char_string(66)]);
        assert!(
            arithmetic::op_add(&mut interp).is_err(),
            "ADD must reject String operands (fast path {enabled})"
        );
        assert_eq!(
            interp.runtime_metrics().scalar_fastpath_count,
            0,
            "a String must not take the numeric scalar fast path"
        );
    }
}

#[test]
fn division_by_zero_matches_baseline() {
    let (on, _) = assert_on_equals_off("6 0 /");
    assert!(
        on.runtime_metrics().scalar_fastpath_count >= 1,
        "division by zero still uses the scalar fast path to produce the same bubble"
    );
}

#[test]
fn comparison_on_singletons_lifts_instead_of_fast_pathing() {
    // The fast path used to project a one-element Vector to its element and
    // answer a bare Boolean, so `[ 6 ] [ 3 ] LT` was `FALSE` — a collapse
    // LANG.COLLECTIONS.LIFT forbids. It now lifts, which is a shape the fast
    // path deliberately declines.
    for enabled in [true, false] {
        let mut interp = Interpreter::new();
        interp.set_scalar_fastpath_enabled(enabled);
        interp.update_stack(vec![singleton_vector(6), singleton_vector(3)]);
        comparison::op_lt(&mut interp).expect("LT lifts over singletons");
        assert_eq!(
            interp.runtime_metrics().scalar_fastpath_count,
            0,
            "a shaped operand must not take the scalar fast path (enabled={enabled})"
        );
        let result = interp.get_stack().last().expect("a result").clone();
        assert_eq!(result.len(), 1, "the lift preserves the operand shape");
        assert_eq!(
            result.child(0).and_then(|c| c.as_truth()),
            Some(false),
            "6 < 3 is false in the single lane"
        );
    }
}
