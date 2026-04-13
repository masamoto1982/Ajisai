

use ajisai_core::interpreter::Interpreter;
use ajisai_core::types::fraction::Fraction;
use ajisai_core::types::{FlowToken, Value};
use num_bigint::BigInt;


async fn run(code: &str) -> Result<Vec<Value>, String> {
    let mut interp = Interpreter::new();
    interp.execute(code).await.map_err(|e| e.to_string())?;
    Ok(interp.get_stack().clone())
}

async fn run_with_flow_tracking(code: &str) -> Result<(Vec<Value>, Interpreter), String> {
    let mut interp = Interpreter::new();
    interp.update_flow_tracking(true);
    interp.execute(code).await.map_err(|e| e.to_string())?;
    let stack = interp.get_stack().clone();
    Ok((stack, interp))
}

fn frac(n: i64, d: i64) -> Fraction {
    Fraction::new(BigInt::from(n), BigInt::from(d))
}

fn assert_number(val: &Value, num: i64, denom: i64) {
    let expected = Fraction::new(BigInt::from(num), BigInt::from(denom));

    if let Some(f) = val.as_scalar() {
        assert_eq!(f, &expected, "Expected {}/{}, got {}", num, denom, f);
    } else if let Some(vec) = val.as_vector() {
        if vec.len() == 1 {
            let f = vec[0]
                .as_scalar()
                .unwrap_or_else(|| panic!("Expected scalar in vector, got {:?}", vec[0]));
            assert_eq!(f, &expected, "Expected {}/{}, got {}", num, denom, f);
        } else {
            panic!(
                "Expected single scalar, got vector of length {}: {:?}",
                vec.len(),
                val
            );
        }
    } else {
        panic!("Expected scalar, got {:?}", val);
    }
}


#[tokio::test]
async fn test_conservation_flow_token_basic() {

    let val = Value::from_fraction(frac(10, 1));
    let token = FlowToken::from_value(&val);

    assert_eq!(token.total, frac(10, 1));
    assert_eq!(token.remaining, frac(10, 1));
    assert!(!token.is_exhausted());


    let (consumed, token2) = token.consume(&frac(3, 1)).unwrap();
    assert_eq!(consumed, frac(3, 1));
    assert_eq!(token2.remaining, frac(7, 1));


    token2.verify_conservation(&[frac(3, 1)]).unwrap();
}


#[tokio::test]
async fn test_conservation_violation_maps_to_flow_break_error() {
    let val = Value::from_fraction(frac(10, 1));
    let token = FlowToken::from_value(&val);

    let err = token.verify_conservation(&[frac(1, 1)]).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("Flow break"), "Expected FlowBreak display, got: {}", msg);
}

#[tokio::test]
async fn test_bifurcation_violation_maps_to_bifurcation_error() {
    let parent_remaining = frac(5, 1);
    let fake_child = FlowToken::from_value(&Value::from_fraction(frac(1, 1)));

    let err = FlowToken::verify_bifurcation_conservation(&parent_remaining, &[fake_child]).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("Bifurcation conservation violation"), "Expected BifurcationViolation display, got: {}", msg);
}

#[tokio::test]
async fn test_conservation_vector_total() {

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

    let val = Value::from_children(vec![
        Value::from_fraction(frac(-3, 1)),
        Value::from_fraction(frac(5, 1)),
    ]);
    let token = FlowToken::from_value(&val);
    assert_eq!(token.total, frac(8, 1));
}

#[tokio::test]
async fn test_conservation_fractional_values() {

    let val = Value::from_children(vec![
        Value::from_fraction(frac(1, 3)),
        Value::from_fraction(frac(2, 3)),
    ]);
    let token = FlowToken::from_value(&val);
    assert_eq!(token.total, frac(1, 1));
}

#[tokio::test]
async fn test_conservation_multi_step() {

    let val = Value::from_fraction(frac(100, 1));
    let token = FlowToken::from_value(&val);

    let (_, t1) = token.consume(&frac(30, 1)).unwrap();
    t1.verify_conservation(&[frac(30, 1)]).unwrap();

    let (_, t2) = t1.consume(&frac(25, 1)).unwrap();
    t2.verify_conservation(&[frac(30, 1), frac(25, 1)]).unwrap();

    let (_, t3) = t2.consume(&frac(45, 1)).unwrap();
    t3.verify_conservation(&[frac(30, 1), frac(25, 1), frac(45, 1)])
        .unwrap();


    assert!(t3.is_exhausted());
}

