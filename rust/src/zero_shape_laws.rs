//! LANG.VALUES.SHAPE, held against the implementation.
//!
//! A number read as whether its numerator and its denominator are nonzero
//! takes one of four zero shapes — `0` (F), a point `±1/0` (T), `0/0` (N) and
//! every other number (B) — and the arithmetic is graded by them. The clause
//! states the laws that follow from treating a number as a pair to the end;
//! each one is checked here by running programs, never by consulting the
//! implementation's own representation:
//!
//! 1. the shape of a product is the meet of the operands' shapes (FOUR's
//!    knowledge meet ⊗), and the reciprocal exchanges F and T as `NOT` does;
//! 2. each shape is a group under `MUL` — the field's units, `{1/0, -1/0}`
//!    with `1/0` as identity, `{0}` and `{0/0}` — so the numbers form a
//!    Clifford monoid over FOUR;
//! 3. the shape of a sum is the Boolean-fraction sum `(ad ∨ bc, bd)`,
//!    exact except where two B operands cancel to `0`;
//! 4. the wheel axioms that involve addition hold at every point;
//! 5. the reciprocal is an involution everywhere but `-1/0`, and respects
//!    products everywhere but where a zero meets a negative finite factor —
//!    both because infinity is signed and zero is not;
//! 6. the distributive law fails only with a point `±1/0` as the multiplier;
//! 7. the mediant, written from `RATIO`, `ADD`, `GET` and `DIV`, is
//!    `RECONCILE`'s join on shapes for operands of one sign, has `0/0` as its
//!    unit, grows the Stern–Brocot tree from `0` and `1/0`, and is not
//!    associative on values.

use crate::interpreter::Interpreter;
use crate::types::{Value, ValueData};
use num_traits::Zero;

/// Every zero shape, both signs, a proper fraction and an irrational.
const SAMPLE: &[&str] = &["0", "1", "-2", "1/2", "2 SQRT", "1/0", "-1/0", "0/0"];

/// The rational part of the sample: `RATIO` reads only rationals.
const RATIONAL_SAMPLE: &[&str] = &["0", "1", "-2", "1/2", "1/0", "-1/0", "0/0"];

const MEDIANT: &str = "[ RATIO 'B' BIND RATIO B ADD 'P' BIND P 0 GET P 1 GET DIV ] 'MEDIANT' DEF ";

/// `(numerator ≠ 0, denominator ≠ 0)`: F = (false, true), T = (true, false),
/// N = (false, false), B = (true, true).
type Shape = (bool, bool);

const F: Shape = (false, true);
const T: Shape = (true, false);
const N: Shape = (false, false);
const B: Shape = (true, true);

fn meet(a: Shape, b: Shape) -> Shape {
    (a.0 && b.0, a.1 && b.1)
}

fn join(a: Shape, b: Shape) -> Shape {
    (a.0 || b.0, a.1 || b.1)
}

fn swap(a: Shape) -> Shape {
    (a.1, a.0)
}

/// The pair sum read over the Boolean semiring.
fn boolean_sum(a: Shape, b: Shape) -> Shape {
    ((a.0 && b.1) || (a.1 && b.0), a.1 && b.1)
}

fn shape_of(value: &Value) -> Shape {
    match &value.data {
        ValueData::Scalar(f) => (!f.numerator().is_zero(), f.is_finite()),
        // An irrational is a nonzero finite number.
        ValueData::ExactScalar(_) => B,
        other => panic!("expected a number, got {other:?}"),
    }
}

async fn run(code: &str) -> Vec<Value> {
    let mut interp = Interpreter::new();
    interp
        .execute(code)
        .await
        .unwrap_or_else(|e| panic!("`{code}` must run: {e:?}"));
    interp.get_stack().to_vec()
}

async fn number(code: &str) -> Value {
    let stack = run(code).await;
    assert_eq!(stack.len(), 1, "`{code}` must leave one value");
    stack.into_iter().next().unwrap()
}

async fn holds(code: &str) -> bool {
    match run(code).await.as_slice() {
        [v] => matches!(v.data, ValueData::Boolean(true)),
        other => panic!("`{code}` must leave one truth value, left {other:?}"),
    }
}

async fn shape(code: &str) -> Shape {
    shape_of(&number(code).await)
}

