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
//! still charged as the two Words it is. A zero divisor's quotient is one of
//! the three points over zero, which this tier does not hold: the walk goes
//! on to the next tier, which answers it.

use crate::interpreter::arithmetic::ExactArithmeticSchema;
use crate::interpreter::fused_block::{Charges, Compare, FusedBlock, FusedWalk, Op, Plain};
use crate::interpreter::fused_block_general::promote;
use crate::interpreter::fused_block_reg::RegProgram;
use crate::interpreter::runtime_limits::binary_numeric_work;
use crate::interpreter::Interpreter;
use crate::types::fraction::{Fraction, FractionRepr};
use crate::types::{DenseTensor, Value, ValueData};
use std::borrow::Cow;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ty {
    Int,
    Bool,
    /// The quotient of a `DIV` not followed by `FLOOR`: an integer where the
    /// division is exact and otherwise [`POISON`], a fraction this tier
    /// cannot hold. It may only be bound, read back, or be a candidate of
    /// `SELECT` — so a fraction the block computes and then does not choose
    /// (the Collatz step's `N 2 DIV` for odd `N`) never sends the walk to the
    /// small-rational tier. A poisoned lane that reaches a `MAP`'s result
    /// does, from the start.
    Exact,
}

/// The lane value of an inexact quotient (`Ty::Exact`). An exact quotient
/// that happens to be `i64::MIN` reads as one too, which costs only a walk
/// sent on to the next tier.
pub(crate) const POISON: i64 = i64::MIN;

/// The block in stack form, typed; `fused_block_reg` compiles it to
/// register code.
#[derive(Debug, Clone, Copy)]
pub(crate) enum IntOp {
    Push(i64),
    Load(usize),
    Store(usize),
    Add,
    Sub,
    Mul,
    FloorDiv,
    /// `a / b` when `b` divides `a`, else [`POISON`].
    ExactDiv,
    Min,
    Max,
    Lt,
    Gt,
    Eq,
    Not,
    And,
    Select,
    /// `POW` on two integers, as `quickened::small_power` answers it.
    Pow,
}

/// The block checked against `inputs`, with what one run charges.
#[derive(Debug)]
struct Typed {
    ops: Vec<IntOp>,
    out: Ty,
    fastpath_per_run: u64,
    work_per_run: u64,
}

/// The block typed and lowered to register code, per input types, kept on
/// the block (`FusedBlock::int_programs`) so a block walked again and again —
/// the inner `FOLD` of `[ [ 0 ] [ ADD ] FOLD ] MAP` — is compiled once. Both
/// depend on the block and the input types alone; what a walk is charged is
/// checked against the ceilings on every walk.
#[derive(Debug, Default)]
pub(crate) struct IntPrograms(std::sync::Mutex<Vec<KeptProgram>>);

/// One input-type combination and what compiling the block for it gave.
type KeptProgram = (Vec<Ty>, Option<Arc<Compiled>>);

#[derive(Debug)]
struct Compiled {
    typed: Typed,
    program: RegProgram,
}

/// Input-type combinations kept per block; a block meets one or two.
const KEPT_PROGRAMS: usize = 4;

impl Clone for IntPrograms {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl IntPrograms {
    fn get(&self, block: &FusedBlock, inputs: &[Ty]) -> Option<Arc<Compiled>> {
        let mut kept = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((_, compiled)) = kept.iter().find(|(types, _)| types == inputs) {
            return compiled.clone();
        }
        let compiled = typed(block, inputs).and_then(|typed| {
            let program = RegProgram::lower(&typed.ops, inputs.len(), block.slots)?;
            Some(Arc::new(Compiled { typed, program }))
        });
        if kept.len() < KEPT_PROGRAMS {
            kept.push((inputs.to_vec(), compiled.clone()));
        }
        compiled
    }
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
            Op::Push(p) | Op::Const(p) => {
                let (ty, n) = plain_int(p)?;
                (IntOp::Push(n), ty)
            }
            Op::PushWord(b) => (IntOp::Push(i64::from(*b)), Ty::Bool),
            // A Word's plain law runs in the general tier only.
            Op::Kernel(_) => return None,
            Op::Pow => {
                (pair(&mut stack)? == (Ty::Int, Ty::Int)).then_some(())?;
                work += binary_numeric_work(1, 1);
                (IntOp::Pow, Ty::Int)
            }
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
                (mask == Ty::Bool).then_some(())?;
                let ty = match (when_true, when_false) {
                    (a, b) if a == b => a,
                    (Ty::Int, Ty::Exact) | (Ty::Exact, Ty::Int) => Ty::Exact,
                    _ => return None,
                };
                (IntOp::Select, ty)
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
            Op::Extremum { max } => {
                (pair(&mut stack)? == (Ty::Int, Ty::Int)).then_some(())?;
                (if *max { IntOp::Max } else { IntOp::Min }, Ty::Int)
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
                            ops.push(IntOp::ExactDiv);
                            stack.push(Ty::Exact);
                            i += 1;
                            continue;
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

/// Every element as `(type, i64)`, borrowed in place from a pure-integer
/// dense Tensor's numerator column, or `None`. A dense Tensor holds numbers
/// only (`dense_columns`), and `is_pure_integer` is false when any lane is a
/// fraction or a point over zero, so a column it vouches for is integers.
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
        Ty::Int | Ty::Exact => Plain::Num(Fraction::from_repr(FractionRepr::Small(n, 1))),
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
        let tensor = DenseTensor::from_columns(lanes, ones, shape.clone(), true);
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
    let inputs: smallvec::SmallVec<[Ty; 2]> =
        seed.iter().map(|(t, _)| *t).chain([elem_ty]).collect();
    let compiled = block.int_programs.get(block, &inputs)?;
    let (typed, program) = (&compiled.typed, &compiled.program);
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

    let value = match (walk, seed) {
        (FusedWalk::Map, _) => {
            let mut results = Vec::with_capacity(elements.len());
            program.run_columns(&elements, |_, out| results.extend_from_slice(out))?;
            match typed.out {
                // A fraction was chosen for some lane: the next tier's walk.
                Ty::Exact if results.contains(&POISON) => return None,
                Ty::Exact => promote_lanes(Ty::Int, results),
                ty => promote_lanes(ty, results),
            }
        }
        (FusedWalk::Filter, _) => {
            let mut kept = Vec::new();
            program.run_columns(&elements, |xs, keep| {
                kept.extend(
                    xs.iter()
                        .zip(keep)
                        .filter(|(_, k)| **k != 0)
                        .map(|(x, _)| *x),
                );
            })?;
            promote_lanes(elem_ty, kept)
        }
        (FusedWalk::Fold | FusedWalk::Scan, Some((_, mut acc))) => {
            let mut visited = Vec::with_capacity(match walk {
                FusedWalk::Scan => elements.len(),
                _ => 0,
            });
            let scan = walk == FusedWalk::Scan;
            acc = program.run_accumulating(&elements, acc, |a| {
                if scan {
                    visited.push(plain(typed.out, a).into_value());
                }
            })?;
            match walk {
                FusedWalk::Scan => Value::from_vector(visited),
                _ => plain(typed.out, acc).into_value(),
            }
        }
        (FusedWalk::Fold | FusedWalk::Scan, None) => return None,
    };
    let charges = Charges {
        runs,
        steps,
        work,
        fastpath: runs.checked_mul(typed.fastpath_per_run)?,
    };
    Some((value, charges))
}
