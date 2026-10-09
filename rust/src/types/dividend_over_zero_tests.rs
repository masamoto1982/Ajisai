//! A quotient by zero is absent *as a number*: the dividend over the zero
//! that refused it.
//!
//! Absence is a denominator of 0 (`Fraction::is_nil`), and nothing more, so
//! the numerator is free to keep what was being divided: `100 0 DIV` is held
//! as `100/0`, `5 0 DIV` as `5/0`, and `0/0` is what `0 0 DIV` alone leaves.
//! An absence that was never a division — a written `NIL`, a key not found,
//! a negative radicand — holds no pair at all. This file holds that rule at
//! every layer the pair crosses: the `Fraction`, the scalar `Value`, the
//! dense Tensor's columns on the kernel and the general route, promotion into
//! columns and materialization back out, equality and hashing (the pair is
//! not part of an absence's identity), and the persistence codec.

use super::fraction::Fraction;
use super::value_persist::{decode_stack, encode_stack};
use super::{DenseTensor, Value, ValueData};
use crate::error::NilReason;
use crate::interpreter::Interpreter;
use crate::test_support::hash_of;

fn num(n: i64) -> Fraction {
    Fraction::from(n)
}

fn pair_of(value: &Value) -> Option<(i64, i64)> {
    value.absent_pair().and_then(Fraction::extract_i64_pair)
}

fn top(source: &str) -> Value {
    let mut interp = Interpreter::new();
    crate::agent::block_on(interp.execute(source)).unwrap_or_else(|e| panic!("`{source}`: {e:?}"));
    interp.get_stack().last().cloned().expect("a value")
}

fn columns(value: &Value) -> (Vec<i64>, Vec<i64>) {
    let ValueData::Tensor { data, .. } = &value.data else {
        panic!("not a dense Tensor: {value:?}");
    };
    (data.numerators.to_vec(), data.denominators.to_vec())
}

// ---- Fraction ----

#[test]
fn dividing_by_zero_keeps_the_dividend_over_zero() {
    assert_eq!(num(100).div(&num(0)).extract_i64_pair(), Some((100, 0)));
    assert_eq!(num(5).div(&num(0)).extract_i64_pair(), Some((5, 0)));
    assert_eq!(num(-5).div(&num(0)).extract_i64_pair(), Some((-5, 0)));
    // `a/b ÷ 0` is `a/(b·0)`: the dividend's numerator, nothing reduced.
    let half = Fraction::new(1.into(), 2.into());
    assert_eq!(half.div(&num(0)).extract_i64_pair(), Some((1, 0)));
    // `0/0` is what dividing zero by zero leaves, and nothing else here.
    assert_eq!(num(0).div(&num(0)).extract_i64_pair(), Some((0, 0)));
    assert!(num(100).div(&num(0)).is_nil());
}

#[test]
fn a_wide_dividend_is_kept_whole() {
    let wide = Fraction::new(num_bigint::BigInt::from(i64::MAX) * 4, 1.into());
    let absent = wide.div(&num(0));
    assert!(absent.is_nil());
    assert_eq!(absent.numerator(), wide.numerator());
    assert_eq!(absent.denominator(), num_bigint::BigInt::from(0));
}

#[test]
fn an_absent_number_is_not_zero_and_not_a_zero_divisor() {
    let absent = num(0).over_zero();
    assert!(absent.is_nil());
    assert!(!absent.is_zero(), "`0/0` is absent, not zero");
    assert!(!num(7).over_zero().is_zero());
    // So dividing by an absent number passes it through, rather than
    // projecting a division by zero the program never performed.
    assert_eq!(num(3).div(&absent).extract_i64_pair(), Some((0, 0)));
    assert_eq!(
        num(3).div(&num(7).over_zero()).extract_i64_pair(),
        Some((7, 0))
    );
}

#[test]
fn arithmetic_after_the_division_carries_the_pair_unchanged() {
    let absent = num(100).over_zero();
    for result in [
        absent.add(&num(1)),
        num(1).add(&absent),
        absent.sub(&num(1)),
        absent.mul(&num(0)),
        num(0).mul(&absent),
        absent.div(&num(2)),
        absent.neg().neg(),
        absent.floor(),
        absent.round(),
    ] {
        assert_eq!(result.extract_i64_pair(), Some((100, 0)));
    }
    // The leftmost absent operand is the one carried.
    assert_eq!(
        num(100)
            .over_zero()
            .add(&num(5).over_zero())
            .extract_i64_pair(),
        Some((100, 0))
    );
}

