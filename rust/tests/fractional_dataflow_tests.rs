// Fractional Dataflow tests
//
// Verifies the new consumed/remainder semantics introduced in the v1.0.0-draft
// specification (SPECIFICATION.md §0).
//
// Required test categories:
//   1. Conservation law (保存則成立テスト)
//   2. Over-consumption error (過剰消費エラーテスト)
//   3. Remainder inheritance chain (残余継承チェーンテスト)
//   4. Complete consumption — terminal remainder zero (完全消費テスト)

use ajisai_core::interpreter::Interpreter;
use ajisai_core::types::fraction::Fraction;
use ajisai_core::types::{FlowToken, Value, ValueData};
use num_bigint::BigInt;
use num_traits::One;

// ──────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────

async fn run(code: &str) -> Result<Vec<Value>, String> {
    let mut interp = Interpreter::new();
    interp.gui_mode = true;
    interp.execute(code).await.map_err(|e| e.to_string())?;
    Ok(interp.get_stack().clone())
}

async fn run_with_flow_tracking(code: &str) -> Result<(Vec<Value>, Interpreter), String> {
    let mut interp = Interpreter::new();
    interp.gui_mode = true;
    interp.set_flow_tracking(true);
    interp.execute(code).await.map_err(|e| e.to_string())?;
    let stack = interp.get_stack().clone();
    Ok((stack, interp))
}

fn frac(n: i64, d: i64) -> Fraction {
    Fraction::new(BigInt::from(n), BigInt::from(d))
}

fn assert_number(val: &Value, num: i64, denom: i64) {
    let expected = Fraction::new(BigInt::from(num), BigInt::from(denom));
    // In gui_mode, scalars may be wrapped as single-element vectors
    if let Some(f) = val.as_scalar() {
        assert_eq!(f, &expected, "Expected {}/{}, got {}", num, denom, f);
    } else if let Some(vec) = val.as_vector() {
        if vec.len() == 1 {
            let f = vec[0]
                .as_scalar()
                .unwrap_or_else(|| panic!("Expected scalar in vector, got {:?}", vec[0]));
            assert_eq!(f, &expected, "Expected {}/{}, got {}", num, denom, f);
        } else {
            panic!("Expected single scalar, got vector of length {}: {:?}", vec.len(), val);
        }
    } else {
        panic!("Expected scalar, got {:?}", val);
    }
}

// ──────────────────────────────────────────────
// 1. Conservation law tests (保存則成立テスト)
// ──────────────────────────────────────────────

#[tokio::test]
async fn test_conservation_flow_token_basic() {
    // Create a flow token from a scalar value and verify conservation holds
    let val = Value::from_fraction(frac(10, 1));
    let token = FlowToken::from_value(&val);

    assert_eq!(token.total, frac(10, 1));
    assert_eq!(token.remaining, frac(10, 1));
    assert!(!token.is_exhausted());

    // Consume 3 units
    let (consumed, token2) = token.consume(&frac(3, 1)).unwrap();
    assert_eq!(consumed, frac(3, 1));
    assert_eq!(token2.remaining, frac(7, 1));

    // Verify conservation: total == sum(consumed) + remaining
    token2.verify_conservation(&[frac(3, 1)]).unwrap();
}

#[tokio::test]
async fn test_conservation_vector_total() {
    // A vector [3, 5, 2] has total = 3 + 5 + 2 = 10
    let val = Value::from_children(vec![
        Value::from_fraction(frac(3, 1)),
        Value::from_fraction(frac(5, 1)),
        Value::from_fraction(frac(2, 1)),
    ]);
    let token = FlowToken::from_value(&val);
    assert_eq!(token.total, frac(10, 1));
}

#[tokio::test]
async fn test_conservation_negative_values() {
    // For mixed-sign vectors, total uses absolute values: |(-3)| + |5| = 8
    let val = Value::from_children(vec![
        Value::from_fraction(frac(-3, 1)),
        Value::from_fraction(frac(5, 1)),
    ]);
    let token = FlowToken::from_value(&val);
    assert_eq!(token.total, frac(8, 1));
}

