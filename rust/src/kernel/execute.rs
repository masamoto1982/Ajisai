//! The shared Word execution wrapper (migration plan §12).
//!
//! Every Word's shared concerns — arity, NIL policy, consumption — are applied
//! here, once, from the [`WordContract`]. A Word's own code (a [`Primitive`])
//! only computes: given its operands, it returns the values to push. This is
//! the entry point the plan converges every Word onto; Phase 4 routes the
//! arithmetic family through it and validates the result against the live
//! executor, without yet rewiring the runtime's dispatch.

use super::value::KernelValue;
use super::word_contract::WordContract;

/// A minimal value stack the wrapper operates on. The runtime's own stack keeps
/// its optimizations; this is the spine-level view a Word sees.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KernelStack {
    items: Vec<KernelValue>,
}

impl KernelStack {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Build a stack from values in bottom-to-top order.
    pub fn from_values(items: Vec<KernelValue>) -> Self {
        Self { items }
    }

    pub fn push(&mut self, value: KernelValue) {
        self.items.push(value);
    }

    pub fn pop(&mut self) -> Option<KernelValue> {
        self.items.pop()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn as_slice(&self) -> &[KernelValue] {
        &self.items
    }
}

/// How the wrapper treats the consumed operands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Consumption {
    /// Consume the operands (the default `EAT` mode).
    Eat,
    /// Keep the operands and append the results (`KEEP`).
    Keep,
}

/// Why a Word could not run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecError {
    /// Fewer operands on the stack than the contract's arity requires.
    StackUnderflow,
}

/// A Word's own computation: given its operands in stack order (bottom → top),
/// return the values to push. The wrapper has already applied arity and NIL
/// policy, so a primitive only computes.
pub type Primitive = fn(&[KernelValue]) -> Vec<KernelValue>;

fn is_nil(value: &KernelValue) -> bool {
    matches!(value, KernelValue::Nil(_))
}

/// Run a Word through the shared wrapper: validate arity, apply the NIL policy,
/// run the primitive if it is not short-circuited, then consume operands and
/// push results per the consumption mode.
pub fn execute_word(
    contract: &WordContract,
    primitive: Primitive,
    consumption: Consumption,
    stack: &mut KernelStack,
) -> Result<(), ExecError> {
    let consumes = contract.arity.consumes as usize;
    if stack.items.len() < consumes {
        return Err(ExecError::StackUnderflow);
    }

    let operand_start = stack.items.len() - consumes;
    let operands = &stack.items[operand_start..];

    // NIL policy: a NIL operand short-circuits the Word to a NIL result,
    // propagating the first NIL's reason, without running the primitive.
    let results = if contract.nil_policy.passes_nil_through() && operands.iter().any(is_nil) {
        let propagated = operands
            .iter()
            .find(|value| is_nil(value))
            .cloned()
            .unwrap_or(KernelValue::Nil(None));
        vec![propagated]
    } else {
        primitive(operands)
    };

    if consumption == Consumption::Eat {
        stack.items.truncate(operand_start);
    }
    stack.items.extend(results);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel::scalar::Scalar;
    use crate::kernel::word_contract::{Arity, NilPolicy};
    use crate::types::fraction::Fraction;

    fn passthrough_contract() -> WordContract {
        WordContract {
            arity: Arity {
                consumes: 2,
                produces: 1,
            },
            nil_policy: NilPolicy::Passthrough,
        }
    }

    fn scalar(n: i64) -> KernelValue {
        KernelValue::Scalar(Scalar::from_fraction(Fraction::from(n)))
    }

    // A primitive that would panic if a NIL operand ever reached it, so the
    // passthrough test proves the wrapper short-circuited before the primitive.
    fn sum_or_panic(operands: &[KernelValue]) -> Vec<KernelValue> {
        let mut total = 0_i64;
        for operand in operands {
            match operand {
                KernelValue::Scalar(s) => {
                    total += s.as_fraction().and_then(Fraction::to_i64).unwrap()
                }
                other => panic!("primitive received a non-scalar operand: {other:?}"),
            }
        }
        vec![scalar(total)]
    }

    #[test]
    fn eat_consumes_operands_and_pushes_the_result() {
        let mut stack = KernelStack::from_values(vec![scalar(2), scalar(3)]);
        execute_word(
            &passthrough_contract(),
            sum_or_panic,
            Consumption::Eat,
            &mut stack,
        )
        .unwrap();
        assert_eq!(stack.as_slice(), &[scalar(5)]);
    }

    #[test]
    fn keep_preserves_operands_and_appends_the_result() {
        let mut stack = KernelStack::from_values(vec![scalar(2), scalar(3)]);
        execute_word(
            &passthrough_contract(),
            sum_or_panic,
            Consumption::Keep,
            &mut stack,
        )
        .unwrap();
        assert_eq!(stack.as_slice(), &[scalar(2), scalar(3), scalar(5)]);
    }

    #[test]
    fn a_nil_operand_short_circuits_before_the_primitive() {
        let mut stack = KernelStack::from_values(vec![scalar(2), KernelValue::Nil(None)]);
        // sum_or_panic would panic on the NIL; reaching here proves passthrough
        // ran instead of the primitive.
        execute_word(
            &passthrough_contract(),
            sum_or_panic,
            Consumption::Eat,
            &mut stack,
        )
        .unwrap();
        assert_eq!(stack.as_slice(), &[KernelValue::Nil(None)]);
    }

    #[test]
    fn too_few_operands_is_an_underflow() {
        let mut stack = KernelStack::from_values(vec![scalar(2)]);
        let result = execute_word(
            &passthrough_contract(),
            sum_or_panic,
            Consumption::Eat,
            &mut stack,
        );
        assert_eq!(result, Err(ExecError::StackUnderflow));
    }
}
