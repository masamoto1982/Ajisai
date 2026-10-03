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

use crate::interpreter::fused_block_int::IntOp;

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

/// One instruction on scalars: the value, and whether it left the tier.
#[inline(always)]
fn apply(kind: Kind, a: i64, b: i64, c: i64) -> (i64, bool) {
    match kind {
        Kind::Add => a.overflowing_add(b),
        Kind::Sub => a.overflowing_sub(b),
        Kind::Mul => a.overflowing_mul(b),
        Kind::FloorDiv => floor_div(a, b),
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
        Kind::Mul => zip2(dst, a, b, i64::overflowing_mul),
        Kind::FloorDiv => zip2(dst, a, b, floor_div),
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

/// `acc = op(acc, x)` for each lane, in a loop with nothing else in it.
#[inline(always)]
fn fold_with(
    acc: &mut i64,
    lanes: &[i64],
    each: &mut impl FnMut(i64),
    op: impl Fn(i64, i64) -> (i64, bool),
) -> bool {
    for &x in lanes {
        let (v, bad) = op(*acc, x);
        if bad {
            return false;
        }
        *acc = v;
        each(v);
    }
    true
}

/// The same against a constant operand, which is rare enough to dispatch
/// per lane.
fn fold_lanes(
    acc: &mut i64,
    lanes: impl Iterator<Item = i64>,
    kind: Kind,
    flipped: bool,
    each: &mut impl FnMut(i64),
) -> bool {
    for x in lanes {
        let (a, b) = if flipped { (x, *acc) } else { (*acc, x) };
        let (v, bad) = apply(kind, a, b, 0);
        if bad {
            return false;
        }
        *acc = v;
        each(v);
    }
    true
}

/// An operand of an instruction that reads the accumulator, resolved for
/// `run_accumulating`: a per-run scalar register, a lane of a column the
/// accumulator-free instructions filled, or a constant.
#[derive(Debug, Clone, Copy)]
enum Lane {
    Scalar(usize),
    Column(usize),
    Const(i64),
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
        Some(Self {
            instrs,
            regs,
            inputs,
            out,
        })
    }

    /// Run a two-input block — accumulator in register 0, element in
    /// register 1 — over every element from `seed`, handing `each` every
    /// accumulator. Answers the last one, or `None` when a value leaves the
    /// tier.
    ///
    /// Each run needs the last one's accumulator, but usually only a little
    /// of the block does: in `[ 3 MUL ADD ]` the `3 MUL` reads the element
    /// alone. The instructions that never read the accumulator, however
    /// indirectly, run a column at a time as `run_columns` runs a whole
    /// block; only the rest runs per element. When that rest is one
    /// arithmetic instruction on the accumulator — `[ ADD ]`, `[ 3 MUL ADD ]`
    /// — it gets a loop of its own with nothing in it but the operation.
    pub(crate) fn run_accumulating(
        &self,
        elements: &[i64],
        seed: i64,
        mut each: impl FnMut(i64),
    ) -> Option<i64> {
        debug_assert_eq!(self.inputs, 2);
        let mut reads_acc = vec![false; self.regs];
        reads_acc[0] = true;
        let reads = |reads_acc: &[bool], s: Src| matches!(s, Src::Reg(r) if reads_acc[r]);
        for ins in &self.instrs {
            reads_acc[ins.dst] =
                reads(&reads_acc, ins.a) || reads(&reads_acc, ins.b) || reads(&reads_acc, ins.c);
        }
        let (free, per_run): (Vec<Instr>, Vec<Instr>) =
            self.instrs.iter().partition(|ins| !reads_acc[ins.dst]);
        let lane = |s: Src| match s {
            Src::Reg(r) if reads_acc[r] => Lane::Scalar(r),
            Src::Reg(r) => Lane::Column(r),
            Src::Const(v) => Lane::Const(v),
        };
        let per_run: Vec<(Kind, usize, [Lane; 3])> = per_run
            .iter()
            .map(|ins| (ins.kind, ins.dst, [lane(ins.a), lane(ins.b), lane(ins.c)]))
            .collect();
        let out = lane(self.out);
        // `acc = acc op x` and `acc = x op acc`, when that is all that reads it.
        let single = match (per_run.as_slice(), out) {
            ([(kind @ (Kind::Add | Kind::Sub | Kind::Mul), dst, [a, b, _])], Lane::Scalar(o))
                if o == *dst =>
            {
                match (a, b) {
                    (Lane::Scalar(0), x @ (Lane::Column(_) | Lane::Const(_))) => {
                        Some((*kind, false, *x))
                    }
                    (x @ (Lane::Column(_) | Lane::Const(_)), Lane::Scalar(0)) => {
                        Some((*kind, true, *x))
                    }
                    _ => None,
                }
            }
            _ => None,
        };

        let per_run_reads_element = single.is_none();
        // A row is a chunk wide, or as wide as the walk when that is shorter:
        // a walk of three elements need not clear a 256-lane row per register.
        let width = elements.len().clamp(1, CHUNK);
        let mut regs = vec![0i64; self.regs * width];
        let mut scalars = vec![0i64; self.regs];
        let mut acc = seed;
        for chunk in elements.chunks(width) {
            let len = chunk.len();
            if !free.is_empty() || per_run_reads_element {
                regs[width..width + len].copy_from_slice(chunk);
            }
            for ins in &free {
                if exec_column(ins, &mut regs, width, len) {
                    return None;
                }
            }
            let at = |regs: &[i64], scalars: &[i64], l: Lane, i: usize| match l {
                Lane::Scalar(r) => scalars[r],
                Lane::Column(r) => regs[r * width + i],
                Lane::Const(v) => v,
            };
            if let Some((kind, flipped, x)) = single {
                let column = |r: usize| {
                    if r == 1 {
                        chunk
                    } else {
                        &regs[r * width..r * width + len]
                    }
                };
                let lanes: &[i64] = match x {
                    Lane::Column(r) => column(r),
                    _ => &[],
                };
                let ok = match (x, kind, flipped) {
                    (Lane::Const(v), _, _) => fold_lanes(
                        &mut acc,
                        std::iter::repeat_n(v, len),
                        kind,
                        flipped,
                        &mut each,
                    ),
                    (_, Kind::Add, _) => {
                        fold_with(&mut acc, lanes, &mut each, i64::overflowing_add)
                    }
                    (_, Kind::Mul, _) => {
                        fold_with(&mut acc, lanes, &mut each, i64::overflowing_mul)
                    }
                    (_, _, false) => fold_with(&mut acc, lanes, &mut each, i64::overflowing_sub),
                    (_, _, true) => {
                        fold_with(&mut acc, lanes, &mut each, |a, x| x.overflowing_sub(a))
                    }
                };
                if !ok {
                    return None;
                }
                continue;
            }
            for i in 0..len {
                scalars[0] = acc;
                for (kind, dst, [a, b, c]) in &per_run {
                    let (v, bad) = apply(
                        *kind,
                        at(&regs, &scalars, *a, i),
                        at(&regs, &scalars, *b, i),
                        at(&regs, &scalars, *c, i),
                    );
                    if bad {
                        return None;
                    }
                    scalars[*dst] = v;
                }
                acc = at(&regs, &scalars, out, i);
                each(acc);
            }
        }
        Some(acc)
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
