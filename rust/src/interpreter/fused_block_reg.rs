//! The integer tier's block as register code, and the two loops that run it.
//!
//! `fused_block_int` checks a block against its input types and hands over
//! its ops still in stack form. A straight-line block's stack depth at every
//! op is known before it runs, so the stack itself can be compiled away: each
//! value it would have held becomes a register written once (SSA), a literal
//! becomes a constant operand, and a `BIND` or a bound name becomes nothing
//! at all — the name simply refers to whichever operand was bound. What is
//! left is a short list of three-address instructions with no push, no pop
//! and no bounds to check.
//!
//! `run_columns` runs that list for `MAP` and `FILTER`, whose runs are
//! independent of one another: a column at a time, each instruction applied
//! to [`CHUNK`] elements before the next, so its dispatch is paid once per
//! chunk and each inner loop is a plain array loop the compiler can vectorise.
//! Arithmetic is wrapping with an overflow flag accumulated across the chunk;
//! a set flag (or a zero divisor) abandons the tier, as a checked operation
//! would have, so no wrapped value is ever kept. `run_scalar` runs it once per
//! element for `FOLD` and `SCAN`, where each run needs the last one's result.

use crate::interpreter::fused_block_int::{IntOp, POISON};
use crate::types::small_rational::overflowing_mul;

mod accumulating;

/// How many elements `run_columns` takes per instruction.
const CHUNK: usize = 256;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Src {
    Reg(usize),
    Const(i64),
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    Add,
    Sub,
    Mul,
    FloorDiv,
    ExactDiv,
    Min,
    Max,
    Lt,
    Gt,
    Eq,
    Not,
    And,
    Select,
}

/// `dst = kind(a, b, c)`. `dst` is always a register numbered above every
/// register its operands read, which is what lets `run_columns` borrow the
/// operands and the destination out of one buffer at once.
#[derive(Debug, Clone, Copy)]
struct Instr {
    kind: Kind,
    dst: usize,
    a: Src,
    b: Src,
    c: Src,
}

#[derive(Debug, Clone)]
pub(crate) struct RegProgram {
    instrs: Vec<Instr>,
    regs: usize,
    inputs: usize,
    out: Src,
    /// For a two-input block, how `run_accumulating` splits it: worked out
    /// once here rather than on every walk.
    acc: Option<accumulating::AccPlan>,
}

/// `floor(a / b)`, with `true` for a zero divisor or `i64::MIN / -1`.
#[inline(always)]
fn floor_div(a: i64, b: i64) -> (i64, bool) {
    match a.checked_div(b) {
        Some(q) if a % b != 0 && ((a < 0) != (b < 0)) => (q - 1, false),
        Some(q) => (q, false),
        None => (0, true),
    }
}

/// `a / b` when `b` divides `a`, [`POISON`] when it does not, and `true`
/// for a zero divisor or `i64::MIN / -1`. An exact quotient of `i64::MIN`
/// reads as poison too; that only ever sends a walk on to the next tier,
/// which answers it exactly.
#[inline(always)]
fn exact_div(a: i64, b: i64) -> (i64, bool) {
    match a.checked_rem(b) {
        Some(0) => (a / b, false),
        Some(_) => (POISON, false),
        None => (0, true),
    }
}

/// One instruction on scalars: the value, and whether it left the tier.
#[inline(always)]
fn apply(kind: Kind, a: i64, b: i64, c: i64) -> (i64, bool) {
    match kind {
        Kind::Add => a.overflowing_add(b),
        Kind::Sub => a.overflowing_sub(b),
        Kind::Mul => overflowing_mul(a, b),
        Kind::FloorDiv => floor_div(a, b),
        Kind::ExactDiv => exact_div(a, b),
        Kind::Min => (a.min(b), false),
        Kind::Max => (a.max(b), false),
        Kind::Lt => (i64::from(a < b), false),
        Kind::Gt => (i64::from(a > b), false),
        Kind::Eq => (i64::from(a == b), false),
        Kind::Not => (i64::from(a == 0), false),
        Kind::And => (i64::from(a != 0 && b != 0), false),
        Kind::Select => (if c != 0 { a } else { b }, false),
    }
}

/// An operand over one chunk: a register's lanes, or one value for all.
#[derive(Clone, Copy)]
enum Col<'a> {
    Lanes(&'a [i64]),
    Splat(i64),
}

impl Col<'_> {
    #[inline(always)]
    fn at(self, i: usize) -> i64 {
        match self {
            Col::Lanes(lanes) => lanes[i],
            Col::Splat(v) => v,
        }
    }
}

/// `dst[i] = f(a[i], b[i])` over a chunk, answering whether any lane left the
/// tier. Each operand shape gets its own loop, so the loop body is the
/// operation alone.
#[inline(always)]
fn zip2(dst: &mut [i64], a: Col, b: Col, f: impl Fn(i64, i64) -> (i64, bool)) -> bool {
    let mut bad = false;
    match (a, b) {
        (Col::Lanes(a), Col::Lanes(b)) => {
            for ((d, &x), &y) in dst.iter_mut().zip(a).zip(b) {
                let (v, o) = f(x, y);
                *d = v;
                bad |= o;
            }
        }
        (Col::Lanes(a), Col::Splat(y)) => {
            for (d, &x) in dst.iter_mut().zip(a) {
                let (v, o) = f(x, y);
                *d = v;
                bad |= o;
            }
        }
        (Col::Splat(x), Col::Lanes(b)) => {
            for (d, &y) in dst.iter_mut().zip(b) {
                let (v, o) = f(x, y);
                *d = v;
                bad |= o;
            }
        }
        (Col::Splat(x), Col::Splat(y)) => {
            let (v, o) = f(x, y);
            dst.fill(v);
            bad = o;
        }
    }
    bad
}