#[tokio::test]
async fn test_conservation_fractional_values() {
    // 1/3 + 2/3 = 1
    let val = Value::from_children(vec![
        Value::from_fraction(frac(1, 3)),
        Value::from_fraction(frac(2, 3)),
    ]);
    let token = FlowToken::from_value(&val);
    assert_eq!(token.total, frac(1, 1));
}

#[tokio::test]
async fn test_conservation_multi_step() {
    // Consume in multiple steps and verify conservation at each point
    let val = Value::from_fraction(frac(100, 1));
    let token = FlowToken::from_value(&val);

    let (_, t1) = token.consume(&frac(30, 1)).unwrap();
    t1.verify_conservation(&[frac(30, 1)]).unwrap();

    let (_, t2) = t1.consume(&frac(25, 1)).unwrap();
    t2.verify_conservation(&[frac(30, 1), frac(25, 1)]).unwrap();

    let (_, t3) = t2.consume(&frac(45, 1)).unwrap();
    t3.verify_conservation(&[frac(30, 1), frac(25, 1), frac(45, 1)])
        .unwrap();

    // After consuming exactly 100, remainder should be 0
    assert!(t3.is_exhausted());
}

#[tokio::test]
async fn test_conservation_with_interpreter_tracking() {
    // Run a simple arithmetic pipeline with flow tracking enabled
    let (stack, interp) = run_with_flow_tracking("[ 5 ] [ 3 ] ! +").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_number(&stack[0], 8, 1);

    // The interpreter should have tracked flows without conservation violations
    assert!(interp.verify_all_flows().is_ok());
}

// ──────────────────────────────────────────────
// 2. Over-consumption error tests (過剰消費エラーテスト)
// ──────────────────────────────────────────────

#[tokio::test]
async fn test_over_consumption_error() {
    let val = Value::from_fraction(frac(5, 1));
    let token = FlowToken::from_value(&val);

    // Try to consume more than available
    let result = token.consume(&frac(10, 1));
    assert!(result.is_err(), "Should fail when consuming more than available");
}

#[tokio::test]
async fn test_over_consumption_error_type() {
    let val = Value::from_fraction(frac(5, 1));
    let token = FlowToken::from_value(&val);

    let err = token.consume(&frac(10, 1)).unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("Over-consumption"),
        "Expected OverConsumption error, got: {}",
        msg
    );
}

#[tokio::test]
async fn test_over_consumption_fractional() {
    // 3/4 remaining, try to consume 1 (= 4/4 > 3/4)
    let val = Value::from_fraction(frac(3, 4));
    let token = FlowToken::from_value(&val);

    let err = token.consume(&frac(1, 1)).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("Over-consumption"), "Got: {}", msg);
}

#[tokio::test]
async fn test_over_consumption_after_partial() {
    let val = Value::from_fraction(frac(10, 1));
    let token = FlowToken::from_value(&val);

    let (_, t1) = token.consume(&frac(7, 1)).unwrap();
    assert_eq!(t1.remaining, frac(3, 1));

    // Now try to consume 5 from the 3 remaining
    let err = t1.consume(&frac(5, 1)).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("Over-consumption"), "Got: {}", msg);
}

// ──────────────────────────────────────────────
// 3. Remainder inheritance chain tests (残余継承チェーンテスト)
// ──────────────────────────────────────────────

#[tokio::test]
async fn test_remainder_chain_id_preserved() {
    let val = Value::from_fraction(frac(20, 1));
    let token = FlowToken::from_value(&val);
    let original_id = token.id;

    let (_, t1) = token.consume(&frac(5, 1)).unwrap();
    assert_eq!(t1.id, original_id, "Chain ID must be preserved after consumption");

    let (_, t2) = t1.consume(&frac(8, 1)).unwrap();
    assert_eq!(t2.id, original_id, "Chain ID must be preserved across multiple consumptions");
}