#[test]
fn the_pair_is_not_an_absences_identity() {
    let a = num(100).over_zero();
    let b = num(5).over_zero();
    assert_eq!(a, b);
    assert_eq!(a, Fraction::nil());
    let mut ha = std::collections::hash_map::DefaultHasher::new();
    let mut hb = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&a, &mut ha);
    std::hash::Hash::hash(&b, &mut hb);
    assert_eq!(
        std::hash::Hasher::finish(&ha),
        std::hash::Hasher::finish(&hb)
    );
}

// ---- Scalar Value ----

#[test]
fn a_scalar_quotient_by_zero_holds_its_dividend() {
    let quotient = top("100 0 DIV");
    assert!(quotient.is_nil());
    assert_eq!(
        quotient.nil_reason().copied(),
        Some(NilReason::DivisionByZero)
    );
    assert_eq!(pair_of(&quotient), Some((100, 0)));
    assert_eq!(pair_of(&top("5 0 DIV")), Some((5, 0)));
    assert_eq!(pair_of(&top("1/2 0 DIV")), Some((1, 0)));
    assert_eq!(pair_of(&top("0 0 DIV")), Some((0, 0)));
    assert_eq!(pair_of(&top("[ 6 ] [ 0 ] DIV 0 GET")), Some((6, 0)));
}

#[test]
fn an_absence_that_was_never_a_division_holds_no_pair() {
    for source in [
        "NIL",
        "[ 10 20 30 ] 5 GET",
        "'ABC' NUM",
        "-1 SQRT",
        "'why' ABSENT",
        "[ 1 2 ] 3 INDEX-OF",
    ] {
        let value = top(source);
        assert!(value.is_nil(), "`{source}`");
        assert_eq!(
            value.absent_pair(),
            None,
            "`{source}` has nothing over zero"
        );
    }
    // An algebraic dividend has no pair of integers to keep over the zero.
    let value = top("2 SQRT 0 DIV");
    assert_eq!(value.nil_reason().copied(), Some(NilReason::DivisionByZero));
    assert_eq!(value.absent_pair(), None);
}

#[test]
fn the_pair_travels_through_the_words_after_it() {
    assert_eq!(pair_of(&top("100 0 DIV 1 ADD")), Some((100, 0)));
    assert_eq!(pair_of(&top("100 0 DIV 0 MUL")), Some((100, 0)));
    assert_eq!(pair_of(&top("1 100 0 DIV SUB")), Some((100, 0)));
    assert_eq!(pair_of(&top("100 0 DIV FLOOR")), Some((100, 0)));
    assert_eq!(pair_of(&top("100 0 DIV 1 GT")), Some((100, 0)));
    assert_eq!(pair_of(&top("100 0 DIV 1 EQ")), Some((100, 0)));
    assert_eq!(pair_of(&top("100 0 DIV 1 MAX")), Some((100, 0)));
    assert_eq!(pair_of(&top("100 0 DIV 1 COLLECT 0 GET")), Some((100, 0)));
    // The observable value is the absence and its reason, whatever the pair.
    assert_eq!(top("100 0 DIV 1 ADD").to_string(), "NIL");
    assert_eq!(
        top("100 0 DIV 1 ADD NIL-REASON"),
        Value::from_string("divisionByZero")
    );
}

#[test]
fn the_pair_is_not_part_of_a_values_identity() {
    let hundred = top("100 0 DIV");
    let five = top("5 0 DIV");
    assert_eq!(hundred, five);
    assert_eq!(hash_of(&hundred), hash_of(&five));
    assert_eq!(
        top("100 0 DIV 1 COLLECT 5 0 DIV 1 COLLECT EQ"),
        Value::from_bool(true)
    );
    // The reason still is.
    assert_ne!(hundred, top("NIL"));
}

// ---- Dense columns ----