#[tokio::test]
async fn product_shape_is_the_knowledge_meet_and_reciprocal_is_not() {
    for x in SAMPLE {
        let sx = shape(x).await;
        assert_eq!(
            shape(&format!("1 {x} DIV")).await,
            swap(sx),
            "the reciprocal of {x} exchanges its halves' shape, as NOT does"
        );
        for y in SAMPLE {
            let sy = shape(y).await;
            assert_eq!(
                shape(&format!("{x} {y} MUL")).await,
                meet(sx, sy),
                "shape({x} · {y}) is the meet of the operands' shapes"
            );
        }
    }
}

#[tokio::test]
async fn each_shape_is_a_group_under_mul() {
    // T: {1/0, -1/0} is Z/2 with 1/0 as identity.
    assert_eq!(shape("1/0").await, T);
    assert_eq!(shape("-1/0").await, T);
    for (a, b, product) in [
        ("1/0", "1/0", "1/0"),
        ("1/0", "-1/0", "-1/0"),
        ("-1/0", "-1/0", "1/0"),
    ] {
        assert!(
            holds(&format!("{a} {b} MUL {product} EQ")).await,
            "{a} · {b} = {product}"
        );
    }
    // F and N are trivial groups, idempotent under MUL.
    assert!(holds("0 0 MUL 0 EQ").await);
    assert!(holds("0/0 0/0 MUL 0/0 EQ").await);
    // B is the field's multiplicative group: closed, with inverses.
    for x in ["1", "-2", "1/2", "2 SQRT"] {
        assert!(
            holds(&format!("{x} 1 {x} DIV MUL 1 EQ")).await,
            "{x} has an inverse in B"
        );
    }
    // A shape's identity acts on a lower shape as the meet says: B's identity
    // fixes every number, T's sends B to T by sign.
    assert!(holds("2 1/0 MUL 1/0 EQ").await);
    assert!(holds("-2 1/0 MUL -1/0 EQ").await);
}

#[tokio::test]
async fn sum_shape_is_the_boolean_fraction_sum_except_for_cancellation() {
    for x in SAMPLE {
        let sx = shape(x).await;
        for y in SAMPLE {
            let sy = shape(y).await;
            let actual = shape(&format!("{x} {y} ADD")).await;
            if sx == B && sy == B {
                assert!(
                    actual == B || actual == F,
                    "two finite nonzero numbers sum to B or, by cancellation, F: {x} + {y}"
                );
            } else {
                assert_eq!(actual, boolean_sum(sx, sy), "shape({x} + {y})");
            }
        }
    }
    // The table's one corner a field never reaches: T + T is N, whatever the signs.
    assert_eq!(shape("1/0 1/0 ADD").await, N);
    assert_eq!(shape("1/0 -1/0 ADD").await, N);
    assert_eq!(shape("1 -1 ADD").await, F);
}

#[tokio::test]
async fn the_additive_wheel_axioms_hold_at_every_point() {
    for x in SAMPLE {
        for y in SAMPLE {
            for z in SAMPLE {
                let w4 =
                    format!("{x} {z} MUL {y} {z} MUL ADD  {x} {y} ADD {z} MUL 0 {z} MUL ADD EQ");
                let w5 =
                    format!("{x} {y} {z} MUL ADD {y} DIV  {x} {y} DIV {z} ADD 0 {y} MUL ADD EQ");
                let w7 = format!("{x} 0 {y} MUL ADD {z} MUL  {x} {z} MUL 0 {y} MUL ADD EQ");
                let w8 = format!("1 {x} 0 {y} MUL ADD DIV  1 {x} DIV 0 {y} MUL ADD EQ");
                for (name, law) in [("W4", w4), ("W5", w5), ("W7", w7), ("W8", w8)] {
                    assert!(holds(&law).await, "{name} must hold at x={x}, y={y}, z={z}");
                }
            }
        }
    }
}

#[tokio::test]
async fn the_reciprocal_is_an_involution_except_at_negative_infinity() {
    for x in SAMPLE {
        let involutive = holds(&format!("1 1 {x} DIV DIV {x} EQ")).await;
        assert_eq!(
            involutive,
            *x != "-1/0",
            "1/(1/x) = x exactly when x is not -1/0 ({x})"
        );
    }
    // Zero has no sign to give back: the reciprocal of -1/0 is 0, and of 0 is 1/0.
    assert!(holds("1 1 -1/0 DIV DIV 1/0 EQ").await);
}