#[tokio::test]
async fn test_remainder_inheritance_values() {
    let val = Value::from_fraction(frac(50, 1));
    let token = FlowToken::from_value(&val);

    // Chain of consumptions: 10 -> 15 -> 20 -> 5 = 50 total
    let (_, t1) = token.consume(&frac(10, 1)).unwrap();
    assert_eq!(t1.remaining, frac(40, 1));

    let (_, t2) = t1.consume(&frac(15, 1)).unwrap();
    assert_eq!(t2.remaining, frac(25, 1));

    let (_, t3) = t2.consume(&frac(20, 1)).unwrap();
    assert_eq!(t3.remaining, frac(5, 1));

    let (_, t4) = t3.consume(&frac(5, 1)).unwrap();
    assert_eq!(t4.remaining, frac(0, 1));
    assert!(t4.is_exhausted());
}

#[tokio::test]
async fn test_remainder_hint_preserved() {
    // TODO: DisplayHint is now in SemanticRegistry, not on FlowToken.
    // This test verifies that FlowToken preserves value through chain.
    let val = Value::from_bool(true); // Scalar(1/1)
    let token = FlowToken::from_value(&val);

    let (_, t1) = token.consume(&frac(1, 1)).unwrap();
    assert_eq!(t1.remaining, frac(0, 1), "Remaining must be zero after full consumption");
}

#[tokio::test]
async fn test_flow_id_uniqueness() {
    let v1 = Value::from_fraction(frac(1, 1));
    let v2 = Value::from_fraction(frac(2, 1));

    let t1 = FlowToken::from_value(&v1);
    let t2 = FlowToken::from_value(&v2);

    assert_ne!(t1.id, t2.id, "Different values must get unique flow IDs");
}

// ──────────────────────────────────────────────
// 4. Complete consumption tests (完全消費テスト)
// ──────────────────────────────────────────────

#[tokio::test]
async fn test_complete_consumption_success() {
    let val = Value::from_fraction(frac(7, 1));
    let token = FlowToken::from_value(&val);

    let (_, t1) = token.consume(&frac(7, 1)).unwrap();
    assert!(t1.is_exhausted());
    assert!(t1.assert_complete("test").is_ok());
}

#[tokio::test]
async fn test_complete_consumption_failure() {
    let val = Value::from_fraction(frac(7, 1));
    let token = FlowToken::from_value(&val);

    let (_, t1) = token.consume(&frac(5, 1)).unwrap();
    assert!(!t1.is_exhausted());

    let err = t1.assert_complete("test pipeline end").unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("Unconsumed leak"),
        "Expected UnconsumedLeak error, got: {}",
        msg
    );
}

#[tokio::test]
async fn test_complete_consumption_nil_has_zero_total() {
    let val = Value::nil();
    let token = FlowToken::from_value(&val);

    // NIL contributes nothing to conservation — already exhausted
    assert_eq!(token.total, frac(0, 1));
    assert!(token.is_exhausted());
    assert!(token.assert_complete("nil context").is_ok());
}

#[tokio::test]
async fn test_complete_consumption_via_chain() {
    let val = Value::from_fraction(frac(12, 1));
    let token = FlowToken::from_value(&val);

    let (_, t1) = token.consume(&frac(4, 1)).unwrap();
    let (_, t2) = t1.consume(&frac(4, 1)).unwrap();
    let (_, t3) = t2.consume(&frac(4, 1)).unwrap();

    assert!(t3.is_exhausted());
    assert!(t3.assert_complete("chained pipeline end").is_ok());
    t3.verify_conservation(&[frac(4, 1), frac(4, 1), frac(4, 1)])
        .unwrap();
}

// ──────────────────────────────────────────────
// 5. Integration with interpreter execution
// ──────────────────────────────────────────────

