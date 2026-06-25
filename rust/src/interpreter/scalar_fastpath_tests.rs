//! Differential coverage for the D1 scalar-scalar arithmetic/comparison fast
//! path. The fast path is observational only: with it enabled or disabled, the
//! stack values, rendered forms, and per-value hints must be identical.

use crate::interpreter::{arithmetic, comparison, Interpreter};
use crate::types::{Interpretation, Value};

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

fn text_singleton_vector(n: i64) -> Value {
    Value::from_vector_with_hint(vec![Value::from_int(n)], Interpretation::Text)
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
fn comparison_fast_path_matches_baseline_for_bare_scalars_and_singleton_tensors() {
    for src in [
        "2 3 <",
        "3 3 <=",
        "4 3 >",
        "4 4 >=",
        "4 4 =",
        "4 5 !=",
        "[ 2 ] [ 3 ] <",
        "[ 3 ] [ 3 ] <=",
        "[ 4 ] [ 3 ] >",
        "[ 4 ] [ 4 ] >=",
        "[ 4 ] [ 4 ] =",
        "[ 4 ] [ 5 ] !=",
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
    for src in [
        "3 4 KEEP ADD",
        "[ 3 ] [ 4 ] KEEP ADD",
        "3 4 KEEP >",
        "[ 3 ] [ 4 ] KEEP >",
        "3 3 KEEP =",
        "[ 3 ] [ 3 ] KEEP =",
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
    for op in [
        arithmetic::op_add as fn(&mut Interpreter) -> crate::error::Result<()>,
        arithmetic::op_sub,
        arithmetic::op_mul,
        arithmetic::op_div,
        comparison::op_lt,
        comparison::op_gt,
        comparison::op_eq,
        comparison::op_neq,
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
fn text_singleton_vector_stays_on_baseline_path() {
    let (on, off) = assert_direct_on_equals_off(
        vec![text_singleton_vector(6), text_singleton_vector(3)],
        arithmetic::op_add,
    );
    assert_eq!(
        on.runtime_metrics().scalar_fastpath_count,
        0,
        "Text-hinted singleton Vector must not take numeric scalar fast path"
    );
    assert_eq!(off.runtime_metrics().scalar_fastpath_count, 0);
}

#[test]
fn division_by_zero_matches_baseline() {
    let (on, _) = assert_on_equals_off("6 0 /");
    assert!(
        on.runtime_metrics().scalar_fastpath_count >= 1,
        "division by zero still uses the scalar fast path to produce the same bubble"
    );
}