#[test]
fn the_kernel_writes_each_dividend_over_its_zero() {
    let (nums, dens) = columns(&top("[ 100 5 3 ] [ 0 0 1 ] DIV"));
    assert_eq!(nums, [100, 5, 3]);
    assert_eq!(dens, [0, 0, 1]);
    // Rational lanes take the other kernel loop; the dividend's numerator is
    // what is kept there too.
    let (nums, dens) = columns(&top("[ 1/2 3 ] [ 0 2 ] DIV"));
    assert_eq!(nums, [1, 3]);
    assert_eq!(dens, [0, 2]);
    // A scalar dividend across a Vector of divisors.
    let (nums, dens) = columns(&top("6 [ 1 0 2 ] DIV"));
    assert_eq!(nums, [6, 6, 3]);
    assert_eq!(dens, [1, 0, 1]);
    // The one-lane fast path.
    let (nums, dens) = columns(&top("[ 6 ] [ 0 ] DIV"));
    assert_eq!(nums, [6]);
    assert_eq!(dens, [0]);
}

#[test]
fn the_general_route_writes_the_same_columns() {
    for source in [
        "[ 100 5 3 ] [ 0 0 1 ] DIV",
        "[ 1/2 3 ] [ 0 2 ] DIV",
        "6 [ 1 0 2 ] DIV",
        "[ 100 5 3 ] [ 0 0 1 ] DIV 2 MUL",
        "[ 100 5 3 ] [ 0 0 1 ] DIV [ 1 1 0 ] DIV",
    ] {
        let mut with = Interpreter::new();
        with.set_dense_kernels_enabled(true);
        crate::agent::block_on(with.execute(source)).expect(source);
        let mut without = Interpreter::new();
        without.set_dense_kernels_enabled(false);
        crate::agent::block_on(without.execute(source)).expect(source);
        let with = with.get_stack().last().cloned().expect(source);
        let without = without.get_stack().last().cloned().expect(source);
        assert_eq!(columns(&with), columns(&without), "`{source}`");
    }
}

#[test]
fn the_words_after_the_division_carry_the_lanes_pair() {
    let (nums, dens) = columns(&top("[ 100 5 3 ] [ 0 0 1 ] DIV 2 MUL"));
    assert_eq!(nums, [100, 5, 6]);
    assert_eq!(dens, [0, 0, 1]);
    let (nums, dens) = columns(&top("[ 100 5 3 ] [ 0 0 1 ] DIV 1 ADD FLOOR"));
    assert_eq!(nums, [100, 5, 4]);
    assert_eq!(dens, [0, 0, 1]);
    // The leftmost absent operand lane is the one carried.
    let (nums, _) = columns(&top("[ 100 5 ] [ 0 1 ] DIV [ 7 8 ] [ 0 0 ] DIV ADD"));
    assert_eq!(nums, [100, 8]);
    // A dividend that would overflow the integer kernel's loop is no reason
    // to decline it: the lane is overwritten with its pair anyway.
    let (nums, dens) = columns(&top("[ 9223372036854775807 2 ] [ 0 1 ] DIV 2 MUL"));
    assert_eq!(nums, [i64::MAX, 4]);
    assert_eq!(dens, [0, 1]);
}

#[test]
fn two_tensors_that_differ_only_in_a_dividend_are_one_value() {
    let a = top("[ 100 3 ] [ 0 1 ] DIV");
    let b = top("[ 5 3 ] [ 0 1 ] DIV");
    assert_ne!(columns(&a), columns(&b), "the columns differ");
    assert_eq!(a, b, "the values do not");
    assert_eq!(hash_of(&a), hash_of(&b));
    assert_eq!(
        top("[ 100 3 ] [ 0 1 ] DIV [ 5 3 ] [ 0 1 ] DIV EQ"),
        Value::from_bool(true)
    );
    // Across representations too.
    let nested = top("[ 5 3 ] [ 0 1 ] DIV [ ] CONCAT");
    assert!(matches!(nested.data, ValueData::Vector(_)));
    assert_eq!(a, nested);
    assert_eq!(hash_of(&a), hash_of(&nested));
}

