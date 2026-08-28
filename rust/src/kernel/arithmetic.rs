//! Spine-level arithmetic primitives for the `ADD`/`SUB`/`MUL`/`DIV`/`FLOOR`/`NEG`
//! family (migration plan §12, Phase 4).
//!
//! These are the first Words routed through the shared [`execute_word`] wrapper.
//! A primitive computes only: the wrapper has already applied arity and NIL
//! policy, so a primitive receives its operands and returns its results.
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
//!
//! [`execute_word`]: super::execute::execute_word

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

/// Apply a unary rational operation. A non-rational operand yields a
/// reasonless NIL (out of Phase 4 scope), same as [`binary`].
fn unary(operands: &[KernelValue], op: impl Fn(&Fraction) -> Fraction) -> Vec<KernelValue> {
    let result = match operands.first().and_then(scalar_fraction) {
        Some(a) => KernelValue::Scalar(Scalar::from_fraction(op(&a))),
        None => KernelValue::Nil(None),
    };
    vec![result]
}

pub fn floor(operands: &[KernelValue]) -> Vec<KernelValue> {
    unary(operands, Fraction::floor)
}

pub fn neg(operands: &[KernelValue]) -> Vec<KernelValue> {
    unary(operands, |a| Fraction::from(0_i64).sub(a))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::execute::{execute_word, Consumption, KernelStack, Primitive};
    use crate::kernel::generated::GENERATED_WORDS;
    use crate::kernel::word_contract::WordContract;

    fn contract(name: &str) -> WordContract {
        let word = GENERATED_WORDS
            .iter()
            .find(|word| word.name == name)
            .expect("word present in generated registry");
        WordContract::from_generated(word).expect("arithmetic Words are fixed-arity")
    }

    /// Evaluate an Ajisai program on a fresh interpreter and lower its
    /// top-of-stack value onto the spine.
    async fn eval_top(program: &str) -> KernelValue {
        let mut interp = crate::interpreter::Interpreter::new();
        interp.execute(program).await.expect("program runs");
        assert_eq!(interp.stack.len(), 1, "program leaves one value: {program}");
        KernelValue::from(&interp.stack[0])
    }

    /// The spine wrapper and the live executor must agree, operand-for-operand.
    /// Operands are produced by the same interpreter, so the comparison isolates
    /// the Word's computation rather than literal parsing.
    async fn assert_agrees(word: &str, primitive: Primitive, a: &str, b: &str) {
        let operands = vec![eval_top(a).await, eval_top(b).await];
        let mut stack = KernelStack::from_values(operands);
        execute_word(&contract(word), primitive, Consumption::Eat, &mut stack).unwrap();
        assert_eq!(stack.len(), 1);
        let spine = stack.as_slice()[0].clone();

        let legacy = eval_top(&format!("{a} {b} {word}")).await;
        assert_eq!(
            spine, legacy,
            "spine and legacy disagree on `{a} {word} {b}`"
        );
    }

    /// The unary counterpart of [`assert_agrees`].
    async fn assert_agrees_unary(word: &str, primitive: Primitive, a: &str) {
        let operands = vec![eval_top(a).await];
        let mut stack = KernelStack::from_values(operands);
        execute_word(&contract(word), primitive, Consumption::Eat, &mut stack).unwrap();
        assert_eq!(stack.len(), 1);
        let spine = stack.as_slice()[0].clone();

        let legacy = eval_top(&format!("{a} {word}")).await;
        assert_eq!(spine, legacy, "spine and legacy disagree on `{a} {word}`");
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
        let mut stack = KernelStack::from_values(vec![eval_top("3").await, eval_top("0").await]);
        execute_word(&contract("DIV"), div, Consumption::Eat, &mut stack).unwrap();
        assert_eq!(
            stack.as_slice()[0],
            KernelValue::Nil(Some(NilReason::DivisionByZero))
        );
    }

    #[tokio::test]
    async fn floor_matches_the_live_executor() {
        assert_agrees_unary("FLOOR", floor, "7").await;
        assert_agrees_unary("FLOOR", floor, "7/2").await;
        assert_agrees_unary("FLOOR", floor, "-7/2").await;
        assert_agrees_unary("FLOOR", floor, "-3").await;
    }

    #[tokio::test]
    async fn neg_matches_the_live_executor() {
        assert_agrees_unary("NEG", neg, "5").await;
        assert_agrees_unary("NEG", neg, "-5").await;
        assert_agrees_unary("NEG", neg, "1/3").await;
        assert_agrees_unary("NEG", neg, "0").await;
    }
}