#[tokio::test]
async fn test_conservation_with_interpreter_tracking() {

    let (stack, interp) = run_with_flow_tracking("[ 5 ] [ 3 ] ! +").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_number(&stack[0], 8, 1);


    assert!(interp.verify_all_flows().is_ok());
}


#[tokio::test]
async fn test_over_consumption_error() {
    let val = Value::from_fraction(frac(5, 1));
    let token = FlowToken::from_value(&val);


    let result = token.consume(&frac(10, 1));
    assert!(
        result.is_err(),
        "Should fail when consuming more than available"
    );
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


    let err = t1.consume(&frac(5, 1)).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("Over-consumption"), "Got: {}", msg);
}


#[tokio::test]
async fn test_remainder_chain_id_preserved() {
    let val = Value::from_fraction(frac(20, 1));
    let token = FlowToken::from_value(&val);
    let original_id = token.id;

    let (_, t1) = token.consume(&frac(5, 1)).unwrap();
    assert_eq!(
        t1.id, original_id,
        "Chain ID must be preserved after consumption"
    );

    let (_, t2) = t1.consume(&frac(8, 1)).unwrap();
    assert_eq!(
        t2.id, original_id,
        "Chain ID must be preserved across multiple consumptions"
    );
}

#[tokio::test]
async fn test_linear_reuse_hint() {
    let val = Value::from_fraction(frac(20, 1));
    let token = FlowToken::from_value(&val);
    assert!(token.is_reusable_allocation());

    let (_, consumed_once) = token.consume(&frac(1, 1)).unwrap();
    assert!(!consumed_once.is_reusable_allocation());
}

#[tokio::test]
async fn test_remainder_inheritance_values() {
    let val = Value::from_fraction(frac(50, 1));
    let token = FlowToken::from_value(&val);


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


    let val = Value::from_bool(true);
    let token = FlowToken::from_value(&val);

    let (_, t1) = token.consume(&frac(1, 1)).unwrap();
    assert_eq!(
        t1.remaining,
        frac(0, 1),
        "Remaining must be zero after full consumption"
    );
}

#[tokio::test]
async fn test_flow_id_uniqueness() {
    let v1 = Value::from_fraction(frac(1, 1));
    let v2 = Value::from_fraction(frac(2, 1));

    let t1 = FlowToken::from_value(&v1);
    let t2 = FlowToken::from_value(&v2);

    assert_ne!(t1.id, t2.id, "Different values must get unique flow IDs");
}


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


#[tokio::test]
async fn test_interpreter_flow_tracking_simple_addition() {
    let (stack, interp) = run_with_flow_tracking("[ 10 ] [ 20 ] ! +").await.unwrap();
    assert_eq!(stack.len(), 1);
    assert_number(&stack[0], 30, 1);

    assert!(interp.verify_all_flows().is_ok());
}

#[tokio::test]
async fn test_interpreter_flow_tracking_vector_ops() {
    let (stack, interp) = run_with_flow_tracking("[ 1 2 3 ] [ 10 ] ! +")
        .await
        .unwrap();
    assert_eq!(stack.len(), 1);

    let vec = stack[0].as_vector().unwrap();
    assert_eq!(vec.len(), 3);
    assert!(interp.verify_all_flows().is_ok());
}

#[tokio::test]
async fn test_interpreter_flow_tracking_chained_ops() {
    let (stack, interp) = run_with_flow_tracking("[ 5 ] [ 3 ] ! + [ 2 ] ! *")
        .await
        .unwrap();
    assert_eq!(stack.len(), 1);

    assert_number(&stack[0], 16, 1);
    assert!(interp.verify_all_flows().is_ok());
}

#[tokio::test]
async fn test_flow_token_shape_tracking() {

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

    assert_eq!(token.total, frac(21, 1));
}


#[tokio::test]
async fn test_bifurcation_mass_sum_equals_parent() {

    let val = Value::from_fraction(frac(100, 1));
    let token = FlowToken::from_value(&val);
    let parent_remaining = token.remaining.clone();

    let (_parent, children) = token.bifurcate(2).unwrap();
    assert_eq!(children.len(), 2);


    assert_eq!(children[0].total, frac(50, 1));
    assert_eq!(children[1].total, frac(50, 1));


    FlowToken::verify_bifurcation_conservation(&parent_remaining, &children).unwrap();
}

