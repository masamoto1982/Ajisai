//! The integer tier of a fused walk (`fused_block`): every value a
//! machine-word integer or a Boolean.
//!
//! A block is straight-line, so once the types of the elements and the seed
//! are known, the type of every value it makes is too. The block is checked
//! against them before the walk starts — `typed` — and every op that would
//! leave this tier (an operand of the wrong domain, a `DIV` that makes a
//! fraction) sends the walk to the general tier before anything has run. What
//! remains runs on `i64`, a Boolean as 0 or 1, with checked arithmetic: an
//! overflow is the one thing left to discover while running, and it, too,
//! falls to the general tier.
//!
//! The same typing settles what each run is charged, so the whole walk's
//! charges are known up front: every operand here is a `Small` fraction,
//! which the meter prices at one limb, so each arithmetic Word costs
//! `binary_numeric_work(1, 1)` and takes the scalar fast path, as `LT`/`GT`
//! and a scalar `EQ` do. Every result fits 64 bits, so the size ceiling can
//! refuse one only when it is set below that, and then this tier steps aside.
//!
//! `DIV` followed directly by `FLOOR` is floor division on integers — `a b
//! DIV FLOOR`, the quotient half of `a - b * floor(a / b)` — and runs as one,
//! still charged as the two Words it is. A zero divisor is the ordinary
//! walk's NIL to project.

use crate::interpreter::arithmetic::ExactArithmeticSchema;
use crate::interpreter::fused_block::{Charges, Compare, FusedBlock, FusedWalk, Op, Plain};
use crate::interpreter::fused_block_general::promote;
use crate::interpreter::runtime_limits::binary_numeric_work;
use crate::interpreter::Interpreter;
use crate::types::fraction::{Fraction, FractionRepr};
use crate::types::{DenseTensor, Value, ValueData};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ty {
    Int,
    Bool,
}

#[derive(Debug, Clone, Copy)]
enum IntOp {
    Push(i64),
    Load(usize),
    Store(usize),
    Add,
    Sub,
    Mul,
    FloorDiv,
    Lt,
    Gt,
    Eq,
    Not,
    And,
    Select,
}

/// The block checked against `inputs`, with what one run charges.
struct Typed {
    ops: Vec<IntOp>,
    out: Ty,
    fastpath_per_run: u64,
    work_per_run: u64,
}

fn small_integer(f: &Fraction) -> Option<i64> {
    match f.repr {
        FractionRepr::Small(n, 1) => Some(n),
        _ => None,
    }
}

fn plain_int(p: &Plain) -> Option<(Ty, i64)> {
    match p {
        Plain::Num(f) => Some((Ty::Int, small_integer(f)?)),
        Plain::Bool(b) => Some((Ty::Bool, i64::from(*b))),
    }
}