#[test]
fn a_lane_materializes_with_its_pair_and_promotes_back_with_it() {
    let lane = top("[ 100 5 ] [ 0 0 ] DIV 1 GET");
    assert_eq!(pair_of(&lane), Some((5, 0)));
    let (nums, dens) = columns(&Value::from_vector_promoted(vec![lane, Value::from_int(1)]));
    assert_eq!(nums, [5, 1]);
    assert_eq!(dens, [0, 1]);
    let (nums, dens) = columns(&top("[ 100 5 ] [ 0 0 ] DIV REVERSE"));
    assert_eq!(nums, [5, 100]);
    assert_eq!(dens, [0, 0]);
    let (nums, _) = columns(&top("[ 100 5 ] [ 0 0 ] DIV [ 7 ] [ 0 ] DIV CONCAT"));
    assert_eq!(nums, [100, 5, 7]);
    let (nums, _) = columns(&top("[ [ 1 2 ] [ 3 4 ] ] [ [ 0 1 ] [ 1 0 ] ] DIV 1 GET"));
    assert_eq!(nums, [3, 4]);
}

#[test]
fn a_dense_lane_holds_only_an_absent_number() {
    // A written NIL, or any other absence that is not a number, has no pair
    // to be a lane: the Vector keeps its nested form, as it does for a String.
    for source in ["[ 1 NIL 3 ]", "[ -1 4 ] SQRT", "[ 'a' ] [ ABSENT ] MAP"] {
        let value = top(source);
        assert!(
            matches!(value.data, ValueData::Vector(_)),
            "`{source}` stays nested: {value:?}"
        );
    }
    // A dense tensor built from its lanes holds `0/0` only where a lane is
    // the quotient of zero by zero.
    let tensor = DenseTensor::from_fractions(vec![num(1), num(9).over_zero()], vec![2])
        .expect("two machine-word lanes");
    assert_eq!(tensor.numerators.as_slice(), [1, 9]);
    assert_eq!(tensor.fraction_or_nil(1).extract_i64_pair(), Some((9, 0)));
    assert_eq!(pair_of(&Value::from_dense_lane(&tensor, 1)), Some((9, 0)));
}

#[test]
fn an_absent_lanes_dividend_is_not_a_number_in_disguise() {
    let value = top("[ 100 5 3 ] [ 0 0 1 ] DIV");
    assert_eq!(value.to_string(), "[ NIL NIL 3/1 ]");
    assert_eq!(
        top("[ 100 5 3 ] [ 0 0 1 ] DIV 0 GET NIL?"),
        Value::from_bool(true)
    );
    assert_eq!(
        top("[ 100 5 3 ] [ 0 0 1 ] DIV 0 GET NIL-REASON"),
        Value::from_string("divisionByZero")
    );
    // Sorting and searching read the lanes as values, not as their pairs.
    assert_eq!(
        top("[ 100 5 3 ] [ 0 0 1 ] DIV 100 MEMBER?"),
        Value::from_bool(false)
    );
}

// ---- Persistence ----

#[test]
fn the_pair_survives_a_save_and_a_restore() {
    let scalar = top("100 0 DIV");
    let dense = top("[ 100 5 3 ] [ 0 0 1 ] DIV");
    let written = top("NIL");
    let encoded = encode_stack([&scalar, &dense, &written].into_iter());
    let restored = decode_stack(&encoded).expect("the stack decodes");
    assert_eq!(restored.len(), 3);
    assert_eq!(restored[0], scalar);
    assert_eq!(pair_of(&restored[0]), Some((100, 0)));
    assert_eq!(
        restored[0].nil_reason().copied(),
        Some(NilReason::DivisionByZero)
    );
    assert_eq!(restored[1], dense);
    assert_eq!(columns(&restored[1]), columns(&dense));
    assert_eq!(restored[2], written);
    assert_eq!(restored[2].absent_pair(), None);
    // A payload written before the dividend was kept decodes as an absence
    // with no pair, which is what it was.
    let old = r#"[{"t":"Nil","r":"divisionByZero"}]"#;
    let restored = decode_stack(old).expect("an older NIL decodes");
    assert!(restored[0].is_nil());
    assert_eq!(restored[0].absent_pair(), None);
    assert_eq!(
        restored[0].nil_reason().copied(),
        Some(NilReason::DivisionByZero)
    );
}

#[test]
fn a_saved_number_never_has_a_zero_denominator() {
    // An absent number is saved as a `Nil` with its dividend, never as a
    // `Scalar`, so a zero denominator there is a malformed payload.
    let payload = r#"[{"t":"Scalar","n":"1","d":"0"}]"#;
    assert!(decode_stack(payload).is_err());
}