#[tokio::test]
async fn test_bifurcation_three_branches() {

    let val = Value::from_fraction(frac(90, 1));
    let token = FlowToken::from_value(&val);
    let parent_remaining = token.remaining.clone();

    let (_parent, children) = token.bifurcate(3).unwrap();
    assert_eq!(children.len(), 3);


    assert_eq!(children[0].total, frac(30, 1));
    assert_eq!(children[1].total, frac(30, 1));
    assert_eq!(children[2].total, frac(30, 1));

    FlowToken::verify_bifurcation_conservation(&parent_remaining, &children).unwrap();
}

#[tokio::test]
async fn test_bifurcation_fractional_mass() {

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


    assert_eq!(parent.child_flow_ids.len(), 2);
    assert_eq!(parent.child_flow_ids[0], children[0].id);
    assert_eq!(parent.child_flow_ids[1], children[1].id);


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

    let val = Value::from_fraction(frac(10, 1));
    let token = FlowToken::from_value(&val);

    let (_parent, children) = token.bifurcate(2).unwrap();


    let err = children[0].consume(&frac(6, 1)).unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("Over-consumption"), "Got: {}", msg);
}

#[tokio::test]
async fn test_bifurcation_child_unconsumed_leak() {

    let val = Value::from_fraction(frac(10, 1));
    let token = FlowToken::from_value(&val);

    let (_parent, children) = token.bifurcate(2).unwrap();


    let (_, child_after) = children[0].consume(&frac(3, 1)).unwrap();
    let err = child_after
        .assert_complete("bifurcation branch end")
        .unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("Unconsumed leak"), "Got: {}", msg);
}

#[tokio::test]
async fn test_bifurcation_zero_mass() {

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

    let result = run("[ 1 2 3 4 5 ] ,, LENGTH").await.unwrap();
    assert_eq!(result.len(), 2);
    assert_number(&result[1], 5, 1);
}

#[tokio::test]
async fn test_bifurcation_interpreter_keep_mode() {

    let result = run("[ 10 20 30 ] [ 1 ] ,, GET").await.unwrap();
    assert_eq!(result.len(), 3);
    assert_number(&result[2], 20, 1);
}

#[tokio::test]
async fn test_bifurcation_interpreter_arithmetic() {

    let result = run("[ 3 ] [ 4 ] ,, +").await.unwrap();
    assert_eq!(result.len(), 3);
    assert_number(&result[0], 3, 1);
    assert_number(&result[1], 4, 1);
    assert_number(&result[2], 7, 1);
}


#[tokio::test]
async fn test_can_update_in_place_uniquely_owned_scalar() {
    let val = Value::from_fraction(frac(42, 1));
    let token = FlowToken::from_value(&val);


    assert!(token.can_update_in_place(&val));
}

#[tokio::test]
async fn test_can_update_in_place_uniquely_owned_vector() {
    let val = Value::from_vector(vec![
        Value::from_int(1),
        Value::from_int(2),
        Value::from_int(3),
    ]);
    let token = FlowToken::from_value(&val);


    assert!(token.can_update_in_place(&val));
}

#[tokio::test]
async fn test_can_update_in_place_after_partial_consumption() {
    let val = Value::from_fraction(frac(10, 1));
    let token = FlowToken::from_value(&val);


    let (_, consumed_token) = token.consume(&frac(3, 1)).unwrap();
    assert!(!consumed_token.can_update_in_place(&val));
}

#[tokio::test]
async fn test_can_update_in_place_with_aliased_vector() {
    use std::rc::Rc;
    use ajisai_core::types::ValueData;

    let children = Rc::new(vec![Value::from_int(1), Value::from_int(2)]);
    let _alias = children.clone();
    let val = Value { data: ValueData::Vector(children) };
    let token = FlowToken::from_value(&val);


    assert!(!val.is_uniquely_owned());
    assert!(token.is_reusable_allocation());
    assert!(!token.can_update_in_place(&val));
}

#[tokio::test]
async fn test_can_update_in_place_after_bifurcation() {
    let val = Value::from_fraction(frac(100, 1));
    let token = FlowToken::from_value(&val);


    let (parent, children) = token.bifurcate(2).unwrap();
    assert!(!parent.can_update_in_place(&val));


    assert!(!children[0].can_update_in_place(&val));
}

#[tokio::test]
async fn test_is_uniquely_owned_nil() {
    let val = Value::nil();
    assert!(val.is_uniquely_owned());
}

#[tokio::test]
async fn test_is_uniquely_owned_code_block() {
    let val = Value::from_code_block(vec![]);
    assert!(!val.is_uniquely_owned());
}


