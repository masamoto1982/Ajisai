//! Spine-level arithmetic primitives for the `ADD`/`SUB`/`MUL`/`DIV` family
//! (migration plan §12, Phase 4).
//!
//! A primitive computes only: it receives its operands and returns its results.
//! `interpreter::arithmetic` applies arity and NIL policy before calling one,
//! and the differential tests below pin each primitive against the live
//! executor operand-for-operand.
//!
//! Scope: Phase 4 covers the rational-scalar domain, computed with the same
//! [`Fraction`] arithmetic the legacy executor uses, so results agree by
//! construction (the differential tests confirm this against the live
//! interpreter). Operands outside that domain — exact-real scalars, vectors,
//! tensors — are left to the legacy executor until a later phase widens the
//! primitives; here they yield a reasonless NIL rather than a wrong number.
//! `SQRT` is not covered here: over the rational domain it can still leave
//! `Fraction` (a non-perfect-square root is an exact-real value), so it needs
//! the wider `KernelValue`/`ScalarRepr` surface a later phase adds rather than
//! this module's Fraction-only primitives.

use super::scalar::Scalar;
use super::value::KernelValue;
use crate::error::NilReason;
use crate::types::fraction::Fraction;

fn scalar_fraction(value: &KernelValue) -> Option<Fraction> {
    match value {
        KernelValue::Scalar(scalar) => scalar.as_fraction().cloned(),
        _ => None,
    }
}

/// Apply a binary rational operation to two operands. `op` returns `None` when
/// the operation projects to NIL (division by zero); a non-rational operand
/// yields a reasonless NIL (out of Phase 4 scope).
fn binary(
    operands: &[KernelValue],
    op: impl Fn(&Fraction, &Fraction) -> Option<Fraction>,
) -> Vec<KernelValue> {
    let result = match (operands.first(), operands.get(1)) {
        (Some(a), Some(b)) => match (scalar_fraction(a), scalar_fraction(b)) {
            (Some(a), Some(b)) => match op(&a, &b) {
                Some(value) => KernelValue::Scalar(Scalar::from_fraction(value)),
                None => KernelValue::Nil(Some(NilReason::DivisionByZero)),
            },
            _ => KernelValue::Nil(None),
        },
        _ => KernelValue::Nil(None),
    };
    vec![result]
}

pub fn add(operands: &[KernelValue]) -> Vec<KernelValue> {
    binary(operands, |a, b| Some(a.add(b)))
}

pub fn sub(operands: &[KernelValue]) -> Vec<KernelValue> {
    binary(operands, |a, b| Some(a.sub(b)))
}

pub fn mul(operands: &[KernelValue]) -> Vec<KernelValue> {
    binary(operands, |a, b| Some(a.mul(b)))
}

pub fn div(operands: &[KernelValue]) -> Vec<KernelValue> {
    binary(
        operands,
        |a, b| if b.is_zero() { None } else { Some(a.div(b)) },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A spine primitive, as `interpreter::arithmetic` holds one.
    type Primitive = fn(&[KernelValue]) -> Vec<KernelValue>;

    /// Evaluate an Ajisai program on a fresh interpreter and lower its
    /// top-of-stack value onto the spine.
    async fn eval_top(program: &str) -> KernelValue {
        let mut interp = crate::interpreter::Interpreter::new();
        interp.execute(program).await.expect("program runs");
        assert_eq!(interp.stack.len(), 1, "program leaves one value: {program}");
        KernelValue::from(&interp.stack[0])
    }

    /// The spine primitive and the live executor must agree, operand-for-operand.
    /// Operands are produced by the same interpreter, so the comparison isolates
    /// the Word's computation rather than literal parsing.
    async fn assert_agrees(word: &str, primitive: Primitive, a: &str, b: &str) {
        let operands = [eval_top(a).await, eval_top(b).await];
        let results = primitive(&operands);
        assert_eq!(results.len(), 1);
        let spine = results.into_iter().next().expect("one result");

        let legacy = eval_top(&format!("{a} {b} {word}")).await;
        assert_eq!(
            spine, legacy,
            "spine and legacy disagree on `{a} {word} {b}`"
        );
    }

    #[tokio::test]
    async fn add_matches_the_live_executor() {
        assert_agrees("ADD", add, "3", "4").await;
        assert_agrees("ADD", add, "1/2", "1/3").await;
        assert_agrees("ADD", add, "-5", "2").await;
    }

    #[tokio::test]
    async fn sub_matches_the_live_executor() {
        assert_agrees("SUB", sub, "10", "3").await;
        assert_agrees("SUB", sub, "1/2", "1/3").await;
    }

    #[tokio::test]
    async fn mul_matches_the_live_executor() {
        assert_agrees("MUL", mul, "6", "7").await;
        assert_agrees("MUL", mul, "0.6", "0.8").await;
    }

    #[tokio::test]
    async fn div_matches_the_live_executor() {
        assert_agrees("DIV", div, "20", "4").await;
        assert_agrees("DIV", div, "1", "3").await;
    }

    #[tokio::test]
    async fn div_by_zero_projects_to_the_same_nil() {
        // Both paths project division by zero to NIL(DivisionByZero).
        assert_agrees("DIV", div, "3", "0").await;
        let operands = [eval_top("3").await, eval_top("0").await];
        assert_eq!(
            div(&operands)[0],
            KernelValue::Nil(Some(NilReason::DivisionByZero))
        );
    }

    /// A non-rational operand is out of Phase 4 scope and must project to a
    /// reasonless NIL rather than a wrong number.
    #[test]
    fn non_rational_operands_project_to_a_reasonless_nil() {
        let operands = [
            KernelValue::Boolean(true),
            KernelValue::Scalar(Scalar::from_fraction(Fraction::from(1_i64))),
        ];
        assert_eq!(add(&operands)[0], KernelValue::Nil(None));
    }
}
