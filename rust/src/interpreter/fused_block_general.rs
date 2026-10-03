//! The general tier of a fused walk (`fused_block`): any plain rationals and
//! Booleans, each op charged as the interpreted route charges it.
//!
//! What each op costs on the interpreted route, besides its one step:
//!
//! - `ADD`/`SUB`/`MUL`/`DIV` on two scalars take the scalar fast path, so a
//!   fastpath hit and `binary_numeric_work` of the operands' widths, charged
//!   before the operation runs; the result is held to the size ceiling.
//! - `LT`/`GT` on two scalars take the comparison fast path: a fastpath hit
//!   and no work. `EQ` takes it only for two scalars; any other pair is
//!   decided by `pairwise_eq`, which counts nothing.
//! - `FLOOR`, `ROUND`, `NOT`, `AND`, `SELECT` and `BIND` charge nothing else.
//!
//! An operand outside the domain a Word accepts is that Word's ERROR, and a
//! zero divisor its NIL projection; both answer `None` here, for the ordinary
//! walk to reproduce with its diagnostics.

use crate::interpreter::fused_block::{Charges, Compare, FusedBlock, FusedWalk, Op, Plain};
use crate::interpreter::runtime_limits::{
    binary_numeric_work, fraction_result_bits, fraction_work_bits,
};
use crate::interpreter::Interpreter;
use crate::types::fraction::Fraction;
use crate::types::{DenseTensor, Value, ValueData};
use std::sync::Arc;

/// Every element of `target` as a plain value, or `None` if any is not.
///
/// A one-dimensional dense Tensor — what `RANGE` and every arithmetic Word
/// over a Vector build — is read straight from its columns; `Value::child`
/// would box each lane as a `Value` only for this to unbox it again.
fn elements(target: &Value) -> Option<Vec<Plain>> {
    if let ValueData::Tensor { data, shape } = &target.data {
        if shape.len() == 1 {
            return (0..data.len())
                .map(|i| data.get_small_fraction(i).map(Plain::Num))
                .collect();
        }
    }
    (0..target.len())
        .map(|i| target.child(i).and_then(|v| Plain::of(&v)))
        .collect()
}

/// `Value::from_vector_promoted` of plain values. A non-empty run of
/// rationals that each fit a machine word is the dense Tensor promotion
/// builds for it, built from the rationals directly; anything else goes
/// through `from_vector_promoted` itself.
pub(crate) fn promote(values: Vec<Plain>) -> Value {
    let fits = |p: &Plain| matches!(p, Plain::Num(f) if f.extract_i64_pair().is_some());
    if !values.is_empty() && values.iter().all(fits) {
        let shape = vec![values.len()];
        let fractions: Vec<Fraction> = values
            .into_iter()
            .map(|p| match p {
                Plain::Num(f) => f,
                Plain::Bool(_) => unreachable!("checked above"),
            })
            .collect();
        let tensor =
            DenseTensor::from_fractions(fractions, shape.clone()).expect("every lane fits");
        return Value::new(
            ValueData::Tensor {
                data: Arc::new(tensor),
                shape: Arc::new(shape),
            },
            None,
        );
    }
    Value::from_vector_promoted(values.into_iter().map(Plain::into_value).collect())
}

struct Meter<'a> {
    interp: &'a Interpreter,
    work: u64,
    work_budget: u64,
    fastpath: u64,
}

impl Meter<'_> {
    fn arith(&mut self, op: Op, a: Plain, b: Plain) -> Option<Plain> {
        let (Op::Arith(schema), Plain::Num(a), Plain::Num(b)) = (op, a, b) else {
            return None;
        };
        self.work = self.work.saturating_add(binary_numeric_work(
            fraction_work_bits(&a),
            fraction_work_bits(&b),
        ));
        if self.work > self.work_budget {
            return None;
        }
        let r = schema.fraction(&a, &b).ok()?;
        self.interp
            .runtime_limits
            .check_algebraic_size(0, fraction_result_bits(&r))
            .ok()?;
        self.fastpath += 1;
        Some(Plain::Num(r))
    }

    fn compare(&mut self, kind: Compare, a: Plain, b: Plain) -> Option<Plain> {
        Some(Plain::Bool(match (kind, a, b) {
            (Compare::Lt, Plain::Num(a), Plain::Num(b)) => {
                self.fastpath += 1;
                a.lt(&b)
            }
            (Compare::Gt, Plain::Num(a), Plain::Num(b)) => {
                self.fastpath += 1;
                a.gt(&b)
            }
            (Compare::Eq, Plain::Num(a), Plain::Num(b)) => {
                self.fastpath += 1;
                a == b
            }
            (Compare::Eq, a, b) => a == b,
            (Compare::Lt | Compare::Gt, _, _) => return None,
        }))
    }
}

