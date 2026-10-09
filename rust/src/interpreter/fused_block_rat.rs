//! The small-rational tier of a fused walk (`fused_block`): every number a
//! rational whose halves each fit a machine word, every other value a Boolean.
//!
//! The integer tier (`fused_block_int`) declines a block as soon as a value
//! can be a fraction — a `DIV` that is not a floor division, a fractional
//! literal or element — and the general tier then holds every value as a
//! `Fraction`, an enum with a heap-backed `BigInt` variant, dispatched op by
//! op. Most fractions a program meets are small, though, and a rational
//! whose numerator and denominator are each an `i64` is the `Small` form the
//! interpreted route stores them in too. This tier keeps them as `(i64, i64)`
//! pairs in registers and computes in `i128`, reducing each result with the
//! binary gcd, and steps aside the moment a result no longer fits a pair.
//!
//! Like the integer tier, it types the straight-line block against the input
//! types before the walk, so its charges are known up front: every number
//! here is `Small`, which the meter prices at one limb, so each arithmetic
//! Word costs `binary_numeric_work(1, 1)` and takes the scalar fast path, as
//! `LT`/`GT` and a numeric `EQ` do, and every result fits 64 bits for the
//! size ceiling. A zero divisor is the ordinary walk's NIL to project.
//!
//! The results are the ones `Fraction` reaches: a rational's lowest-terms
//! form with a positive denominator is unique, so any correct reduction
//! agrees with it, which `fused_block_tests` holds the tier to.

use crate::interpreter::arithmetic::ExactArithmeticSchema;
use crate::interpreter::fused_block::{Charges, Compare, FusedBlock, FusedWalk, Op, Plain};
use crate::interpreter::fused_block_general::promote;
use crate::interpreter::runtime_limits::binary_numeric_work;
use crate::interpreter::Interpreter;
use crate::types::fraction::{Fraction, FractionRepr};
use crate::types::small_rational::{self, add, div, mul, order};
use crate::types::{DenseTensor, Value, ValueData};
use std::cmp::Ordering;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ty {
    Num,
    Bool,
}

/// A number as a `small_rational` pair; a Boolean is `(0 or 1, 1)`.
type Pair = crate::types::small_rational::Pair;

#[derive(Debug, Clone, Copy)]
enum Src {
    Reg(usize),
    Const(Pair),
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    Add,
    Sub,
    Mul,
    Div,
    Min,
    Max,
    Floor,
    Round,
    Lt,
    Gt,
    Eq,
    Not,
    And,
    Select,
    Pow,
}

/// `dst = kind(a, b, c)`, each value written to a register of its own.
#[derive(Debug, Clone, Copy)]
struct Instr {
    kind: Kind,
    dst: usize,
    a: Src,
    b: Src,
    c: Src,
}

/// The block typed against its inputs and lowered to register code, with
/// what one run charges.
struct Program {
    instrs: Vec<Instr>,
    regs: usize,
    out: Src,
    out_ty: Ty,
    fastpath_per_run: u64,
    work_per_run: u64,
}

fn plain_pair(p: &Plain) -> Option<(Ty, Pair)> {
    match p {
        Plain::Num(f) => Some((Ty::Num, f.extract_i64_pair()?)),
        Plain::Bool(b) => Some((Ty::Bool, (i64::from(*b), 1))),
    }
}