fn typed(block: &FusedBlock, inputs: &[Ty]) -> Option<Typed> {
    let mut stack: Vec<Ty> = inputs.to_vec();
    let mut slots: Vec<Option<Ty>> = vec![None; block.slots];
    let mut ops = Vec::with_capacity(block.ops.len());
    let (mut fastpath, mut work) = (0u64, 0u64);
    let pair = |stack: &mut Vec<Ty>| Some((stack.pop()?, stack.pop()?));
    let mut i = 0;
    while i < block.ops.len() {
        let (op, ty) = match &block.ops[i] {
            Op::Push(p) => {
                let (ty, n) = plain_int(p)?;
                (IntOp::Push(n), ty)
            }
            Op::PushWord(b) => (IntOp::Push(i64::from(*b)), Ty::Bool),
            Op::Load(slot) => (IntOp::Load(*slot), slots[*slot]?),
            Op::Bind(slot) => {
                slots[*slot] = Some(stack.pop()?);
                ops.push(IntOp::Store(*slot));
                i += 1;
                continue;
            }
            Op::Floor | Op::Round => {
                // The floor and nearest integer of an integer are itself.
                (stack.pop()? == Ty::Int).then_some(())?;
                stack.push(Ty::Int);
                i += 1;
                continue;
            }
            Op::Not => {
                (stack.pop()? == Ty::Bool).then_some(())?;
                (IntOp::Not, Ty::Bool)
            }
            Op::And => {
                (pair(&mut stack)? == (Ty::Bool, Ty::Bool)).then_some(())?;
                (IntOp::And, Ty::Bool)
            }
            Op::Select => {
                let mask = stack.pop()?;
                let (when_false, when_true) = pair(&mut stack)?;
                (mask == Ty::Bool && when_true == when_false).then_some(())?;
                (IntOp::Select, when_true)
            }
            Op::Compare(kind) => {
                let (b, a) = pair(&mut stack)?;
                let op = match (kind, a, b) {
                    (Compare::Lt, Ty::Int, Ty::Int) => IntOp::Lt,
                    (Compare::Gt, Ty::Int, Ty::Int) => IntOp::Gt,
                    (Compare::Eq, Ty::Int, Ty::Int) => IntOp::Eq,
                    // Two Booleans compare by `pairwise_eq`, off the fast path.
                    (Compare::Eq, Ty::Bool, Ty::Bool) => {
                        stack.push(Ty::Bool);
                        ops.push(IntOp::Eq);
                        i += 1;
                        continue;
                    }
                    _ => return None,
                };
                fastpath += 1;
                (op, Ty::Bool)
            }
            Op::Arith(schema) => {
                (pair(&mut stack)? == (Ty::Int, Ty::Int)).then_some(())?;
                fastpath += 1;
                work += binary_numeric_work(1, 1);
                let op = match schema {
                    ExactArithmeticSchema::Add => IntOp::Add,
                    ExactArithmeticSchema::Sub => IntOp::Sub,
                    ExactArithmeticSchema::Mul => IntOp::Mul,
                    ExactArithmeticSchema::Div => {
                        // Only as floor division: the `FLOOR` is consumed
                        // here, and costs its step as the `steps_per_run`
                        // count already has it.
                        if !matches!(block.ops.get(i + 1), Some(Op::Floor)) {
                            return None;
                        }
                        i += 1;
                        IntOp::FloorDiv
                    }
                };
                (op, Ty::Int)
            }
        };
        ops.push(op);
        stack.push(ty);
        i += 1;
    }
    Some(Typed {
        ops,
        out: *stack.last()?,
        fastpath_per_run: fastpath,
        work_per_run: work,
    })
}

/// `floor(a / b)`, or `None` for a zero divisor or `i64::MIN / -1`.
fn floor_div(a: i64, b: i64) -> Option<i64> {
    let q = a.checked_div(b)?;
    Some(if a % b != 0 && ((a < 0) != (b < 0)) {
        q - 1
    } else {
        q
    })
}

/// Every element as `(type, i64)`, borrowed in place from a pure-integer
/// dense Tensor's numerator column, or `None`. `is_pure_integer` is false for
/// a Tensor with an absent lane (its denominator is 0), so a column it
/// vouches for holds no NIL.
fn elements(target: &Value) -> Option<(Ty, Cow<'_, [i64]>)> {
    match &target.data {
        ValueData::Tensor { data, shape } if shape.len() == 1 && data.is_pure_integer => {
            Some((Ty::Int, Cow::Borrowed(&data.numerators)))
        }
        ValueData::Vector(items) => {
            let lanes: Vec<(Ty, i64)> = items
                .iter()
                .map(|v| plain_int(&Plain::of(v)?))
                .collect::<Option<_>>()?;
            let ty = lanes.first()?.0;
            lanes.iter().all(|(t, _)| *t == ty).then_some(())?;
            Some((ty, Cow::Owned(lanes.into_iter().map(|(_, n)| n).collect())))
        }
        _ => None,
    }
}

fn plain(ty: Ty, n: i64) -> Plain {
    match ty {
        Ty::Int => Plain::Num(Fraction::from_repr(FractionRepr::Small(n, 1))),
        Ty::Bool => Plain::Bool(n != 0),
    }
}

