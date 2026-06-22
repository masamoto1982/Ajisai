//! Phase C: end-to-end verification of the real WASM serialization boundary.
//!
//! Phases A and B verify the (Value, hint) -> protocol mapping and the
//! interpreter's NIL/Bubble behavior natively, on the host target -- those
//! never cross the `wasm-bindgen` glue. This crate closes the last gap: it
//! drives the public `AjisaiInterpreter` API compiled to `wasm32`, executes
//! code, and reads back the actual `JsValue` the GUI receives from
//! `collect_stack`, so the wasm-bindgen codegen and `js_sys::Reflect`
//! plumbing are exercised for real, not merely compile-checked.
//!
//! Run: `cd rust/wasm-tests && wasm-pack test --node`.
//!
//! Trace: docs/quality/TRACEABILITY_MATRIX.md (AQ-REQ-003, WASM boundary).
#![cfg(target_arch = "wasm32")]

use ajisai_core::AjisaiInterpreter;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

fn field(obj: &JsValue, key: &str) -> JsValue {
    js_sys::Reflect::get(obj, &JsValue::from_str(key)).expect("field present")
}

fn type_of(node: &JsValue) -> String {
    field(node, "type").as_string().expect("type is a string")
}

fn children(node: &JsValue) -> js_sys::Array {
    js_sys::Array::from(&field(node, "value"))
}

async fn stack_of(code: &str) -> js_sys::Array {
    let mut interp = AjisaiInterpreter::new();
    interp.execute(code).await.expect("execution succeeds");
    js_sys::Array::from(&interp.collect_stack())
}

/// Regression for #972 at the real boundary: `[ TRUE ]` is a promoted dense
/// boolean tensor; its element must cross the wasm boundary as a boolean,
/// not a `1/1` number.
#[wasm_bindgen_test]
async fn boolean_vector_serializes_as_booleans() {
    let stack = stack_of("[ TRUE ]").await;
    assert_eq!(stack.length(), 1);
    let node = stack.get(0);
    assert_eq!(type_of(&node), "vector");
    let kids = children(&node);
    assert_eq!(kids.length(), 1);
    let kid = kids.get(0);
    assert_eq!(type_of(&kid), "boolean");
    assert_eq!(field(&kid, "value").as_bool(), Some(true));
}

#[wasm_bindgen_test]
async fn false_vector_serializes_as_boolean_false() {
    let stack = stack_of("[ FALSE ]").await;
    let kid = children(&stack.get(0)).get(0);
    assert_eq!(type_of(&kid), "boolean");
    assert_eq!(field(&kid, "value").as_bool(), Some(false));
}

#[wasm_bindgen_test]
async fn number_vector_serializes_as_numbers() {
    let stack = stack_of("[ 1 2 3 ]").await;
    let node = stack.get(0);
    assert_eq!(type_of(&node), "vector");
    let kids = children(&node);
    assert_eq!(kids.length(), 3);
    for i in 0..3 {
        let kid = kids.get(i);
        assert_eq!(type_of(&kid), "number");
        let value = field(&kid, "value");
        assert!(field(&value, "numerator").as_string().is_some());
        assert!(field(&value, "denominator").as_string().is_some());
    }
}

/// A scalar comparison result crosses the boundary as a top-level boolean
/// (TruthValue role on a bare scalar), distinct from the vector case above.
#[wasm_bindgen_test]
async fn scalar_comparison_serializes_as_boolean() {
    let stack = stack_of("3 5 <").await;
    assert_eq!(stack.length(), 1);
    let node = stack.get(0);
    assert_eq!(type_of(&node), "boolean");
    assert_eq!(field(&node, "value").as_bool(), Some(true));
}

/// A Bubble/NIL (division by zero) crosses the boundary as a `nil`-typed node.
#[wasm_bindgen_test]
async fn bubble_nil_serializes_as_nil() {
    let stack = stack_of("1 0 DIV").await;
    assert_eq!(stack.length(), 1);
    assert_eq!(type_of(&stack.get(0)), "nil");
}