/// Run the block once on `stack`, answering its top value.
fn run_block(
    block: &FusedBlock,
    meter: &mut Meter,
    stack: &mut Vec<Plain>,
    slots: &mut [Option<Plain>],
) -> Option<Plain> {
    for op in &block.ops {
        let value = match op {
            Op::Push(p) => p.clone(),
            Op::PushWord(b) => Plain::Bool(*b),
            Op::Load(slot) => slots[*slot].clone()?,
            Op::Bind(slot) => {
                slots[*slot] = Some(stack.pop()?);
                continue;
            }
            Op::Floor | Op::Round | Op::Not => match (op, stack.pop()?) {
                (Op::Floor, Plain::Num(f)) => Plain::Num(f.floor()),
                (Op::Round, Plain::Num(f)) => Plain::Num(f.round()),
                (Op::Not, Plain::Bool(b)) => Plain::Bool(!b),
                _ => return None,
            },
            Op::Select => {
                let mask = stack.pop()?;
                let when_false = stack.pop()?;
                let when_true = stack.pop()?;
                match mask {
                    Plain::Bool(true) => when_true,
                    Plain::Bool(false) => when_false,
                    Plain::Num(_) => return None,
                }
            }
            Op::Extremum { max } => {
                let b = stack.pop()?;
                let a = stack.pop()?;
                let (Plain::Num(x), Plain::Num(y)) = (&a, &b) else {
                    return None;
                };
                // The left operand on a tie, as MIN and MAX keep it.
                let take_right = if *max { x.lt(y) } else { y.lt(x) };
                if take_right {
                    b
                } else {
                    a
                }
            }
            Op::Arith(_) | Op::Compare(_) | Op::And => {
                let b = stack.pop()?;
                let a = stack.pop()?;
                match op {
                    Op::Arith(_) => meter.arith(op.clone(), a, b)?,
                    Op::Compare(kind) => meter.compare(*kind, a, b)?,
                    _ => match (a, b) {
                        (Plain::Bool(a), Plain::Bool(b)) => Plain::Bool(a && b),
                        _ => return None,
                    },
                }
            }
        };
        stack.push(value);
    }
    stack.pop()
}

pub(crate) fn run(
    block: &FusedBlock,
    interp: &Interpreter,
    walk: FusedWalk,
    target: &Value,
    seed: Option<&Plain>,
) -> Option<(Value, Charges)> {
    let elements = elements(target)?;
    let runs = elements.len() as u64;
    let steps = block.steps_within_ceiling(interp, runs)?;
    let mut meter = Meter {
        interp,
        work: 0,
        work_budget: interp
            .runtime_limits
            .max_numeric_work
            .checked_sub(interp.numeric_work_used)?,
        fastpath: 0,
    };

    let mut accumulator = seed.cloned();
    let mut results: Vec<Plain> = Vec::new();
    let mut stack: Vec<Plain> = Vec::with_capacity(block.ops.len() + 2);
    let mut slots: Vec<Option<Plain>> = vec![None; block.slots];
    for element in elements {
        stack.clear();
        if let Some(acc) = accumulator.take() {
            stack.push(acc);
        }
        stack.push(element.clone());
        let result = run_block(block, &mut meter, &mut stack, &mut slots)?;
        match walk {
            FusedWalk::Map => results.push(result),
            FusedWalk::Filter => match result {
                Plain::Bool(true) => results.push(element),
                Plain::Bool(false) => {}
                // A predicate must decide in the truth domain.
                Plain::Num(_) => return None,
            },
            FusedWalk::Scan => {
                results.push(result.clone());
                accumulator = Some(result);
            }
            FusedWalk::Fold => accumulator = Some(result),
        }
    }

    let value = match walk {
        FusedWalk::Map | FusedWalk::Filter => promote(results),
        FusedWalk::Scan => Value::from_vector(results.into_iter().map(Plain::into_value).collect()),
        FusedWalk::Fold => accumulator?.into_value(),
    };
    let charges = Charges {
        runs,
        steps,
        work: meter.work,
        fastpath: meter.fastpath,
    };
    Some((value, charges))
}