#[tokio::test]
async fn test_interpreter_flow_tracking_simple_addition() {
    let (stack, interp) = run_with_flow_tracking("[ 10 ] [ 20 ] ! +").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_number(&stack[0], 30, 1);
    // Should not violate conservation
    assert!(interp.verify_all_flows().is_ok());
}

#[tokio::test]
async fn test_interpreter_flow_tracking_vector_ops() {
    let (stack, interp) = run_with_flow_tracking("[ 1 2 3 ] [ 10 ] ! +").await.unwrap();
    assert_eq!(stack.len(), 1);
    // [1,2,3] + [10] broadcasts to [11,12,13]
    let vec = stack[0].as_vector().unwrap();
    assert_eq!(vec.len(), 3);
    assert!(interp.verify_all_flows().is_ok());
}

#[tokio::test]
async fn test_interpreter_flow_tracking_chained_ops() {
    let (stack, interp) = run_with_flow_tracking("[ 5 ] [ 3 ] ! + [ 2 ] ! *").await.unwrap();
    assert_eq!(stack.len(), 1);
    // (5 + 3) * 2 = 16
    assert_number(&stack[0], 16, 1);
    assert!(interp.verify_all_flows().is_ok());
}

#[tokio::test]
async fn test_flow_token_shape_tracking() {
    // 2x3 matrix
    let val = Value::from_children(vec![
        Value::from_children(vec![
            Value::from_fraction(frac(1, 1)),
            Value::from_fraction(frac(2, 1)),
            Value::from_fraction(frac(3, 1)),
        ]),
        Value::from_children(vec![
            Value::from_fraction(frac(4, 1)),
            Value::from_fraction(frac(5, 1)),
            Value::from_fraction(frac(6, 1)),
        ]),
    ]);
    let token = FlowToken::from_value(&val);
    assert_eq!(token.shape, vec![2, 3]);
    // Total = 1+2+3+4+5+6 = 21
    assert_eq!(token.total, frac(21, 1));
}

// ──────────────────────────────────────────────
// 6. Bifurcation tests (分流テスト)
// ──────────────────────────────────────────────

#[tokio::test]
async fn test_bifurcation_mass_sum_equals_parent() {
    // Bifurcating a flow into 2 branches: child masses should sum to parent remaining
    let val = Value::from_fraction(frac(100, 1));
    let token = FlowToken::from_value(&val);
    let parent_remaining = token.remaining.clone();

    let (_parent, children) = token.bifurcate(2).unwrap();
    assert_eq!(children.len(), 2);

    // Each child gets half
    assert_eq!(children[0].total, frac(50, 1));
    assert_eq!(children[1].total, frac(50, 1));

    // Sum of children == parent remaining
    FlowToken::verify_bifurcation_conservation(&parent_remaining, &children).unwrap();
}

#[tokio::test]
async fn test_bifurcation_three_branches() {
    // Bifurcating into 3 branches (e.g., ,, on a Fold-type word with 2 operands + result)
    let val = Value::from_fraction(frac(90, 1));
    let token = FlowToken::from_value(&val);
    let parent_remaining = token.remaining.clone();

    let (_parent, children) = token.bifurcate(3).unwrap();
    assert_eq!(children.len(), 3);

    // Each child gets 30
    assert_eq!(children[0].total, frac(30, 1));
    assert_eq!(children[1].total, frac(30, 1));
    assert_eq!(children[2].total, frac(30, 1));

    FlowToken::verify_bifurcation_conservation(&parent_remaining, &children).unwrap();
}

#[tokio::test]
async fn test_bifurcation_fractional_mass() {
    // 1/3 bifurcated into 2: each child gets 1/6
    let val = Value::from_fraction(frac(1, 3));
    let token = FlowToken::from_value(&val);
    let parent_remaining = token.remaining.clone();

    let (_parent, children) = token.bifurcate(2).unwrap();
    assert_eq!(children[0].total, frac(1, 6));
    assert_eq!(children[1].total, frac(1, 6));

    FlowToken::verify_bifurcation_conservation(&parent_remaining, &children).unwrap();
}

