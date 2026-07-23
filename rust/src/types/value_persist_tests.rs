//! Round-trip identity tests for the lossless persistence codec
//! (`crate::types::value_persist`). The oracle is `decode(encode(v)) == v`,
//! exercised through the `pub(crate)` stack boundary (`encode_stack` /
//! `decode_stack`) so the tests cover the exact path the WASM
//! `snapshot_stack` / `restore_stack_snapshot` methods take.

use crate::types::exact::ExactReal;
use crate::types::fraction::Fraction;
use crate::types::record_shape::record_shape_from_ordered_keys;
use crate::types::value_persist::{decode_stack, encode_stack};
use crate::types::{Interpretation, Token, Value, ValueData};
use num_bigint::BigInt;
use num_traits::One;
use std::str::FromStr;
use std::sync::Arc;

/// Round-trip one value as a single stack slot and return the decoded value.
fn roundtrip(value: &Value, role: Interpretation) -> (Value, Interpretation) {
    let json = encode_stack(std::iter::once((value, role))).expect("encode_stack");
    let mut decoded = decode_stack(&json).expect("decode_stack");
    assert_eq!(decoded.len(), 1, "single slot in, single slot out");
    decoded.pop().unwrap()
}

/// Assert a value survives the codec with its identity (data + hint) intact.
fn assert_value_roundtrip(value: Value) {
    let (decoded, _) = roundtrip(&value, Interpretation::Unassigned);
    assert_eq!(decoded, value, "value round-trip must preserve identity");
}

/// Assert both the value and its stack-position role survive.
fn assert_stack_roundtrip(value: Value, role: Interpretation) {
    let (decoded, decoded_role) = roundtrip(&value, role);
    assert_eq!(decoded, value);
    assert_eq!(decoded_role, role);
}

fn sqrt(n: i64) -> Value {
    Value::from_exact_real(ExactReal::from_sqrt_rational(Fraction::from(n)).expect("sqrt exists"))
}

#[test]
fn code_block_survives_round_trip_instead_of_becoming_nil() {
    // Regression: the observation protocol mapped CodeBlock -> nil, so
    // save/restore replaced a code block with a genuine NIL.
    let value = Value::from_code_block(vec![
        Token::Number(Arc::from("42")),
        Token::Symbol(Arc::from("ADD")),
        Token::VectorStart,
        Token::VectorEnd,
    ]);
    assert!(matches!(value.data, ValueData::CodeBlock(_)));
    assert_stack_roundtrip(value, Interpretation::Unassigned);
}

#[test]
fn exact_sqrt_survives_round_trip_instead_of_becoming_rational() {
    // Regression: √2 was serialized as its rational approximation and
    // restored as that exact rational, changing the mathematical value.
    let value = sqrt(2);
    assert!(matches!(value.data, ValueData::ExactScalar(_)));
    assert_stack_roundtrip(value, Interpretation::RawNumber);
}

#[test]
fn exact_algebraic_sum_round_trips() {
    // A multi-term multiquadratic value: √2 + √3.
    let a = ExactReal::from_sqrt_rational(Fraction::from(2)).unwrap();
    let b = ExactReal::from_sqrt_rational(Fraction::from(3)).unwrap();
    let value = Value::from_exact_real(a.add(&b));
    assert!(matches!(value.data, ValueData::ExactScalar(_)));
    assert_value_roundtrip(value);
}

#[test]
fn exact_algebraic_with_rational_part_round_trips() {
    // 1 + √2 exercises the monomial-1 (rational) term alongside √2.
    let one = ExactReal::from_integer(1);
    let root2 = ExactReal::from_sqrt_rational(Fraction::from(2)).unwrap();
    let value = Value::from_exact_real(one.add(&root2));
    assert_value_roundtrip(value);
}

#[test]
fn scalars_booleans_and_absence_values_round_trip() {
    assert_value_roundtrip(Value::from_int(42));
    assert_value_roundtrip(Value::from_fraction(Fraction::new(
        BigInt::from(22),
        BigInt::from(7),
    )));
    assert_value_roundtrip(Value::from_bool(true));
    assert_value_roundtrip(Value::from_bool(false));
    assert_value_roundtrip(Value::nil());
    assert_value_roundtrip(Value::unknown());
}

#[test]
fn big_integer_scalar_round_trips() {
    // Beyond i64 range: the codec must not narrow through i64.
    let big = BigInt::from_str("340282366920938463463374607431768211457").unwrap();
    assert_value_roundtrip(Value::from_fraction(Fraction::new(big, BigInt::one())));
}

#[test]
fn handles_round_trip() {
    assert_value_roundtrip(Value::from_process_handle(7));
    assert_value_roundtrip(Value::from_supervisor_handle(9));
}

#[test]
fn nested_vector_round_trips() {
    let value = Value::from_vector(vec![
        Value::from_int(1),
        Value::from_vector(vec![Value::from_int(2), sqrt(5)]),
        Value::nil(),
    ]);
    assert_value_roundtrip(value);
}

#[test]
fn tensor_with_nil_lane_round_trips() {
    let mut tensor = Value::from_int_tensor(vec![1, 2, 3, 4]);
    if let ValueData::Tensor { data, .. } = &mut tensor.data {
        let dense = Arc::make_mut(data);
        dense.clear_valid(1);
    } else {
        panic!("expected tensor");
    }
    assert_value_roundtrip(tensor);
}

#[test]
fn record_round_trips_keys_and_values() {
    let shape = record_shape_from_ordered_keys(["name".to_string(), "age".to_string()]);
    let value = Value {
        data: ValueData::Record {
            pairs: Arc::new(vec![sqrt(2), Value::from_int(30)]),
            shape,
        },
        hint: Interpretation::Unassigned,
        absence: None,
    };
    assert_value_roundtrip(value);
}

#[test]
fn hint_role_is_preserved_across_the_stack_boundary() {
    // The value's own hint and the stack-position role are independent
    // and must both survive.
    let value = Value {
        data: ValueData::Scalar(Fraction::from(5)),
        hint: Interpretation::Text,
        absence: None,
    };
    assert_stack_roundtrip(value, Interpretation::Timestamp);
}

#[test]
fn multi_slot_stack_round_trips_in_order() {
    let values = [
        (Value::from_int(1), Interpretation::RawNumber),
        (sqrt(2), Interpretation::RawNumber),
        (
            Value::from_code_block(vec![Token::Number(Arc::from("9"))]),
            Interpretation::Unassigned,
        ),
    ];
    let json = encode_stack(values.iter().map(|(v, r)| (v, *r))).expect("encode");
    let decoded = decode_stack(&json).expect("decode");
    assert_eq!(decoded.len(), values.len());
    for (got, want) in decoded.iter().zip(values.iter()) {
        assert_eq!(got.0, want.0);
        assert_eq!(got.1, want.1);
    }
}
