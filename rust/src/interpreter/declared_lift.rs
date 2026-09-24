//! Lifting a Word over the Vectors and Records in its `leaf` and `truth`
//! operands (LANG.COLLECTIONS.LIFT).
//!
//! A `leaf` operand is read as one Scalar, String or Boolean, so a container
//! there means "apply the Word to each element". The arithmetic, comparison
//! and logic families lift natively (the tensor path and `lane_lift`);
//! every other Word with a lifted operand is lifted here, through the same
//! `lift_lanes_dyn`, so one rule decides how `[ 'a' 'b' ] UPPER`,
//! `[ 1 2 ] 10 ADD` and `R [ 'x' 'y' ] AT` combine their elements. Each
//! element runs through the full dispatcher, so the NIL roles, the declared
//! conditions and the cost charges are exactly the Word's own.

use crate::error::Result;
use crate::kernel::generated::{Arity, GeneratedWord, OperandRole};
use crate::types::Value;

use super::lane_lift::lift_lanes_dyn;
use super::Interpreter;

fn is_container(value: &Value) -> bool {
    value.is_vector() || value.as_record().is_some()
}

impl Interpreter {
    /// Lift the Word over its operands when a `leaf` or `truth` operand holds
    /// a Vector or Record and the Word does not lift natively. `None` means
    /// there is nothing to lift here and the primitive runs.
    pub(super) fn apply_declared_lift(
        &mut self,
        word: &'static GeneratedWord,
    ) -> Option<Result<()>> {
        if word.lifts_natively {
            return None;
        }
        let Arity::Fixed(arity) = word.stack_inputs else {
            return None;
        };
        let arity = arity as usize;
        if arity == 0 || self.stack.len() < arity {
            return None;
        }
        let lifted: Vec<bool> = word
            .operand_roles
            .iter()
            .map(|role| matches!(role, OperandRole::Leaf | OperandRole::Truth))
            .collect();
        let window = &self.stack.as_slice()[self.stack.len() - arity..];
        if !window
            .iter()
            .zip(&lifted)
            .any(|(operand, lifts)| *lifts && is_container(operand))
        {
            return None;
        }

        let start = self.stack.len() - arity;
        let operands: Vec<Value> = self.stack.drain(start..).collect();
        let refs: Vec<&Value> = operands.iter().collect();
        let mut element = |lane: &[&Value]| -> Result<Value> {
            let base = self.stack.len();
            for operand in lane {
                self.stack.push((*operand).clone());
            }
            match self.execute_generated_word(word) {
                Ok(()) => {
                    // Every lifted Word answers exactly one value
                    // (word-schema:check), so the lane leaves one result.
                    assert_eq!(
                        self.stack.len(),
                        base + 1,
                        "{} answers one value",
                        word.name
                    );
                    Ok(self.stack.pop().expect("the result just asserted"))
                }
                Err(e) => {
                    self.stack.truncate(base);
                    Err(e)
                }
            }
        };
        let outcome = lift_lanes_dyn(&refs, &lifted, &mut element);
        Some(match outcome {
            Ok(result) => {
                self.stack.push(result);
                Ok(())
            }
            Err(e) => {
                self.stack.extend(operands);
                Err(e)
            }
        })
    }
}