/// `Value::from_vector_promoted` of a run of values of one type: for a
/// non-empty run of integers, `from_fractions` of `n/1` lanes is exactly
/// these columns.
fn promote_lanes(ty: Ty, lanes: Vec<i64>) -> Value {
    if ty == Ty::Int && !lanes.is_empty() {
        let shape = vec![lanes.len()];
        let ones = vec![1; lanes.len()];
        let tensor = DenseTensor::from_columns(lanes, ones, shape.clone(), true, BTreeMap::new());
        return Value::new(
            ValueData::Tensor {
                data: Arc::new(tensor),
                shape: Arc::new(shape),
            },
            None,
        );
    }
    promote(lanes.into_iter().map(|n| plain(ty, n)).collect())
}

fn run_block(ops: &[IntOp], stack: &mut Vec<i64>, slots: &mut [i64]) -> Option<i64> {
    for op in ops {
        let value = match *op {
            IntOp::Push(n) => n,
            IntOp::Load(slot) => slots[slot],
            IntOp::Store(slot) => {
                slots[slot] = stack.pop()?;
                continue;
            }
            IntOp::Not => i64::from(stack.pop()? == 0),
            IntOp::Select => {
                let mask = stack.pop()?;
                let when_false = stack.pop()?;
                let when_true = stack.pop()?;
                if mask != 0 {
                    when_true
                } else {
                    when_false
                }
            }
            _ => {
                let b = stack.pop()?;
                let a = stack.pop()?;
                match *op {
                    IntOp::Add => a.checked_add(b)?,
                    IntOp::Sub => a.checked_sub(b)?,
                    IntOp::Mul => a.checked_mul(b)?,
                    IntOp::FloorDiv => floor_div(a, b)?,
                    IntOp::Lt => i64::from(a < b),
                    IntOp::Gt => i64::from(a > b),
                    IntOp::Eq => i64::from(a == b),
                    _ => i64::from(a != 0 && b != 0),
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
    if interp.runtime_limits.max_bigint_bits < 64 {
        return None;
    }
    let (elem_ty, elements) = elements(target)?;
    let seed = match seed {
        Some(p) => Some(plain_int(p)?),
        None => None,
    };
    let inputs: Vec<Ty> = seed.iter().map(|(t, _)| *t).chain([elem_ty]).collect();
    let typed = typed(block, &inputs)?;
    match walk {
        FusedWalk::Filter if typed.out != Ty::Bool => return None,
        FusedWalk::Fold | FusedWalk::Scan if Some(typed.out) != seed.map(|(t, _)| t) => {
            return None
        }
        _ => {}
    }

    let runs = elements.len() as u64;
    let steps = block.steps_within_ceiling(interp, runs)?;
    let work = runs.checked_mul(typed.work_per_run)?;
    if interp.numeric_work_used.checked_add(work)? > interp.runtime_limits.max_numeric_work {
        return None;
    }

    let mut stack: Vec<i64> = Vec::with_capacity(typed.ops.len() + 2);
    let mut slots = vec![0i64; block.slots];
    let mut accumulator = seed.map(|(_, n)| n);
    let mut results: Vec<i64> = Vec::with_capacity(match walk {
        FusedWalk::Fold => 0,
        _ => elements.len(),
    });
    for &x in elements.iter() {
        stack.clear();
        if let Some(acc) = accumulator {
            stack.push(acc);
        }
        stack.push(x);
        let result = run_block(&typed.ops, &mut stack, &mut slots)?;
        match walk {
            FusedWalk::Map => results.push(result),
            FusedWalk::Filter => {
                if result != 0 {
                    results.push(x);
                }
            }
            FusedWalk::Scan => {
                results.push(result);
                accumulator = Some(result);
            }
            FusedWalk::Fold => accumulator = Some(result),
        }
    }

    let value = match walk {
        FusedWalk::Map => promote_lanes(typed.out, results),
        FusedWalk::Filter => promote_lanes(elem_ty, results),
        FusedWalk::Scan => Value::from_vector(
            results
                .into_iter()
                .map(|n| plain(typed.out, n).into_value())
                .collect(),
        ),
        FusedWalk::Fold => plain(typed.out, accumulator?).into_value(),
    };
    let charges = Charges {
        runs,
        steps,
        work,
        fastpath: runs.checked_mul(typed.fastpath_per_run)?,
    };
    Some((value, charges))
}