fn compile(block: &FusedBlock, inputs: &[Ty]) -> Option<Program> {
    let mut stack: Vec<(Src, Ty)> = (0..inputs.len())
        .map(|i| (Src::Reg(i), inputs[i]))
        .collect();
    let mut slots: Vec<Option<(Src, Ty)>> = vec![None; block.slots];
    let mut instrs: Vec<Instr> = Vec::with_capacity(block.ops.len());
    let (mut fastpath, mut work) = (0u64, 0u64);
    let none = Src::Const((0, 1));
    for op in &block.ops {
        let (kind, a, b, c, ty) = match op {
            Op::Push(p) | Op::Const(p) => {
                let (ty, pair) = plain_pair(p)?;
                stack.push((Src::Const(pair), ty));
                continue;
            }
            // A Word's plain law runs in the general tier only.
            Op::Kernel(_) => return None,
            Op::Pow => {
                let (b, b_ty) = stack.pop()?;
                let (a, a_ty) = stack.pop()?;
                (a_ty == Ty::Num && b_ty == Ty::Num).then_some(())?;
                work += binary_numeric_work(1, 1);
                (Kind::Pow, a, b, none, Ty::Num)
            }
            Op::PushWord(b) => {
                stack.push((Src::Const((i64::from(*b), 1)), Ty::Bool));
                continue;
            }
            Op::Load(slot) => {
                stack.push(slots[*slot]?);
                continue;
            }
            Op::Bind(slot) => {
                slots[*slot] = Some(stack.pop()?);
                continue;
            }
            Op::Floor | Op::Round | Op::Not => {
                let (a, ty) = stack.pop()?;
                let (kind, want) = match op {
                    Op::Floor => (Kind::Floor, Ty::Num),
                    Op::Round => (Kind::Round, Ty::Num),
                    _ => (Kind::Not, Ty::Bool),
                };
                (ty == want).then_some(())?;
                (kind, a, none, none, want)
            }
            Op::Select => {
                let (mask, mask_ty) = stack.pop()?;
                let (when_false, f_ty) = stack.pop()?;
                let (when_true, t_ty) = stack.pop()?;
                (mask_ty == Ty::Bool && t_ty == f_ty).then_some(())?;
                (Kind::Select, when_true, when_false, mask, t_ty)
            }
            Op::Extremum { max } => {
                let (b, b_ty) = stack.pop()?;
                let (a, a_ty) = stack.pop()?;
                (a_ty == Ty::Num && b_ty == Ty::Num).then_some(())?;
                let kind = if *max { Kind::Max } else { Kind::Min };
                (kind, a, b, none, Ty::Num)
            }
            Op::And | Op::Compare(_) | Op::Arith(_) => {
                let (b, b_ty) = stack.pop()?;
                let (a, a_ty) = stack.pop()?;
                let kind = match (op, a_ty, b_ty) {
                    (Op::And, Ty::Bool, Ty::Bool) => Kind::And,
                    // Two Booleans compare by `pairwise_eq`, off the fast path.
                    (Op::Compare(Compare::Eq), Ty::Bool, Ty::Bool) => Kind::Eq,
                    (Op::Compare(kind), Ty::Num, Ty::Num) => {
                        fastpath += 1;
                        match kind {
                            Compare::Lt => Kind::Lt,
                            Compare::Gt => Kind::Gt,
                            Compare::Eq => Kind::Eq,
                        }
                    }
                    (Op::Arith(schema), Ty::Num, Ty::Num) => {
                        fastpath += 1;
                        work += binary_numeric_work(1, 1);
                        match schema {
                            ExactArithmeticSchema::Add => Kind::Add,
                            ExactArithmeticSchema::Sub => Kind::Sub,
                            ExactArithmeticSchema::Mul => Kind::Mul,
                            ExactArithmeticSchema::Div => Kind::Div,
                        }
                    }
                    _ => return None,
                };
                let ty = match kind {
                    Kind::Add | Kind::Sub | Kind::Mul | Kind::Div => Ty::Num,
                    _ => Ty::Bool,
                };
                (kind, a, b, none, ty)
            }
        };
        let dst = inputs.len() + instrs.len();
        instrs.push(Instr { kind, dst, a, b, c });
        stack.push((Src::Reg(dst), ty));
    }
    let (out, out_ty) = stack.pop()?;
    Some(Program {
        regs: inputs.len() + instrs.len(),
        instrs,
        out,
        out_ty,
        fastpath_per_run: fastpath,
        work_per_run: work,
    })
}

impl Program {
    /// One run on `regs`, whose first registers hold the inputs. `None`
    /// when a result leaves the tier.
    fn run(&self, regs: &mut [Pair]) -> Option<Pair> {
        let read = |regs: &[Pair], s: Src| match s {
            Src::Reg(r) => regs[r],
            Src::Const(p) => p,
        };
        let truth = |b: bool| (i64::from(b), 1);
        for ins in &self.instrs {
            let (a, b) = (read(regs, ins.a), read(regs, ins.b));
            regs[ins.dst] = match ins.kind {
                Kind::Add => add(a, b, false)?,
                Kind::Sub => add(a, b, true)?,
                Kind::Mul => mul(a, b)?,
                Kind::Div => div(a, b)?,
                Kind::Pow => crate::interpreter::quickened::small_power(a, b)?,
                // The left operand on a tie, as MIN and MAX keep it.
                Kind::Min if order(b, a) == Ordering::Less => b,
                Kind::Max if order(a, b) == Ordering::Less => b,
                Kind::Min | Kind::Max => a,
                Kind::Floor => (a.0.div_euclid(a.1), 1),
                Kind::Round => (small_rational::round_half_away_from_zero(a.0, a.1), 1),
                Kind::Lt => truth(order(a, b) == Ordering::Less),
                Kind::Gt => truth(order(a, b) == Ordering::Greater),
                Kind::Eq => truth(a == b),
                Kind::Not => truth(a.0 == 0),
                Kind::And => truth(a.0 != 0 && b.0 != 0),
                Kind::Select => {
                    if read(regs, ins.c).0 != 0 {
                        a
                    } else {
                        b
                    }
                }
            };
        }
        Some(read(regs, self.out))
    }
}

/// Every element as a pair of one type: a dense Tensor's columns, borrowed
/// in place, or the pairs of a Vector of plain values. `None` for an absent
/// lane, a lane wider than a pair, or mixed types.
enum Elements<'a> {
    Columns(&'a [i64], &'a [i64]),
    Pairs(Vec<Pair>),
}