#[tokio::test]
async fn the_reciprocal_respects_products_except_where_zero_meets_a_negative() {
    let negative_finite = ["-2"];
    for x in SAMPLE {
        for y in SAMPLE {
            let law = holds(&format!("1 {x} {y} MUL DIV  1 {x} DIV 1 {y} DIV MUL EQ")).await;
            let expected_failure = (*x == "0" && negative_finite.contains(y))
                || (*y == "0" && negative_finite.contains(x));
            assert_eq!(
                law, !expected_failure,
                "1/(xy) = (1/x)(1/y) at x={x}, y={y}"
            );
        }
    }
}

#[tokio::test]
async fn distribution_fails_only_under_a_signed_point() {
    let mut failures = 0;
    for x in SAMPLE {
        for y in SAMPLE {
            for z in SAMPLE {
                let law = format!("{x} {y} {z} ADD MUL  {x} {y} MUL {x} {z} MUL ADD EQ");
                if !holds(&law).await {
                    failures += 1;
                    assert!(
                        *x == "1/0" || *x == "-1/0",
                        "x(y+z) = xy+xz may fail only with x = ±1/0, failed at x={x}, y={y}, z={z}"
                    );
                }
            }
        }
    }
    assert!(failures > 0, "the distributive law does give way at ±1/0");
}

#[tokio::test]
async fn the_mediant_is_the_join_on_shapes_for_operands_of_one_sign() {
    let non_negative = ["0", "1", "1/2", "1/0", "0/0"];
    let non_positive = ["0", "-2", "-1/0", "0/0"];
    for side in [&non_negative[..], &non_positive[..]] {
        for x in side {
            for y in side {
                let mediant = shape(&format!("{MEDIANT}{x} {y} MEDIANT")).await;
                assert_eq!(mediant, join(shape(x).await, shape(y).await), "{x} ⊕ {y}");
            }
        }
    }
    // 0/0 is the unit, as UNKNOWN is RECONCILE's.
    for x in RATIONAL_SAMPLE {
        assert!(
            holds(&format!("{MEDIANT}0/0 {x} MEDIANT {x} EQ")).await,
            "0/0 ⊕ {x} = {x}"
        );
    }
    // FALSE ⊕ TRUE is BOTH, and the number it is is 1.
    assert!(holds(&format!("{MEDIANT}0 1/0 MEDIANT 1 EQ")).await);
    assert!(holds(&format!("{MEDIANT}-1/0 0 MEDIANT -1 EQ")).await);
}

#[tokio::test]
async fn the_mediant_grows_every_positive_rational_from_false_and_true() {
    // Stern–Brocot: in-order, each level inserts the mediant of its neighbours.
    let mut row: Vec<String> = vec!["0".into(), "1/0".into()];
    for _ in 0..5 {
        let mut next = Vec::with_capacity(row.len() * 2);
        for pair in row.windows(2) {
            next.push(pair[0].clone());
            let m = number(&format!("{MEDIANT}{} {} MEDIANT", pair[0], pair[1])).await;
            next.push(m.to_string());
        }
        next.push(row.last().unwrap().clone());
        row = next;
    }
    // Strictly increasing, so every number appears once.
    for pair in row.windows(2) {
        assert!(
            holds(&format!("{} {} LT", pair[0], pair[1])).await,
            "{} < {}",
            pair[0],
            pair[1]
        );
    }
    // Every reduced p/q with 1 ≤ p, q ≤ 4 has appeared within five levels.
    for p in 1..=4i64 {
        for q in 1..=4i64 {
            if num_integer::Integer::gcd(&p, &q) != 1 {
                continue;
            }
            let mut found = false;
            for item in &row {
                if holds(&format!("{item} {p}/{q} EQ")).await {
                    found = true;
                    break;
                }
            }
            assert!(found, "{p}/{q} appears in the Stern–Brocot tree");
        }
    }
}

#[tokio::test]
async fn the_mediant_is_not_associative_on_values() {
    // RECONCILE folds in any order; the mediant remembers the order.
    assert!(holds(&format!("{MEDIANT}1 1 MEDIANT 1/2 MEDIANT 2/3 EQ")).await);
    assert!(holds(&format!("{MEDIANT}1 1 1/2 MEDIANT MEDIANT 3/4 EQ")).await);
    // On shapes the two agree, as the join is associative.
    assert_eq!(
        shape(&format!("{MEDIANT}1 1 MEDIANT 1/2 MEDIANT")).await,
        shape(&format!("{MEDIANT}1 1 1/2 MEDIANT MEDIANT")).await
    );
}