/// One instruction over a chunk of `len` lanes held in `regs`, a register
/// per `width`-wide row. Answers whether any lane left the tier.
fn exec_column(ins: &Instr, regs: &mut [i64], width: usize, len: usize) -> bool {
    let (lo, hi) = regs.split_at_mut(ins.dst * width);
    let dst = &mut hi[..len];
    let col = |s: Src| match s {
        Src::Reg(r) => Col::Lanes(&lo[r * width..r * width + len]),
        Src::Const(v) => Col::Splat(v),
    };
    let (a, b, c) = (col(ins.a), col(ins.b), col(ins.c));
    match ins.kind {
        Kind::Add => zip2(dst, a, b, i64::overflowing_add),
        Kind::Sub => zip2(dst, a, b, i64::overflowing_sub),
        Kind::Mul => zip2(dst, a, b, overflowing_mul),
        Kind::FloorDiv => zip2(dst, a, b, floor_div),
        Kind::ExactDiv => zip2(dst, a, b, exact_div),
        Kind::Min => zip2(dst, a, b, |x, y| (x.min(y), false)),
        Kind::Max => zip2(dst, a, b, |x, y| (x.max(y), false)),
        Kind::Lt => zip2(dst, a, b, |x, y| (i64::from(x < y), false)),
        Kind::Gt => zip2(dst, a, b, |x, y| (i64::from(x > y), false)),
        Kind::Eq => zip2(dst, a, b, |x, y| (i64::from(x == y), false)),
        Kind::And => zip2(dst, a, b, |x, y| (i64::from(x != 0 && y != 0), false)),
        Kind::Not => zip2(dst, a, Col::Splat(0), |x, _| (i64::from(x == 0), false)),
        Kind::Select => {
            for (i, d) in dst.iter_mut().enumerate() {
                *d = if c.at(i) != 0 { a.at(i) } else { b.at(i) };
            }
            false
        }
    }
}

impl RegProgram {
    /// Compile stack-form `ops`, which start on `inputs` values and bind
    /// `slots` names, to register code. `None` only for ops that would
    /// underflow, which `fused_block_int::typed` has already ruled out.
    pub(crate) fn lower(ops: &[IntOp], inputs: usize, slots: usize) -> Option<Self> {
        let mut stack: Vec<Src> = (0..inputs).map(Src::Reg).collect();
        let mut bound: Vec<Option<Src>> = vec![None; slots];
        let mut instrs = Vec::new();
        let mut emit = |stack: &mut Vec<Src>, kind, a, b, c| {
            let dst = inputs + instrs.len();
            instrs.push(Instr { kind, dst, a, b, c });
            stack.push(Src::Reg(dst));
        };
        let none = Src::Const(0);
        for op in ops {
            match *op {
                IntOp::Push(n) => stack.push(Src::Const(n)),
                IntOp::Load(slot) => stack.push(bound[slot]?),
                IntOp::Store(slot) => bound[slot] = Some(stack.pop()?),
                IntOp::Not => {
                    let a = stack.pop()?;
                    emit(&mut stack, Kind::Not, a, none, none);
                }
                IntOp::Select => {
                    let mask = stack.pop()?;
                    let when_false = stack.pop()?;
                    let when_true = stack.pop()?;
                    emit(&mut stack, Kind::Select, when_true, when_false, mask);
                }
                IntOp::Add
                | IntOp::Sub
                | IntOp::Mul
                | IntOp::FloorDiv
                | IntOp::ExactDiv
                | IntOp::Min
                | IntOp::Max
                | IntOp::Lt
                | IntOp::Gt
                | IntOp::Eq
                | IntOp::And => {
                    let b = stack.pop()?;
                    let a = stack.pop()?;
                    let kind = match *op {
                        IntOp::Add => Kind::Add,
                        IntOp::Sub => Kind::Sub,
                        IntOp::Mul => Kind::Mul,
                        IntOp::FloorDiv => Kind::FloorDiv,
                        IntOp::ExactDiv => Kind::ExactDiv,
                        IntOp::Min => Kind::Min,
                        IntOp::Max => Kind::Max,
                        IntOp::Lt => Kind::Lt,
                        IntOp::Gt => Kind::Gt,
                        IntOp::Eq => Kind::Eq,
                        _ => Kind::And,
                    };
                    emit(&mut stack, kind, a, b, none);
                }
            }
        }
        let out = stack.pop()?;
        let regs = inputs + instrs.len();
        let mut program = Self {
            instrs,
            regs,
            inputs,
            out,
            acc: None,
        };
        if inputs == 2 {
            program.acc = Some(program.accumulating_plan());
        }
        Some(program)
    }

    /// Run a one-input block over every element, a chunk at a time, handing
    /// `sink` each chunk of elements beside the chunk of results. `None` when
    /// a value leaves the tier.
    pub(crate) fn run_columns(
        &self,
        elements: &[i64],
        mut sink: impl FnMut(&[i64], &[i64]),
    ) -> Option<()> {
        debug_assert_eq!(self.inputs, 1);
        let width = elements.len().clamp(1, CHUNK);
        let mut regs = vec![0i64; self.regs * width];
        let mut splat = vec![0i64; width];
        for chunk in elements.chunks(width) {
            let len = chunk.len();
            regs[..len].copy_from_slice(chunk);
            for ins in &self.instrs {
                if exec_column(ins, &mut regs, width, len) {
                    return None;
                }
            }
            let out: &[i64] = match self.out {
                Src::Reg(r) => &regs[r * width..r * width + len],
                Src::Const(v) => {
                    splat[..len].fill(v);
                    &splat[..len]
                }
            };
            sink(chunk, out);
        }
        Some(())
    }
}