/// An ExactScalar (√2) under the default `RawNumber` role crosses the boundary
/// as a `number` (its best rational approximation) carrying an explicit
/// `semantics.approximate === true` marker, so the GUI never mistakes the
/// approximation for an exact rational (SPEC §2.3 firewall; P1).
#[wasm_bindgen_test]
async fn exact_scalar_rawnumber_marks_approximate_at_boundary() {
    let stack = stack_of("'math' IMPORT 2 SQRT").await;
    assert_eq!(stack.length(), 1);
    let node = stack.get(0);
    assert_eq!(
        type_of(&node),
        "number",
        "√2 (RawNumber) serializes as number"
    );
    // The numeric value is present (the rational approximation).
    let value = field(&node, "value");
    assert!(field(&value, "numerator").as_string().is_some());
    assert!(field(&value, "denominator").as_string().is_some());
    // The approximation marker rides on the semantics metadata bag.
    let semantics = field(&node, "semantics");
    assert_eq!(
        field(&semantics, "approximate").as_bool(),
        Some(true),
        "ExactScalar rendered lossily must be marked approximate"
    );
}

/// A genuinely exact rational (not an ExactScalar) is NOT marked approximate:
/// the marker is specific to exact irrationals collapsed to an approximation.
#[wasm_bindgen_test]
async fn exact_rational_is_not_marked_approximate() {
    let stack = stack_of("3 4 /").await;
    assert_eq!(stack.length(), 1);
    let node = stack.get(0);
    assert_eq!(type_of(&node), "number");
    let semantics = field(&node, "semantics");
    // `approximate` is absent (undefined) for exact rationals.
    assert!(
        field(&semantics, "approximate").as_bool().is_none(),
        "exact rational must not carry an approximate marker"
    );
}

// ---------------------------------------------------------------------------
// Inbound boundary: restore_stack with untrusted / malformed values.
//
// `restore_stack` feeds each element through `js_value_to_value`. A restored
// snapshot is untrusted (it can be tampered with in IndexedDB or arrive across
// the worker boundary), so a malformed value must surface a recoverable error
// rather than panic the module to an unrecoverable trap. Two such inputs used
// to abort: a number whose denominator is "0" (panicked in `Fraction::new`),
// and a deeply nested vector (overflowed the stack via unbounded recursion).
// ---------------------------------------------------------------------------

fn set_field(obj: &js_sys::Object, key: &str, val: &JsValue) {
    js_sys::Reflect::set(obj, &JsValue::from_str(key), val).expect("set field");
}

fn number_node(numerator: &str, denominator: &str) -> JsValue {
    let frac = js_sys::Object::new();
    set_field(&frac, "numerator", &JsValue::from_str(numerator));
    set_field(&frac, "denominator", &JsValue::from_str(denominator));
    let node = js_sys::Object::new();
    set_field(&node, "type", &JsValue::from_str("number"));
    set_field(&node, "value", &frac.into());
    node.into()
}

fn single_stack(node: JsValue) -> JsValue {
    let arr = js_sys::Array::new();
    arr.push(&node);
    arr.into()
}

#[wasm_bindgen_test]
fn restore_stack_rejects_zero_denominator_without_panicking() {
    let mut interp = AjisaiInterpreter::new();
    let result = interp.restore_stack(single_stack(number_node("1", "0")));
    assert!(
        result.is_err(),
        "a zero-denominator value must be a recoverable error, not a Fraction::new panic"
    );
}

#[wasm_bindgen_test]
fn restore_stack_accepts_valid_rational() {
    let mut interp = AjisaiInterpreter::new();
    let result = interp.restore_stack(single_stack(number_node("3", "4")));
    assert!(result.is_ok(), "a valid rational must restore successfully");
}

#[wasm_bindgen_test]
fn restore_stack_rejects_deeply_nested_vector_without_overflow() {
    // Wrap a scalar in `{type:'vector', value:[ ... ]}` far beyond the depth
    // cap; deserializing this used to recurse until the WASM stack overflowed.
    let mut node = number_node("1", "1");
    for _ in 0..1000 {
        let arr = js_sys::Array::new();
        arr.push(&node);
        let vector = js_sys::Object::new();
        set_field(&vector, "type", &JsValue::from_str("vector"));
        set_field(&vector, "value", &arr.into());
        node = vector.into();
    }
    let mut interp = AjisaiInterpreter::new();
    let result = interp.restore_stack(single_stack(node));
    assert!(
        result.is_err(),
        "deeply nested restored value must error, not overflow the stack"
    );
}