#[tokio::test]
async fn test_fraction_svo_small_construction() {

    let f = Fraction::from(42i64);
    assert_eq!(f.to_i64(), Some(42));
    assert!(f.is_integer());
    assert!(!f.is_nil());
    assert!(!f.is_zero());
}

#[tokio::test]
async fn test_fraction_svo_zero() {
    let f = Fraction::from(0i64);
    assert!(f.is_zero());
    assert!(f.is_integer());
    assert_eq!(f.to_i64(), Some(0));
}

#[tokio::test]
async fn test_fraction_svo_nil() {
    let f = Fraction::nil();
    assert!(f.is_nil());
    assert_eq!(f.to_i64(), None);
}

#[tokio::test]
async fn test_fraction_svo_arithmetic_stays_small() {

    let a = Fraction::from(100i64);
    let b = Fraction::from(200i64);
    let c = a.add(&b);
    assert_eq!(c.to_i64(), Some(300));
}

#[tokio::test]
async fn test_fraction_svo_fractional_arithmetic() {

    let a = Fraction::new(BigInt::from(1), BigInt::from(3));
    let b = Fraction::new(BigInt::from(1), BigInt::from(6));
    let c = a.add(&b);
    assert_eq!(c, Fraction::new(BigInt::from(1), BigInt::from(2)));
}

#[tokio::test]
async fn test_fraction_svo_clone_is_cheap() {

    let f = Fraction::from(999i64);
    let g = f.clone();
    assert_eq!(f, g);
    assert_eq!(g.to_i64(), Some(999));
}

#[tokio::test]
async fn test_fraction_svo_comparison() {
    let half = Fraction::new(BigInt::from(1), BigInt::from(2));
    let third = Fraction::new(BigInt::from(1), BigInt::from(3));
    assert!(half.gt(&third));
    assert!(third.lt(&half));
    assert_eq!(half, half.clone());
}

#[tokio::test]
async fn test_fraction_svo_display() {
    let f = Fraction::from(42i64);
    assert_eq!(format!("{}", f), "42");

    let g = Fraction::new(BigInt::from(1), BigInt::from(3));
    assert_eq!(format!("{}", g), "1/3");
}

#[tokio::test]
async fn test_fraction_svo_accessor_methods() {
    let f = Fraction::new(BigInt::from(3), BigInt::from(7));
    assert_eq!(f.numerator(), BigInt::from(3));
    assert_eq!(f.denominator(), BigInt::from(7));
}

#[tokio::test]
async fn test_fraction_svo_floor_ceil_round() {
    let f = Fraction::new(BigInt::from(7), BigInt::from(3));
    assert_eq!(f.floor().to_i64(), Some(2));
    assert_eq!(f.ceil().to_i64(), Some(3));
    assert_eq!(f.round().to_i64(), Some(2));

    let neg = Fraction::new(BigInt::from(-7), BigInt::from(3));
    assert_eq!(neg.floor().to_i64(), Some(-3));
    assert_eq!(neg.ceil().to_i64(), Some(-2));
    assert_eq!(neg.round().to_i64(), Some(-2));
}

#[tokio::test]
async fn test_fraction_svo_modulo() {
    let a = Fraction::from(17i64);
    let b = Fraction::from(5i64);
    assert_eq!(a.modulo(&b).to_i64(), Some(2));
}


#[tokio::test]
async fn test_collect_fractions_flat_into_preallocated() {
    let val = Value::from_vector(vec![
        Value::from_int(10),
        Value::from_int(20),
        Value::from_int(30),
    ]);


    assert_eq!(val.count_fractions(), 3);


    let mut buf = Vec::with_capacity(val.count_fractions());
    val.collect_fractions_flat_into(&mut buf);
    assert_eq!(buf.len(), 3);
    assert_eq!(buf[0].to_i64(), Some(10));
    assert_eq!(buf[1].to_i64(), Some(20));
    assert_eq!(buf[2].to_i64(), Some(30));
}

#[tokio::test]
async fn test_collect_fractions_nested_tensor() {

    let val = Value::from_vector(vec![
        Value::from_vector(vec![Value::from_int(1), Value::from_int(2), Value::from_int(3)]),
        Value::from_vector(vec![Value::from_int(4), Value::from_int(5), Value::from_int(6)]),
    ]);

    assert_eq!(val.count_fractions(), 6);
    let fracs = val.collect_fractions_flat();
    assert_eq!(fracs.len(), 6);
    for (i, f) in fracs.iter().enumerate() {
        assert_eq!(f.to_i64(), Some((i + 1) as i64));
    }
}