impl Elements<'_> {
    fn len(&self) -> usize {
        match self {
            Elements::Columns(nums, _) => nums.len(),
            Elements::Pairs(pairs) => pairs.len(),
        }
    }

    #[inline]
    fn at(&self, i: usize) -> Pair {
        match self {
            Elements::Columns(nums, dens) => (nums[i], dens[i]),
            Elements::Pairs(pairs) => pairs[i],
        }
    }
}

fn elements(target: &Value) -> Option<(Ty, Elements<'_>)> {
    match &target.data {
        ValueData::Tensor { data, shape } if shape.len() == 1 && data.all_finite() => Some((
            Ty::Num,
            Elements::Columns(&data.numerators, &data.denominators),
        )),
        ValueData::Vector(items) => {
            let lanes: Vec<(Ty, Pair)> = items
                .iter()
                .map(|v| plain_pair(&Plain::of(v)?))
                .collect::<Option<_>>()?;
            let ty = lanes.first()?.0;
            lanes.iter().all(|(t, _)| *t == ty).then_some(())?;
            Some((
                ty,
                Elements::Pairs(lanes.into_iter().map(|(_, p)| p).collect()),
            ))
        }
        _ => None,
    }
}

/// Results gathered as the two columns a dense Tensor stores.
#[derive(Default)]
struct Columns {
    nums: Vec<i64>,
    dens: Vec<i64>,
}

impl Columns {
    fn with_capacity(n: usize) -> Self {
        Self {
            nums: Vec::with_capacity(n),
            dens: Vec::with_capacity(n),
        }
    }

    #[inline]
    fn push(&mut self, (n, d): Pair) {
        self.nums.push(n);
        self.dens.push(d);
    }
}

fn plain(ty: Ty, (n, d): Pair) -> Plain {
    match ty {
        Ty::Num => Plain::Num(Fraction::from_repr(FractionRepr::Small(n, d))),
        Ty::Bool => Plain::Bool(n != 0),
    }
}

/// `Value::from_vector_promoted` of a run of one type: for a non-empty run
/// of numbers, `from_fractions` of these pairs is exactly these columns.
fn promote_columns(ty: Ty, columns: Columns) -> Value {
    let Columns { nums, dens } = columns;
    if ty == Ty::Bool || nums.is_empty() {
        return promote(nums.into_iter().zip(dens).map(|p| plain(ty, p)).collect());
    }
    let shape = vec![nums.len()];
    let integer = dens.iter().all(|d| *d == 1);
    let tensor = DenseTensor::from_columns(nums, dens, shape.clone(), integer);
    Value::new(
        ValueData::Tensor {
            data: Arc::new(tensor),
            shape: Arc::new(shape),
        },
        None,
    )
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
        Some(p) => Some(plain_pair(p)?),
        None => None,
    };
    let inputs: Vec<Ty> = seed.iter().map(|(t, _)| *t).chain([elem_ty]).collect();
    let program = compile(block, &inputs)?;
    match walk {
        FusedWalk::Filter if program.out_ty != Ty::Bool => return None,
        FusedWalk::Fold | FusedWalk::Scan if Some(program.out_ty) != seed.map(|(t, _)| t) => {
            return None
        }
        _ => {}
    }

    let runs = elements.len() as u64;
    let steps = block.steps_within_ceiling(interp, runs)?;
    let work = runs.checked_mul(program.work_per_run)?;
    if interp.numeric_work_used.checked_add(work)? > interp.runtime_limits.max_numeric_work {
        return None;
    }

    let mut regs = vec![(0i64, 1i64); program.regs];
    let mut accumulator = seed.map(|(_, p)| p);
    let mut results = Columns::with_capacity(match walk {
        FusedWalk::Fold => 0,
        _ => elements.len(),
    });
    for i in 0..elements.len() {
        let x = elements.at(i);
        match accumulator {
            Some(acc) => {
                regs[0] = acc;
                regs[1] = x;
            }
            None => regs[0] = x,
        }
        let result = program.run(&mut regs)?;
        match walk {
            FusedWalk::Map => results.push(result),
            FusedWalk::Filter => {
                if result.0 != 0 {
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
        FusedWalk::Map => promote_columns(program.out_ty, results),
        FusedWalk::Filter => promote_columns(elem_ty, results),
        FusedWalk::Scan => Value::from_vector(
            results
                .nums
                .into_iter()
                .zip(results.dens)
                .map(|p| plain(program.out_ty, p).into_value())
                .collect(),
        ),
        FusedWalk::Fold => plain(program.out_ty, accumulator?).into_value(),
    };
    let charges = Charges {
        runs,
        steps,
        work,
        fastpath: runs.checked_mul(program.fastpath_per_run)?,
    };
    #[cfg(test)]
    RAT_RUNS.with(|c| c.set(c.get() + 1));
    Some((value, charges))
}

#[cfg(test)]
thread_local! {
    static RAT_RUNS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Walks this tier answered on this thread, for tests that pin which tier ran.
#[cfg(test)]
pub(crate) fn rat_runs_on_this_thread() -> u64 {
    RAT_RUNS.with(|c| c.get())
}