#[tokio::test]
async fn test_bifurcation_parent_exhausted() {
    // After bifurcation, parent's remaining should be 0
    let val = Value::from_fraction(frac(42, 1));
    let token = FlowToken::from_value(&val);

    let (parent, _children) = token.bifurcate(2).unwrap();
    assert!(parent.is_exhausted());
    assert_eq!(parent.remaining, frac(0, 1));
}

#[tokio::test]
async fn test_bifurcation_parent_child_ids() {
    let val = Value::from_fraction(frac(10, 1));
    let token = FlowToken::from_value(&val);

    let (parent, children) = token.bifurcate(2).unwrap();

    // Parent should record child IDs
    assert_eq!(parent.child_flow_ids.len(), 2);
    assert_eq!(parent.child_flow_ids[0], children[0].id);
    assert_eq!(parent.child_flow_ids[1], children[1].id);

    // Children should reference parent
    assert_eq!(children[0].parent_flow_id, Some(parent.id));
    assert_eq!(children[1].parent_flow_id, Some(parent.id));
}

#[tokio::test]
async fn test_bifurcation_mass_ratio() {
    let val = Value::from_fraction(frac(10, 1));
    let token = FlowToken::from_value(&val);

    let (_parent, children) = token.bifurcate(3).unwrap();
    for child in &children {
        assert_eq!(child.mass_ratio, (1, 3));
    }
}

#[tokio::test]
async fn test_bifurcation_child_overconsumption() {
    // After bifurcation, each child has limited mass; consuming too much should fail
    let val = Value::from_fraction(frac(10, 1));
    let token = FlowToken::from_value(&val);

    let (_parent, children) = token.bifurcate(2).unwrap();
    // Each child has mass 5

    let err = children[0].consume(&frac(6, 1)).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("Over-consumption"), "Got: {}", msg);
}

#[tokio::test]
async fn test_bifurcation_child_unconsumed_leak() {
    // If a child is only partially consumed, assert_complete should fail
    let val = Value::from_fraction(frac(10, 1));
    let token = FlowToken::from_value(&val);

    let (_parent, children) = token.bifurcate(2).unwrap();
    // Each child has mass 5

    let (_, child_after) = children[0].consume(&frac(3, 1)).unwrap();
    let err = child_after.assert_complete("bifurcation branch end").unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("Unconsumed leak"), "Got: {}", msg);
}

#[tokio::test]
async fn test_bifurcation_zero_mass() {
    // NIL (zero mass) bifurcation should succeed with zero-mass children
    let val = Value::nil();
    let token = FlowToken::from_value(&val);

    let (parent, children) = token.bifurcate(2).unwrap();
    assert!(parent.is_exhausted());
    assert_eq!(children[0].total, frac(0, 1));
    assert_eq!(children[1].total, frac(0, 1));
    assert!(children[0].is_exhausted());
    assert!(children[1].is_exhausted());
}

#[tokio::test]
async fn test_bifurcation_with_dot_dot_combined() {
    // ,, with .. mode: the interpreter should still produce correct results
    let result = run("[ 1 2 3 4 5 ] ,, LENGTH").await.unwrap();
    assert_eq!(result.len(), 2); // original + length
    assert_number(&result[1], 5, 1);
}

#[tokio::test]
async fn test_bifurcation_interpreter_keep_mode() {
    // ,, GET should produce 3 values on stack (original vec, index, result)
    let result = run("[ 10 20 30 ] [ 1 ] ,, GET").await.unwrap();
    assert_eq!(result.len(), 3);
    assert_number(&result[2], 20, 1);
}

#[tokio::test]
async fn test_bifurcation_interpreter_arithmetic() {
    // ,, + should produce 3 values on stack
    let result = run("[ 3 ] [ 4 ] ,, +").await.unwrap();
    assert_eq!(result.len(), 3);
    assert_number(&result[0], 3, 1);
    assert_number(&result[1], 4, 1);
    assert_number(&result[2], 7, 1);
}
